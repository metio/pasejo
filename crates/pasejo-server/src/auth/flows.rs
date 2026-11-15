// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sign-in flows: the OAuth 2.0 device authorization grant for the CLI,
//! the console's cookie exchange, workload tokens and the emergency window.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use pasejo_protocol::Id;
use pasejo_protocol::api::{
    DeviceAuthorization, DeviceAuthorizationRequest, ErrorCode, TokenRequest, TokenResponse,
    purpose,
};
use pasejo_protocol::encoding::{Fixed, b64, random, sha256, unb64};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use sqlx::Row;

use super::{
    Authed, EMERGENCY_LIFETIME, PERSON_LIFETIME, SessionKind, check_claim, create_session,
};
use crate::audit::{self, Event};
use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult};
use crate::html::{escape, message, page};
use crate::state::{AppState, RequestCtx};
use crate::users;

const GRANT_LIFETIME: u64 = 600_000;
const POLL_INTERVAL: u64 = 5_000;
const FLOW_LIFETIME: u64 = 600_000;
const WEB_COOKIE: &str = "pasejo_web";
const USER_CODE_ALPHABET: &[u8; 20] = b"BCDFGHJKLMNPQRSTVWXZ";

/// Parses a JSON request body into a stable error on failure.
///
/// # Errors
///
/// Fails with `bad_request`.
pub fn parse<T: DeserializeOwned>(body: &[u8]) -> ApiResult<T> {
    Ok(serde_json::from_slice(body)?)
}

/// Counts a request against the authentication rate limit.
///
/// # Errors
///
/// Fails with `rate_limited` above the limit.
pub fn limit(state: &AppState, ctx: &RequestCtx, bucket: &'static str) -> ApiResult<()> {
    if state.limiter.allow(
        bucket,
        ctx.ip,
        state.config.limits.auth_per_minute,
        state.now(),
    ) {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::RateLimited, "too many attempts"))
    }
}

