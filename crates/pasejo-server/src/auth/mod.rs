// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sessions, request proofs and the per-organization view of a session.

pub mod break_glass;
pub mod flows;

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use pasejo_protocol::api::ErrorCode;
use pasejo_protocol::device::DeviceRecord;
use pasejo_protocol::encoding::{Fixed, b64, random, sha256};
use pasejo_protocol::log::{OrgLog, Role};
use pasejo_protocol::request::{
    MAX_SKEW_MILLIS, NONCE_HEADER, PROOF_HEADER, Received, TIMESTAMP_HEADER, check,
    token_from_header,
};
use pasejo_protocol::{Id, Millis};
use serde::de::DeserializeOwned;
use sqlx::Row;

use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult, is_unique_violation};
use crate::state::{AppState, RequestCtx};

pub const PERSON_LIFETIME: Millis = 12 * 3_600_000;
pub const BREAK_GLASS_LIFETIME: Millis = 15 * 60_000;
pub const EMERGENCY_LIFETIME: Millis = 3_600_000;

/// What kind of sign-in created a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Person,
    Machine,
    BreakGlass,
    Emergency,
}

impl SessionKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Machine => "machine",
            Self::BreakGlass => "break-glass",
            Self::Emergency => "emergency",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "person" => Some(Self::Person),
            "machine" => Some(Self::Machine),
            "break-glass" => Some(Self::BreakGlass),
            "emergency" => Some(Self::Emergency),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub token_hash: Vec<u8>,
    pub user: Option<Id>,
    pub signing_key: Fixed<32>,
    pub kind: SessionKind,
    pub expires: Millis,
}

/// Creates a session bound to `signing_key` and returns its token. The
/// session lasts `lifetime`, and never past `ends_by`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn create_session(
    state: &AppState,
    user: Option<Id>,
    signing_key: &Fixed<32>,
    kind: SessionKind,
    lifetime: Millis,
    ends_by: Option<Millis>,
) -> ApiResult<(String, Millis)> {
    let token = b64(&random::<32>());
    let now = state.now();
    let expires = ends_by.map_or(now + lifetime, |end| end.min(now + lifetime));
    sqlx::query(state.db.sql("INSERT INTO sessions (token_hash, user_id, signing_key, kind, created, expires, revoked) VALUES ($1, $2, $3, $4, $5, $6, 0)"))
        .bind(sha256(token.as_bytes()).to_vec())
        .bind(user.map(|user| user.to_string()))
        .bind(signing_key.0.to_vec())
        .bind(kind.label())
        .bind(to_i64(now))
        .bind(to_i64(expires))
        .execute(state.db.pool())
        .await?;
    Ok((token, expires))
}

/// Revokes every session of `user`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn revoke_sessions_of(state: &AppState, user: Id) -> ApiResult<u64> {
    Ok(sqlx::query(
        state
            .db
            .sql("UPDATE sessions SET revoked = 1 WHERE user_id = $1 AND revoked = 0"),
    )
    .bind(user.to_string())
    .execute(state.db.pool())
    .await?
    .rows_affected())
}

/// Revokes every session bound to `signing_key`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn revoke_sessions_of_key(state: &AppState, signing_key: &Fixed<32>) -> ApiResult<u64> {
    Ok(sqlx::query(
        state
            .db
            .sql("UPDATE sessions SET revoked = 1 WHERE signing_key = $1 AND revoked = 0"),
    )
    .bind(signing_key.0.to_vec())
    .execute(state.db.pool())
    .await?
    .rows_affected())
}

async fn load_session(state: &AppState, token: &str) -> ApiResult<Session> {
    let row = sqlx::query(state.db.sql("SELECT s.user_id, s.signing_key, s.kind, s.expires, s.revoked, u.active FROM sessions s LEFT JOIN users u ON u.id = s.user_id WHERE s.token_hash = $1"))
        .bind(sha256(token.as_bytes()).to_vec())
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| ApiError::unauthorized("unknown session"))?;
    let expires = db::uint(&row, "expires")?;
    if db::uint(&row, "revoked")? != 0 || expires <= state.now() {
        return Err(ApiError::unauthorized("session ended"));
    }
    let user = db::optional_id(&row, "user_id")?;
    if user.is_some() && row.try_get::<Option<i64>, _>("active")? != Some(1) {
        return Err(ApiError::unauthorized("user suspended"));
    }
    let kind: String = row.try_get("kind")?;
    Ok(Session {
        token_hash: sha256(token.as_bytes()).to_vec(),
        user,
        signing_key: Fixed(db::bytes32(&row, "signing_key")?),
        kind: SessionKind::parse(&kind).ok_or_else(ApiError::internal)?,
        expires,
    })
}

