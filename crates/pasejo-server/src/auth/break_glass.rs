// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The server break-glass account, a local account for when SSO is down.
//!
//! It is created on the server host and protected by a password and TOTP.
//! It manages users and opens the emergency window, and never reads a
//! secret or a name.

use std::sync::Arc;

use argon2::password_hash::rand_core::OsRng as PasswordRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use pasejo_protocol::api::{
    BreakGlassLogin, BreakGlassToken, EmergencyState, EmergencyWindow, ErrorCode, OrgList,
    OrgSummary, Users,
};
use pasejo_protocol::crypto::{open, seal};
use pasejo_protocol::encoding::random;
use sqlx::Row;
use zeroize::Zeroizing;

use super::flows::{emergency_until, limit, parse};
use super::{Authed, BREAK_GLASS_LIFETIME, SessionKind, create_session, revoke_sessions_of};
use crate::audit::{self, Event};
use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, RequestCtx, parse_id};
use crate::{totp, users};

const MAX_FAILURES: u64 = 5;
const LOCKOUT: u64 = 15 * 60_000;
const MAX_EMERGENCY: u64 = 24 * 3_600_000;
const PASSWORD_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
const TOTP_AAD: &[u8] = b"pasejo-server break-glass totp v1";

fn argon2() -> Argon2<'static> {
    let params = Params::new(64 * 1024, 3, 1, None)
        .unwrap_or_else(|_| unreachable!("constant parameters are valid"));
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// A freshly created break-glass account's credentials, shown once.
pub struct Created {
    pub password: Zeroizing<String>,
    pub totp_secret: Zeroizing<String>,
    pub totp_uri: Zeroizing<String>,
}

/// Creates or replaces the break-glass account `username`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn create(state: &AppState, username: &str) -> anyhow::Result<Created> {
    let password: Zeroizing<String> = Zeroizing::new(
        random::<32>()
            .iter()
            .map(|byte| char::from(PASSWORD_ALPHABET[usize::from(*byte) % PASSWORD_ALPHABET.len()]))
            .collect(),
    );
    let hash = argon2()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut PasswordRng))
        .map_err(|error| anyhow::anyhow!("cannot hash the password: {error}"))?
        .to_string();
    let secret = Zeroizing::new(random::<20>().to_vec());
    let sealed = seal(&state.keys.totp, &secret, TOTP_AAD)?;
    let db = &state.db;
    sqlx::query(db.sql("DELETE FROM break_glass WHERE username = $1"))
        .bind(username.to_owned())
        .execute(db.pool())
        .await?;
    sqlx::query(db.sql("INSERT INTO break_glass (username, password_hash, totp, failures, locked_until, last_step, disabled, created) VALUES ($1, $2, $3, 0, 0, 0, 0, $4)"))
        .bind(username.to_owned())
        .bind(hash)
        .bind(sealed)
        .bind(to_i64(state.now()))
        .execute(db.pool())
        .await?;
    let encoded = Zeroizing::new(totp::base32(&secret));
    let issuer = state
        .config
        .public_url
        .host_str()
        .unwrap_or("pasejo")
        .to_owned();
    Ok(Created {
        password,
        totp_uri: Zeroizing::new(format!(
            "otpauth://totp/{issuer}:{username}?secret={}&issuer={issuer}&algorithm=SHA1&digits=6&period=30",
            encoded.as_str()
        )),
        totp_secret: encoded,
    })
}

/// Disables the break-glass account `username`.
///
/// # Errors
///
/// Fails when the database does or the account does not exist.
pub async fn disable(state: &AppState, username: &str) -> anyhow::Result<()> {
    let changed = sqlx::query(
        state
            .db
            .sql("UPDATE break_glass SET disabled = 1 WHERE username = $1"),
    )
    .bind(username.to_owned())
    .execute(state.db.pool())
    .await?
    .rows_affected();
    if changed == 0 {
        anyhow::bail!("no break-glass account named {username}");
    }
    Ok(())
}