/// The fingerprint shown for a signing key during a device grant: the
/// first 20 bytes of its SHA-256 in five groups of eight hex digits.
#[must_use]
pub fn key_fingerprint(key: &[u8; 32]) -> String {
    let digest = sha256(key);
    let mut text = String::new();
    for (index, byte) in digest[..20].iter().enumerate() {
        if index > 0 && index % 4 == 0 {
            text.push(' ');
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}

fn user_code() -> String {
    let bytes = random::<8>();
    let mut code = String::with_capacity(9);
    for (index, byte) in bytes.iter().enumerate() {
        if index == 4 {
            code.push('-');
        }
        code.push(char::from(
            USER_CODE_ALPHABET[usize::from(*byte) % USER_CODE_ALPHABET.len()],
        ));
    }
    code
}

fn normalize_user_code(text: &str) -> String {
    let letters: String = text
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|character| character.to_ascii_uppercase())
        .collect();
    if letters.len() == 8 {
        format!("{}-{}", &letters[..4], &letters[4..])
    } else {
        letters
    }
}

/// `POST /v1/auth/device`
pub async fn device(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    body: Bytes,
) -> ApiResult<Json<DeviceAuthorization>> {
    limit(&state, &ctx, "auth")?;
    let request: DeviceAuthorizationRequest = parse(&body)?;
    ed25519_dalek::VerifyingKey::from_bytes(&request.signing_key.0)
        .map_err(|_| ApiError::bad_request("invalid signing key"))?;
    let label = request.label.trim();
    if label.is_empty() || label.len() > 200 || label.chars().any(char::is_control) {
        return Err(ApiError::bad_request("invalid label"));
    }
    if state.oidc.is_none() {
        return Err(ApiError::new(
            ErrorCode::Unavailable,
            "no identity provider is configured",
        ));
    }
    let device_code = b64(&random::<32>());
    let now = state.now();
    let code = loop {
        let code = user_code();
        let taken = sqlx::query(
            state
                .db
                .sql("SELECT 1 AS taken FROM device_grants WHERE user_code = $1"),
        )
        .bind(code.clone())
        .fetch_optional(state.db.pool())
        .await?;
        if taken.is_none() {
            break code;
        }
    };
    sqlx::query(state.db.sql("INSERT INTO device_grants (code_hash, user_code, signing_key, label, created, expires, user_id, status, last_poll) VALUES ($1, $2, $3, $4, $5, $6, NULL, 'pending', 0)"))
        .bind(sha256(device_code.as_bytes()).to_vec())
        .bind(code.clone())
        .bind(request.signing_key.0.to_vec())
        .bind(label.to_owned())
        .bind(to_i64(now))
        .bind(to_i64(now + GRANT_LIFETIME))
        .execute(state.db.pool())
        .await?;
    audit::record(
        &state,
        Some(&ctx),
        Event::ok("auth.device").detail(label.to_owned()),
    )
    .await?;
    let verification_uri = state.config.url("/auth/verify");
    Ok(Json(DeviceAuthorization {
        device_code,
        verification_uri_complete: format!("{verification_uri}?user_code={code}"),
        verification_uri,
        user_code: code,
        expires_in: GRANT_LIFETIME / 1_000,
        interval: POLL_INTERVAL / 1_000,
    }))
}

/// `POST /v1/auth/token`
pub async fn token(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    body: Bytes,
) -> ApiResult<Json<TokenResponse>> {
    let request: TokenRequest = parse(&body)?;
    let now = state.now();
    let claim = check_claim(&request.claim, purpose::DEVICE, now)?;
    let hash = sha256(claim.value.as_bytes()).to_vec();
    let row = sqlx::query(state.db.sql("SELECT signing_key, expires, user_id, status, last_poll FROM device_grants WHERE code_hash = $1"))
        .bind(hash.clone())
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ExpiredToken, "unknown device code"))?;
    if db::bytes32(&row, "signing_key")? != claim.signing_key.0 {
        return Err(ApiError::new(
            ErrorCode::InvalidProof,
            "the claim is signed by another key",
        ));
    }
    if db::uint(&row, "expires")? <= now {
        return Err(ApiError::new(
            ErrorCode::ExpiredToken,
            "the device code expired",
        ));
    }
    let status: String = row.try_get("status")?;
    match status.as_str() {
        "pending" => {
            let last_poll = db::uint(&row, "last_poll")?;
            sqlx::query(
                state
                    .db
                    .sql("UPDATE device_grants SET last_poll = $1 WHERE code_hash = $2"),
            )
            .bind(to_i64(now))
            .bind(hash)
            .execute(state.db.pool())
            .await?;
            if now.saturating_sub(last_poll) < POLL_INTERVAL {
                Err(ApiError::new(ErrorCode::SlowDown, "poll less often"))
            } else {
                Err(ApiError::new(
                    ErrorCode::AuthorizationPending,
                    "waiting for approval",
                ))
            }
        }
        "denied" => Err(ApiError::new(
            ErrorCode::AccessDenied,
            "the sign-in was denied",
        )),
        "approved" => {
            let user = db::optional_id(&row, "user_id")?.ok_or_else(ApiError::internal)?;
            let consumed = sqlx::query(state.db.sql("UPDATE device_grants SET status = 'consumed' WHERE code_hash = $1 AND status = 'approved'"))
                .bind(hash)
                .execute(state.db.pool())
                .await?
                .rows_affected();
            if consumed == 0 {
                return Err(ApiError::new(
                    ErrorCode::ExpiredToken,
                    "the device code was used",
                ));
            }
            let (token, expires) = create_session(
                &state,
                Some(user),
                &claim.signing_key,
                SessionKind::Person,
                PERSON_LIFETIME,
                None,
            )
            .await?;
            ctx.set_user(Some(user), SessionKind::Person.label());
            audit::record(
                &state,
                Some(&ctx),
                Event::ok("auth.session").detail("device grant"),
            )
            .await?;
            Ok(Json(TokenResponse {
                token,
                expires,
                user: Some(user),
            }))
        }
        _ => Err(ApiError::new(
            ErrorCode::ExpiredToken,
            "the device code was used",
        )),
    }
}