/// Records a request nonce; a nonce seen within the window is refused.
async fn remember_nonce(state: &AppState, nonce: &str, now: Millis) -> ApiResult<()> {
    let result = sqlx::query(
        state
            .db
            .sql("INSERT INTO nonces (nonce, expires) VALUES ($1, $2)"),
    )
    .bind(nonce.to_owned())
    .bind(to_i64(now + 2 * MAX_SKEW_MILLIS))
    .execute(state.db.pool())
    .await;
    match result {
        Ok(_) => Ok(()),
        Err(error) if is_unique_violation(&error) => {
            Err(ApiError::new(ErrorCode::InvalidProof, "nonce already used"))
        }
        Err(error) => Err(error.into()),
    }
}

/// An authenticated request: a live session, a verified request proof and
/// the request body the proof covers.
pub struct Authed {
    pub session: Session,
    pub body: Bytes,
    pub ctx: Arc<RequestCtx>,
}

impl Authed {
    /// Parses the body as JSON.
    ///
    /// # Errors
    ///
    /// Fails with `bad_request` for malformed JSON.
    pub fn json<T: DeserializeOwned>(&self) -> ApiResult<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// The session's user; break-glass sessions have none.
    ///
    /// # Errors
    ///
    /// Fails with `forbidden` for a session without a user.
    pub fn user(&self) -> ApiResult<Id> {
        self.session
            .user
            .ok_or_else(|| ApiError::forbidden("this session has no user"))
    }

    /// Requires a break-glass session.
    ///
    /// # Errors
    ///
    /// Fails with `forbidden` otherwise.
    pub fn require_break_glass(&self) -> ApiResult<()> {
        if self.session.kind == SessionKind::BreakGlass {
            Ok(())
        } else {
            Err(ApiError::forbidden("break-glass session required"))
        }
    }
}

fn header<'a>(parts: &'a axum::http::request::Parts, name: &str) -> &'a str {
    parts
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

impl FromRequest<AppState> for Authed {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> ApiResult<Self> {
        let (parts, body) = request.into_parts();
        let ctx = parts
            .extensions
            .get::<Arc<RequestCtx>>()
            .cloned()
            .ok_or_else(ApiError::internal)?;
        let token = token_from_header(header(&parts, "authorization"))
            .ok_or_else(|| ApiError::unauthorized("missing session token"))?
            .to_owned();
        let session = load_session(state, &token).await?;
        ctx.set_user(session.user, session.kind.label());
        let method = parts.method.as_str().to_owned();
        // Nested routers see the URI without their prefix; the proof covers
        // the URI the client sent.
        let uri = parts
            .extensions
            .get::<axum::extract::OriginalUri>()
            .map_or(&parts.uri, |original| &original.0);
        let path_and_query = uri
            .path_and_query()
            .map_or_else(|| uri.path().to_owned(), ToString::to_string);
        let timestamp = header(&parts, TIMESTAMP_HEADER).to_owned();
        let nonce = header(&parts, NONCE_HEADER).to_owned();
        let proof = header(&parts, PROOF_HEADER).to_owned();
        let body = Bytes::from_request(Request::from_parts(parts, body), state)
            .await
            .map_err(|_| ApiError::new(ErrorCode::TooLarge, "request body too large"))?;
        let now = state.now();
        check(
            &Received {
                method: &method,
                path_and_query: &path_and_query,
                timestamp: &timestamp,
                nonce: &nonce,
                proof: &proof,
                body: &body,
                token: &token,
            },
            &session.signing_key,
            now,
        )
        .map_err(|error| ApiError::new(ErrorCode::InvalidProof, error.to_string()))?;
        remember_nonce(state, &nonce, now).await?;
        Ok(Self { session, body, ctx })
    }
}

/// A session's view of one organization.
pub struct OrgAccess {
    pub org: Id,
    pub log: Arc<OrgLog>,
    pub user: Option<Id>,
    pub device: Option<DeviceRecord>,
    pub role: Option<Role>,
    pub break_glass: bool,
}

