// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use pasejo_protocol::api::{AuditPage, CheckpointEntry, Checkpoints, Cosign};
use pasejo_protocol::log::Role;

use crate::audit::{self, Event, SERVER_CHAIN};
use crate::auth::{Authed, OrgAccess};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, parse_id};

const PAGE: u64 = 1_000;

fn paging(query: &HashMap<String, String>) -> (u64, u64) {
    let since = query
        .get("since")
        .and_then(|since| since.parse().ok())
        .unwrap_or(0);
    let limit = query
        .get("limit")
        .and_then(|limit| limit.parse().ok())
        .unwrap_or(PAGE)
        .clamp(1, PAGE);
    (since, limit)
}

async fn page(
    state: &AppState,
    chain: &str,
    query: &HashMap<String, String>,
) -> ApiResult<AuditPage> {
    let (since, limit) = paging(query);
    let events = audit::events(state, chain, since, limit).await?;
    let next =
        (events.len() as u64 == limit).then(|| events.last().map_or(since, |event| event.seq));
    Ok(AuditPage { events, next })
}

async fn require_auditor(
    state: &AppState,
    authed: &Authed,
    org: pasejo_protocol::Id,
) -> ApiResult<()> {
    if authed.require_break_glass().is_ok() {
        return Ok(());
    }
    let access = OrgAccess::load(state, authed, org).await?;
    access.require(&[Role::Admin, Role::Auditor])?;
    Ok(())
}

/// `GET /v1/orgs/{org}/audit`
pub async fn events(
    State(state): State<AppState>,
    Path(org): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<AuditPage>> {
    let org = parse_id(&org)?;
    require_auditor(&state, &authed, org).await?;
    let result = page(&state, &org.to_string(), &query).await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("audit.read")
            .org(org)
            .detail(format!("{} events", result.events.len())),
    )
    .await?;
    Ok(Json(result))
}

/// `GET /v1/audit`: the server's own chain, for break-glass sessions.
pub async fn server_events(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<AuditPage>> {
    authed.require_break_glass()?;
    let result = page(&state, SERVER_CHAIN, &query).await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("audit.read").detail(format!("{} events", result.events.len())),
    )
    .await?;
    Ok(Json(result))
}

/// `GET /v1/orgs/{org}/audit/checkpoints`
pub async fn checkpoints(
    State(state): State<AppState>,
    Path(org): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<Checkpoints>> {
    let org = parse_id(&org)?;
    if authed.require_break_glass().is_err() {
        OrgAccess::load(&state, &authed, org).await?.device()?;
    }
    let (since, _) = paging(&query);
    let checkpoints = audit::checkpoints(&state, &org.to_string(), since)
        .await?
        .into_iter()
        .map(|(seq, checkpoint)| CheckpointEntry { seq, checkpoint })
        .collect();
    Ok(Json(Checkpoints { checkpoints }))
}

/// `GET /v1/audit/checkpoints`: the server chain's checkpoints, for
/// break-glass sessions.
pub async fn server_checkpoints(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<Checkpoints>> {
    authed.require_break_glass()?;
    let (since, _) = paging(&query);
    let checkpoints = audit::checkpoints(&state, SERVER_CHAIN, since)
        .await?
        .into_iter()
        .map(|(seq, checkpoint)| CheckpointEntry { seq, checkpoint })
        .collect();
    Ok(Json(Checkpoints { checkpoints }))
}

/// `POST /v1/orgs/{org}/audit/checkpoints/{seq}/cosign`
pub async fn cosign(
    State(state): State<AppState>,
    Path((org, seq)): Path<(String, u64)>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?.clone();
    let request: Cosign = authed.json()?;
    if request.signature.device != device.id {
        return Err(ApiError::forbidden("co-sign with this device"));
    }
    audit::cosign(
        &state,
        &org.to_string(),
        seq,
        request.signature,
        &device.signing_key.0,
    )
    .await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("checkpoint.cosign")
            .org(org)
            .detail(format!("checkpoint {seq}")),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paging_clamps_the_limit() {
        let mut query = HashMap::new();
        assert_eq!(paging(&query), (0, PAGE));
        query.insert("since".into(), "5".into());
        query.insert("limit".into(), "999999".into());
        assert_eq!(paging(&query), (5, PAGE));
        query.insert("limit".into(), "0".into());
        assert_eq!(paging(&query), (5, 1));
    }
}