async fn start_flow(
    state: &AppState,
    purpose: &str,
    user_code: Option<String>,
) -> ApiResult<Response> {
    let oidc = state.oidc.as_ref().ok_or_else(|| {
        ApiError::new(ErrorCode::Unavailable, "no identity provider is configured")
    })?;
    let flow = oidc.start().await.map_err(|error| {
        tracing::warn!(error = %format!("{error:#}"), "OIDC discovery failed");
        ApiError::new(
            ErrorCode::Unavailable,
            "the identity provider is unavailable",
        )
    })?;
    sqlx::query(state.db.sql("INSERT INTO oidc_flows (state, nonce, verifier, purpose, user_code, created, user_id, approval) VALUES ($1, $2, $3, $4, $5, $6, NULL, NULL)"))
        .bind(flow.state)
        .bind(flow.nonce)
        .bind(flow.verifier)
        .bind(purpose.to_owned())
        .bind(user_code)
        .bind(to_i64(state.now()))
        .execute(state.db.pool())
        .await?;
    Ok(Redirect::to(&flow.url).into_response())
}

async fn pending_grant(state: &AppState, user_code: &str) -> ApiResult<Option<(String, [u8; 32])>> {
    let row = sqlx::query(state.db.sql("SELECT label, signing_key FROM device_grants WHERE user_code = $1 AND status = 'pending' AND expires > $2"))
        .bind(user_code.to_owned())
        .bind(to_i64(state.now()))
        .fetch_optional(state.db.pool())
        .await?;
    row.map(|row| Ok((row.try_get("label")?, db::bytes32(&row, "signing_key")?)))
        .transpose()
}

/// `GET /auth/verify`
pub async fn verify(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let Some(code) = query.get("user_code").map(|code| normalize_user_code(code)) else {
        return page(
            StatusCode::OK,
            "Sign in a device",
            "<form method=\"get\" action=\"/auth/verify\"><label>Code shown on the device <input name=\"user_code\" autocomplete=\"off\" autofocus required></label> <button type=\"submit\">Continue</button></form>",
        );
    };
    if limit(&state, &ctx, "verify").is_err() {
        return message(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many attempts",
            "Wait a minute and try again.",
        );
    }
    match pending_grant(&state, &code).await {
        Ok(Some(_)) => match start_flow(&state, "device", Some(code)).await {
            Ok(response) => response,
            Err(error) => message(
                StatusCode::SERVICE_UNAVAILABLE,
                "Sign-in unavailable",
                &error.message,
            ),
        },
        Ok(None) => message(
            StatusCode::NOT_FOUND,
            "Unknown code",
            "This code is unknown or expired. Start the sign-in on the device again.",
        ),
        Err(_) => message(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error",
            "The server could not look up the code.",
        ),
    }
}

/// `GET /auth/web/start`
pub async fn web_start(State(state): State<AppState>) -> Response {
    match start_flow(&state, "web", None).await {
        Ok(response) => response,
        Err(error) => message(
            StatusCode::SERVICE_UNAVAILABLE,
            "Sign-in unavailable",
            &error.message,
        ),
    }
}