impl OrgAccess {
    /// Resolves the session's device and role in `org`. A user whom SCIM
    /// deactivated in the organization holds neither, even before the log
    /// removes them.
    ///
    /// # Errors
    ///
    /// Fails with `unknown_org` for an unknown organization.
    pub async fn load(state: &AppState, authed: &Authed, org: Id) -> ApiResult<Self> {
        let log = state.logs.get(&state.db, org).await?;
        let user = authed.session.user;
        let suspended = match user {
            Some(user) => scim_suspended(state, org, user).await?,
            None => false,
        };
        let device = if suspended {
            None
        } else {
            user.and_then(|user| {
                log.state()
                    .devices_of(user)
                    .find(|device| device.signing_key == authed.session.signing_key)
                    .cloned()
            })
        };
        let role = if suspended {
            None
        } else {
            user.and_then(|user| log.state().members.get(&user).copied())
        };
        authed
            .ctx
            .set_device(device.as_ref().map(|device| device.id));
        Ok(Self {
            org,
            log,
            user,
            device,
            role,
            break_glass: authed.session.kind == SessionKind::BreakGlass,
        })
    }

    /// The session's active device in the organization.
    ///
    /// # Errors
    ///
    /// Fails with `pending_device` while the device is not in the log.
    pub fn device(&self) -> ApiResult<&DeviceRecord> {
        self.device.as_ref().ok_or_else(|| {
            ApiError::new(
                ErrorCode::PendingDevice,
                "this device is not active in the organization",
            )
        })
    }

    /// Requires an active device whose user holds one of `roles`.
    ///
    /// # Errors
    ///
    /// Fails with `pending_device` or `forbidden`.
    pub fn require(&self, roles: &[Role]) -> ApiResult<&DeviceRecord> {
        let device = self.device()?;
        match self.role {
            Some(role) if roles.contains(&role) => Ok(device),
            _ => Err(ApiError::forbidden("your role does not allow this")),
        }
    }

    #[must_use]
    pub fn has_role(&self, roles: &[Role]) -> bool {
        self.device.is_some() && self.role.is_some_and(|role| roles.contains(&role))
    }
}

/// Whether SCIM deactivated or deleted `user` in `org`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn scim_suspended(state: &AppState, org: Id, user: Id) -> ApiResult<bool> {
    let row = sqlx::query(
        state
            .db
            .sql("SELECT active, deleted FROM scim_users WHERE org = $1 AND user_id = $2"),
    )
    .bind(org.to_string())
    .bind(user.to_string())
    .fetch_optional(state.db.pool())
    .await?;
    Ok(match row {
        Some(row) => db::uint(&row, "active")? == 0 || db::uint(&row, "deleted")? != 0,
        None => false,
    })
}

/// Parses an identifier.
///
/// # Errors
///
/// Fails with `bad_request`.
pub fn id(text: &str) -> ApiResult<Id> {
    Id::from_str(text).map_err(|_| ApiError::bad_request("malformed identifier"))
}

/// Checks a `token/v1` claim: its signature by the key it names, its
/// purpose and its freshness.
///
/// # Errors
///
/// Fails with `bad_request` or `invalid_proof`.
pub fn check_claim(
    envelope: &pasejo_protocol::envelope::Envelope,
    purpose: &str,
    now: Millis,
) -> ApiResult<pasejo_protocol::envelope::TokenClaim> {
    use pasejo_protocol::envelope::{Context, TokenClaim, verify};
    let claim: TokenClaim = envelope.open()?;
    let signature = envelope
        .signatures
        .first()
        .ok_or_else(|| ApiError::new(ErrorCode::InvalidProof, "unsigned claim"))?;
    verify(
        Context::Token,
        &envelope.payload.0,
        &claim.signing_key.0,
        &signature.signature.0,
    )
    .map_err(|_| ApiError::new(ErrorCode::InvalidProof, "claim signature does not verify"))?;
    if claim.purpose != purpose {
        return Err(ApiError::bad_request("claim has another purpose"));
    }
    if claim.time.abs_diff(now) > MAX_SKEW_MILLIS {
        return Err(ApiError::new(ErrorCode::InvalidProof, "claim is stale"));
    }
    ed25519_dalek::VerifyingKey::from_bytes(&claim.signing_key.0)
        .map_err(|_| ApiError::bad_request("invalid signing key"))?;
    Ok(claim)
}