/// `POST /v1/break-glass/login`
#[allow(clippy::too_many_lines)]
pub async fn login(
    State(state): State<AppState>,
    Extension(ctx): Extension<Arc<RequestCtx>>,
    body: Bytes,
) -> ApiResult<Json<BreakGlassToken>> {
    if !state.config.break_glass.enabled {
        return Err(ApiError::not_found("break-glass access is disabled"));
    }
    if !state
        .config
        .break_glass
        .networks
        .iter()
        .any(|network| network.contains(ctx.ip))
    {
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("break-glass.login").detail("network not allowed"),
        )
        .await?;
        return Err(ApiError::forbidden(
            "break-glass login is not allowed from this network",
        ));
    }
    limit(&state, &ctx, "break-glass")?;
    let request: BreakGlassLogin = parse(&body)?;
    let password = Zeroizing::new(request.password);
    ed25519_dalek::VerifyingKey::from_bytes(&request.signing_key.0)
        .map_err(|_| ApiError::bad_request("invalid signing key"))?;
    let db = &state.db;
    let now = state.now();
    let row = sqlx::query(db.sql("SELECT password_hash, totp, failures, locked_until, last_step, disabled FROM break_glass WHERE username = $1"))
        .bind(request.username.clone())
        .fetch_optional(db.pool())
        .await?;
    let Some(row) = row else {
        // Spend the same time as a real verification so the answer does
        // not reveal whether the account exists.
        let _ =
            argon2().hash_password(password.as_bytes(), &SaltString::generate(&mut PasswordRng));
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("break-glass.login").detail("unknown account"),
        )
        .await?;
        return Err(ApiError::unauthorized("invalid credentials"));
    };
    if db::uint(&row, "disabled")? != 0 {
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("break-glass.login").detail("account disabled"),
        )
        .await?;
        return Err(ApiError::unauthorized("invalid credentials"));
    }
    if db::uint(&row, "locked_until")? > now {
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("break-glass.login").detail("account locked"),
        )
        .await?;
        return Err(ApiError::new(ErrorCode::Locked, "the account is locked"));
    }
    let hash: String = row.try_get("password_hash")?;
    let parsed = PasswordHash::new(&hash).map_err(|_| ApiError::internal())?;
    let password_ok = argon2()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();
    let sealed: Vec<u8> = row.try_get("totp")?;
    let secret = open(&state.keys.totp, &sealed, TOTP_AAD).map_err(|_| ApiError::internal())?;
    let step = if password_ok {
        totp::verify(&secret, &request.totp, now, db::uint(&row, "last_step")?)
    } else {
        None
    };
    let Some(step) = step else {
        let failures = db::uint(&row, "failures")? + 1;
        let locked_until = if failures >= MAX_FAILURES {
            now + LOCKOUT
        } else {
            0
        };
        sqlx::query(
            db.sql("UPDATE break_glass SET failures = $1, locked_until = $2 WHERE username = $3"),
        )
        .bind(to_i64(if failures >= MAX_FAILURES {
            0
        } else {
            failures
        }))
        .bind(to_i64(locked_until))
        .bind(request.username.clone())
        .execute(db.pool())
        .await?;
        audit::record(
            &state,
            Some(&ctx),
            Event::denied("break-glass.login").detail(if locked_until > 0 {
                "invalid credentials; account locked"
            } else {
                "invalid credentials"
            }),
        )
        .await?;
        return Err(ApiError::unauthorized("invalid credentials"));
    };
    sqlx::query(db.sql(
        "UPDATE break_glass SET failures = 0, locked_until = 0, last_step = $1 WHERE username = $2",
    ))
    .bind(to_i64(step))
    .bind(request.username.clone())
    .execute(db.pool())
    .await?;
    let (token, expires) = create_session(
        &state,
        None,
        &request.signing_key,
        SessionKind::BreakGlass,
        BREAK_GLASS_LIFETIME,
        None,
    )
    .await?;
    ctx.set_user(None, SessionKind::BreakGlass.label());
    audit::record(
        &state,
        Some(&ctx),
        Event::ok("break-glass.login").detail(request.username),
    )
    .await?;
    Ok(Json(BreakGlassToken { token, expires }))
}