/// `GET /auth/callback`
pub async fn callback(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    match finish_callback(&state, &ctx, &query).await {
        Ok(response) => response,
        Err(error) => {
            let _ = audit::record(
                &state,
                Some(&ctx),
                Event::denied("auth.oidc").detail(error.message.clone()),
            )
            .await;
            message(StatusCode::BAD_REQUEST, "Sign-in failed", &error.message)
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn finish_callback(
    state: &AppState,
    ctx: &RequestCtx,
    query: &HashMap<String, String>,
) -> ApiResult<Response> {
    if let Some(error) = query.get("error") {
        return Err(ApiError::bad_request(format!(
            "the identity provider reported {error}"
        )));
    }
    let flow_state = query
        .get("state")
        .ok_or_else(|| ApiError::bad_request("missing state"))?;
    let code = query
        .get("code")
        .ok_or_else(|| ApiError::bad_request("missing code"))?;
    let row = sqlx::query(state.db.sql("SELECT nonce, verifier, purpose, user_code, created FROM oidc_flows WHERE state = $1 AND user_id IS NULL"))
        .bind(flow_state.clone())
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| ApiError::bad_request("unknown or used sign-in state"))?;
    let now = state.now();
    if db::uint(&row, "created")? + FLOW_LIFETIME <= now {
        return Err(ApiError::bad_request("the sign-in took too long"));
    }
    let nonce: String = row.try_get("nonce")?;
    let verifier: String = row.try_get("verifier")?;
    let purpose: String = row.try_get("purpose")?;
    let user_code: Option<String> = row.try_get("user_code")?;
    let oidc = state.oidc.as_ref().ok_or_else(ApiError::internal)?;
    let identity = oidc
        .finish(code, &verifier, &nonce, now)
        .await
        .map_err(|error| {
            tracing::warn!(error = %format!("{error:#}"), "OIDC sign-in failed");
            ApiError::bad_request("the identity provider's answer did not verify")
        })?;
    let user = users::sign_in(state, &identity).await?;
    ctx.set_user(Some(user), SessionKind::Person.label());
    let active = users::summary(state, user)
        .await?
        .is_some_and(|summary| summary.active);
    if !active {
        sqlx::query(state.db.sql("DELETE FROM oidc_flows WHERE state = $1"))
            .bind(flow_state.clone())
            .execute(state.db.pool())
            .await?;
        return Err(ApiError::forbidden("this account is suspended"));
    }
    audit::record(
        state,
        Some(ctx),
        Event::ok("auth.oidc").detail(purpose.clone()),
    )
    .await?;
    match purpose.as_str() {
        "device" => {
            let user_code = user_code.ok_or_else(ApiError::internal)?;
            let (label, key) = pending_grant(state, &user_code)
                .await?
                .ok_or_else(|| ApiError::bad_request("the device code expired"))?;
            let approval = b64(&random::<24>());
            sqlx::query(
                state
                    .db
                    .sql("UPDATE oidc_flows SET user_id = $1, approval = $2 WHERE state = $3"),
            )
            .bind(user.to_string())
            .bind(approval.clone())
            .bind(flow_state.clone())
            .execute(state.db.pool())
            .await?;
            Ok(page(
                StatusCode::OK,
                "Approve this device?",
                &format!(
                    "<p>Signed in as {}.</p><dl><dt>Device</dt><dd>{}</dd><dt>Fingerprint</dt><dd><code>{}</code></dd></dl><p>Approve only if the device shows the same fingerprint.</p><form method=\"post\" action=\"/auth/verify/approve\"><input type=\"hidden\" name=\"approval\" value=\"{}\"><button type=\"submit\" name=\"decision\" value=\"approve\">Approve</button> <button type=\"submit\" name=\"decision\" value=\"deny\">Deny</button></form>",
                    escape(if identity.email.is_empty() {
                        &identity.name
                    } else {
                        &identity.email
                    }),
                    escape(&label),
                    escape(&key_fingerprint(&key)),
                    escape(&approval),
                ),
            ))
        }
        "web" => {
            sqlx::query(state.db.sql("DELETE FROM oidc_flows WHERE state = $1"))
                .bind(flow_state.clone())
                .execute(state.db.pool())
                .await?;
            let cookie = b64(&random::<32>());
            sqlx::query(
                state.db.sql(
                    "INSERT INTO web_logins (cookie_hash, user_id, expires) VALUES ($1, $2, $3)",
                ),
            )
            .bind(sha256(cookie.as_bytes()).to_vec())
            .bind(user.to_string())
            .bind(to_i64(now + FLOW_LIFETIME))
            .execute(state.db.pool())
            .await?;
            let secure = if state.config.https() { "; Secure" } else { "" };
            let mut response = Redirect::to("/console/").into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                HeaderValue::from_str(&format!(
                    "{WEB_COOKIE}={cookie}; Path=/v1/auth/web; HttpOnly; SameSite=Strict; Max-Age=600{secure}"
                ))
                .map_err(|_| ApiError::internal())?,
            );
            Ok(response)
        }
        _ => Err(ApiError::internal()),
    }
}

#[derive(Debug, Deserialize)]
pub struct Approval {
    approval: String,
    decision: String,
}

/// `POST /auth/verify/approve`
pub async fn approve(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    Form(form): Form<Approval>,
) -> Response {
    match finish_approval(&state, &ctx, &form).await {
        Ok(true) => message(
            StatusCode::OK,
            "Device approved",
            "Return to the device; it signs in within a few seconds.",
        ),
        Ok(false) => message(
            StatusCode::OK,
            "Device denied",
            "The device was not signed in.",
        ),
        Err(error) => message(StatusCode::BAD_REQUEST, "Approval failed", &error.message),
    }
}

async fn finish_approval(state: &AppState, ctx: &RequestCtx, form: &Approval) -> ApiResult<bool> {
    let row = sqlx::query(
        state
            .db
            .sql("SELECT state, user_id, user_code, created FROM oidc_flows WHERE approval = $1"),
    )
    .bind(form.approval.clone())
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| ApiError::bad_request("unknown or used approval"))?;
    let flow_state: String = row.try_get("state")?;
    sqlx::query(state.db.sql("DELETE FROM oidc_flows WHERE state = $1"))
        .bind(flow_state)
        .execute(state.db.pool())
        .await?;
    if db::uint(&row, "created")? + FLOW_LIFETIME <= state.now() {
        return Err(ApiError::bad_request("the approval took too long"));
    }
    let user = db::optional_id(&row, "user_id")?.ok_or_else(ApiError::internal)?;
    let user_code: String = row
        .try_get::<Option<String>, _>("user_code")?
        .ok_or_else(ApiError::internal)?;
    let approved = form.decision == "approve";
    let updated = sqlx::query(state.db.sql("UPDATE device_grants SET status = $1, user_id = $2 WHERE user_code = $3 AND status = 'pending' AND expires > $4"))
        .bind(if approved { "approved" } else { "denied" })
        .bind(user.to_string())
        .bind(user_code)
        .bind(to_i64(state.now()))
        .execute(state.db.pool())
        .await?
        .rows_affected();
    if updated == 0 {
        return Err(ApiError::bad_request("the device code expired"));
    }
    ctx.set_user(Some(user), SessionKind::Person.label());
    audit::record(
        state,
        Some(ctx),
        if approved {
            Event::ok("auth.device.approve")
        } else {
            Event::denied("auth.device.approve")
        },
    )
    .await?;
    Ok(approved)
}

fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value.to_owned())
}

/// `POST /v1/auth/web`
pub async fn web(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<Json<TokenResponse>> {
    limit(&state, &ctx, "auth")?;
    let request: TokenRequest = parse(&body)?;
    let now = state.now();
    let claim = check_claim(&request.claim, purpose::WEB, now)?;
    let value =
        cookie(&headers, WEB_COOKIE).ok_or_else(|| ApiError::unauthorized("no console sign-in"))?;
    let hash = sha256(value.as_bytes()).to_vec();
    let row = sqlx::query(
        state
            .db
            .sql("SELECT user_id, expires FROM web_logins WHERE cookie_hash = $1"),
    )
    .bind(hash.clone())
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(|| ApiError::unauthorized("unknown console sign-in"))?;
    sqlx::query(
        state
            .db
            .sql("DELETE FROM web_logins WHERE cookie_hash = $1"),
    )
    .bind(hash)
    .execute(state.db.pool())
    .await?;
    if db::uint(&row, "expires")? <= now {
        return Err(ApiError::unauthorized("the console sign-in expired"));
    }
    let user = db::id(&row, "user_id")?;
    let (token, expires) = create_session(
        &state,
        Some(user),
        &claim.signing_key,
        SessionKind::Person,
        PERSON_LIFETIME,
        None,
    )
    .await?;
    ctx.set_user(Some(user), SessionKind::Person.label());
    audit::record(
        &state,
        Some(&ctx),
        Event::ok("auth.session").detail("console"),
    )
    .await?;
    Ok(Json(TokenResponse {
        token,
        expires,
        user: Some(user),
    }))
}

fn unverified_issuer(jwt: &str) -> Option<String> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = unb64(payload).ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims.get("iss")?.as_str().map(ToOwned::to_owned)
}