/// `POST /v1/break-glass/emergency`
pub async fn emergency(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<Json<EmergencyState>> {
    authed.require_break_glass()?;
    let request: EmergencyWindow = authed.json()?;
    if request.duration > MAX_EMERGENCY {
        return Err(ApiError::bad_request(
            "the window may last at most 24 hours",
        ));
    }
    let until = if request.duration == 0 {
        0
    } else {
        state.now() + request.duration
    };
    let db = &state.db;
    sqlx::query(db.sql("DELETE FROM settings WHERE name = $1"))
        .bind("emergency_until")
        .execute(db.pool())
        .await?;
    sqlx::query(db.sql("INSERT INTO settings (name, value) VALUES ($1, $2)"))
        .bind("emergency_until")
        .bind(until.to_string())
        .execute(db.pool())
        .await?;
    if until == 0 {
        sqlx::query(db.sql("UPDATE sessions SET revoked = 1 WHERE kind = $1"))
            .bind(SessionKind::Emergency.label())
            .execute(db.pool())
            .await?;
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok(if until == 0 {
            "emergency.close"
        } else {
            "emergency.open"
        })
        .detail(until.to_string()),
    )
    .await?;
    Ok(Json(EmergencyState {
        until: emergency_until(&state).await?,
    }))
}

/// `GET /v1/break-glass/users`
pub async fn list_users(State(state): State<AppState>, authed: Authed) -> ApiResult<Json<Users>> {
    authed.require_break_glass()?;
    audit::record(&state, Some(&authed.ctx), Event::ok("break-glass.users")).await?;
    Ok(Json(Users {
        users: users::all(&state).await?,
    }))
}

/// `GET /v1/break-glass/orgs`
pub async fn list_orgs(State(state): State<AppState>, authed: Authed) -> ApiResult<Json<OrgList>> {
    authed.require_break_glass()?;
    let rows = sqlx::query("SELECT id, name FROM orgs ORDER BY created")
        .fetch_all(state.db.pool())
        .await?;
    let orgs = rows
        .iter()
        .map(|row| {
            Ok(OrgSummary {
                id: db::id(row, "id")?,
                name: row.try_get("name")?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    audit::record(&state, Some(&authed.ctx), Event::ok("break-glass.orgs")).await?;
    Ok(Json(OrgList { orgs }))
}

async fn set_active(
    state: &AppState,
    authed: &Authed,
    user: &str,
    active: bool,
) -> ApiResult<StatusCode> {
    authed.require_break_glass()?;
    let user = parse_id(user)?;
    if !users::set_active(state, user, active).await? {
        return Err(ApiError::not_found("no such user"));
    }
    if !active {
        revoke_sessions_of(state, user).await?;
    }
    audit::record(
        state,
        Some(&authed.ctx),
        Event::ok(if active {
            "user.reactivate"
        } else {
            "user.suspend"
        })
        .detail(format!("user {user}")),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /v1/break-glass/users/{id}/suspend`
pub async fn suspend(
    State(state): State<AppState>,
    Path(user): Path<String>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    set_active(&state, &authed, &user, false).await
}

/// `POST /v1/break-glass/users/{id}/reactivate`
pub async fn reactivate(
    State(state): State<AppState>,
    Path(user): Path<String>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    set_active(&state, &authed, &user, true).await
}

/// `POST /v1/break-glass/users/{id}/sessions/revoke`
pub async fn end_sessions(
    State(state): State<AppState>,
    Path(user): Path<String>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    authed.require_break_glass()?;
    let user = parse_id(&user)?;
    revoke_sessions_of(&state, user).await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("user.sessions.revoke").detail(format!("user {user}")),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