/// `POST /v1/auth/workload`
pub async fn workload(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    body: Bytes,
) -> ApiResult<Json<TokenResponse>> {
    limit(&state, &ctx, "auth")?;
    let request: TokenRequest = parse(&body)?;
    let now = state.now();
    let claim = check_claim(&request.claim, purpose::WORKLOAD, now)?;
    let jwt = request
        .jwt
        .ok_or_else(|| ApiError::bad_request("missing workload token"))?;
    if claim.value != b64(&sha256(jwt.as_bytes())) {
        return Err(ApiError::new(
            ErrorCode::InvalidProof,
            "the claim names another token",
        ));
    }
    let issuer = unverified_issuer(&jwt)
        .ok_or_else(|| ApiError::unauthorized("malformed workload token"))?;
    let (config, jwks) = state
        .workload
        .iter()
        .find(|(config, _)| config.issuer == issuer)
        .ok_or_else(|| ApiError::unauthorized("unknown workload issuer"))?;
    let claims = match jwks
        .verify(&jwt, &config.issuer, &config.audience, now)
        .await
    {
        Ok(claims) => claims,
        Err(error) => {
            audit::record(
                &state,
                Some(&ctx),
                Event::denied("auth.workload").detail(format!("{error:#}")),
            )
            .await?;
            return Err(ApiError::unauthorized("the workload token does not verify"));
        }
    };
    let subject = claims
        .get("sub")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let Some(rule) = config.rules.iter().find(|rule| rule.matches(subject)) else {
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("auth.workload").detail(format!("no rule for {subject}")),
        )
        .await?;
        return Err(ApiError::forbidden("no rule admits this workload"));
    };
    let user = users::machine(&state, &rule.machine).await?;
    if !users::summary(&state, user)
        .await?
        .is_some_and(|summary| summary.active)
    {
        return Err(ApiError::unauthorized("this machine is suspended"));
    }
    let (token, expires) = create_session(
        &state,
        Some(user),
        &claim.signing_key,
        SessionKind::Machine,
        PERSON_LIFETIME,
        None,
    )
    .await?;
    ctx.set_user(Some(user), SessionKind::Machine.label());
    audit::record(
        &state,
        Some(&ctx),
        Event::ok("auth.session").detail(format!("workload {subject}")),
    )
    .await?;
    Ok(Json(TokenResponse {
        token,
        expires,
        user: Some(user),
    }))
}

/// The end of the open emergency window, if one is open.
///
/// # Errors
///
/// Fails when the database does.
pub async fn emergency_until(state: &AppState) -> ApiResult<Option<u64>> {
    let row = sqlx::query(state.db.sql("SELECT value FROM settings WHERE name = $1"))
        .bind("emergency_until")
        .fetch_optional(state.db.pool())
        .await?;
    let until = row
        .map(|row| row.try_get::<String, _>("value"))
        .transpose()?
        .and_then(|value| value.parse::<u64>().ok());
    Ok(until.filter(|until| *until > state.now()))
}

/// `POST /v1/auth/emergency`
pub async fn emergency(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    body: Bytes,
) -> ApiResult<Json<TokenResponse>> {
    limit(&state, &ctx, "auth")?;
    let request: TokenRequest = parse(&body)?;
    let now = state.now();
    let claim = check_claim(&request.claim, purpose::EMERGENCY, now)?;
    let Some(until) = emergency_until(&state).await? else {
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("auth.emergency").detail("window closed"),
        )
        .await?;
        return Err(ApiError::forbidden("no emergency window is open"));
    };
    let user = device_user(&state, &claim.signing_key)
        .await?
        .ok_or_else(|| ApiError::unauthorized("no active device has this key"))?;
    if !users::summary(&state, user)
        .await?
        .is_some_and(|summary| summary.active)
    {
        return Err(ApiError::unauthorized("user suspended"));
    }
    let (token, expires) = create_session(
        &state,
        Some(user),
        &claim.signing_key,
        SessionKind::Emergency,
        EMERGENCY_LIFETIME,
        Some(until),
    )
    .await?;
    ctx.set_user(Some(user), SessionKind::Emergency.label());
    audit::record(&state, Some(&ctx), Event::ok("auth.emergency")).await?;
    Ok(Json(TokenResponse {
        token,
        expires,
        user: Some(user),
    }))
}

/// The user owning an active device with `key` in some organization.
async fn device_user(state: &AppState, key: &Fixed<32>) -> ApiResult<Option<Id>> {
    let rows = sqlx::query("SELECT id FROM orgs")
        .fetch_all(state.db.pool())
        .await?;
    for row in rows {
        let org = db::id(&row, "id")?;
        let log = state.logs.get(&state.db, org).await?;
        if let Some(device) = log
            .state()
            .devices
            .values()
            .find(|device| device.active && device.record.signing_key == *key)
        {
            return Ok(Some(device.record.user));
        }
    }
    Ok(None)
}

/// `POST /v1/auth/logout`
pub async fn logout(State(state): State<AppState>, authed: Authed) -> ApiResult<StatusCode> {
    sqlx::query(
        state
            .db
            .sql("UPDATE sessions SET revoked = 1 WHERE token_hash = $1"),
    )
    .bind(authed.session.token_hash.clone())
    .execute(state.db.pool())
    .await?;
    audit::record(&state, Some(&authed.ctx), Event::ok("auth.logout")).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_codes_use_the_unambiguous_alphabet() {
        for _ in 0..100 {
            let code = user_code();
            assert_eq!(code.len(), 9);
            assert_eq!(&code[4..5], "-");
            assert!(
                code.chars()
                    .filter(|c| *c != '-')
                    .all(|c| USER_CODE_ALPHABET.contains(&(c as u8)))
            );
        }
    }

    #[test]
    fn typed_user_codes_are_normalized() {
        assert_eq!(normalize_user_code("bcdf ghjk"), "BCDF-GHJK");
        assert_eq!(normalize_user_code("BCDF-GHJK"), "BCDF-GHJK");
        assert_eq!(normalize_user_code("bcd"), "BCD");
    }

    #[test]
    fn key_fingerprints_have_five_groups() {
        let fingerprint = key_fingerprint(&[1; 32]);
        assert_eq!(fingerprint.split(' ').count(), 5);
        assert!(fingerprint.split(' ').all(|group| group.len() == 8));
    }

    #[test]
    fn cookies_are_found_among_others() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("a=1; pasejo_web=xyz; b=2"),
        );
        assert_eq!(cookie(&headers, WEB_COOKIE).as_deref(), Some("xyz"));
        assert_eq!(cookie(&headers, "missing"), None);
    }

    #[test]
    fn issuers_are_read_from_unverified_tokens() {
        let payload = b64(br#"{"iss":"https://kubernetes.default.svc"}"#);
        assert_eq!(
            unverified_issuer(&format!("e30.{payload}.sig")).as_deref(),
            Some("https://kubernetes.default.svc")
        );
        assert_eq!(unverified_issuer("garbage"), None);
    }
}
