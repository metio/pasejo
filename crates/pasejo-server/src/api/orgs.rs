// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet, HashMap};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use pasejo_protocol::api::{
    Append, Appended, Cosign, CreateOrg, Enrollment, EnrollmentRequest, EnrollmentStatus,
    Enrollments, ErrorCode, LogPage, Me, OrgSummary, Proposal, Proposals, ScimTokenCreated,
    ScimTokenInfo, ScimTokens, ServerInfo, Users,
};
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::{Fixed, b64, random, sha256};
use pasejo_protocol::envelope::{Context, Envelope, verify};
use pasejo_protocol::log::{Body, Entry, Rejection, Role};
use pasejo_protocol::{Id, Millis};
use sqlx::Row;

use crate::audit::{self, Event};
use crate::auth::flows::emergency_until;
use crate::auth::{Authed, OrgAccess, revoke_sessions_of_key};
use crate::config::Creation;
use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, parse_id};
use crate::users;

/// `GET /v1/server`
pub async fn server(State(state): State<AppState>) -> ApiResult<Json<ServerInfo>> {
    Ok(Json(ServerInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        checkpoint_key: Fixed(state.keys.checkpoint.verifying_key().to_bytes()),
        oidc: state.oidc.is_some(),
        break_glass: state.config.break_glass.enabled,
        emergency_until: emergency_until(&state).await?,
    }))
}

/// The organizations `user` belongs to or asked to join.
async fn orgs_of(state: &AppState, user: Id) -> ApiResult<Vec<OrgSummary>> {
    let rows = sqlx::query("SELECT id, name FROM orgs ORDER BY created")
        .fetch_all(state.db.pool())
        .await?;
    let pending: BTreeSet<Id> = sqlx::query(
        state
            .db
            .sql("SELECT org FROM enrollments WHERE user_id = $1"),
    )
    .bind(user.to_string())
    .fetch_all(state.db.pool())
    .await?
    .iter()
    .map(|row| db::id(row, "org"))
    .collect::<anyhow::Result<_>>()?;
    let mut orgs = Vec::new();
    for row in rows {
        let org = db::id(&row, "id")?;
        let log = state.logs.get(&state.db, org).await?;
        if log.state().members.contains_key(&user) || pending.contains(&org) {
            orgs.push(OrgSummary {
                id: org,
                name: row.try_get("name")?,
            });
        }
    }
    Ok(orgs)
}

/// `GET /v1/me`
pub async fn me(State(state): State<AppState>, authed: Authed) -> ApiResult<Json<Me>> {
    let user = authed.user()?;
    let summary = users::summary(&state, user)
        .await?
        .ok_or_else(|| ApiError::unauthorized("unknown user"))?;
    Ok(Json(Me {
        user,
        name: summary.name,
        email: summary.email,
        machine: summary.machine,
        orgs: orgs_of(&state, user).await?,
    }))
}

/// `POST /v1/orgs`
pub async fn create(
    State(state): State<AppState>,
    authed: Authed,
) -> ApiResult<(StatusCode, Json<OrgSummary>)> {
    let user = authed.user()?;
    let request: CreateOrg = authed.json()?;
    let allowed = match state.config.orgs.creation {
        Creation::Anyone => true,
        Creation::Nobody => false,
        Creation::Listed => {
            let email = users::summary(&state, user)
                .await?
                .map_or_else(String::new, |summary| summary.email);
            !email.is_empty()
                && state
                    .config
                    .orgs
                    .creators
                    .iter()
                    .any(|creator| creator.eq_ignore_ascii_case(&email))
        }
    };
    if !allowed {
        audit::record(&state, Some(&authed.ctx), Event::denied("org.create")).await?;
        return Err(ApiError::forbidden("you may not create organizations"));
    }
    let entry: Entry = request.genesis.open()?;
    let Body::Genesis {
        founder, device, ..
    } = &entry.body
    else {
        return Err(ApiError::bad_request("the first entry must be a genesis"));
    };
    if *founder != user || device.signing_key != authed.session.signing_key {
        return Err(ApiError::forbidden(
            "the genesis must name your user and this device",
        ));
    }
    let log = state
        .logs
        .create(&state.db, request.genesis, state.now())
        .await?;
    let org = log.state().org;
    authed.ctx.set_device(Some(device.id));
    audit::record(&state, Some(&authed.ctx), Event::ok("org.create").org(org)).await?;
    Ok((
        StatusCode::CREATED,
        Json(OrgSummary {
            id: org,
            name: log.state().name.clone(),
        }),
    ))
}

async fn has_enrollment(state: &AppState, org: Id, user: Id) -> ApiResult<bool> {
    Ok(sqlx::query(
        state
            .db
            .sql("SELECT 1 AS present FROM enrollments WHERE org = $1 AND user_id = $2"),
    )
    .bind(org.to_string())
    .bind(user.to_string())
    .fetch_optional(state.db.pool())
    .await?
    .is_some())
}

/// `GET /v1/orgs/{org}/log`
pub async fn log(
    State(state): State<AppState>,
    Path(org): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<LogPage>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let allowed = access.device.is_some()
        || access.role.is_some()
        || match access.user {
            Some(user) => has_enrollment(&state, org, user).await?,
            None => false,
        };
    if !allowed {
        return Err(ApiError::forbidden("you are not part of this organization"));
    }
    let since: Option<u64> = query.get("since").and_then(|since| since.parse().ok());
    let entries = access.log.entries();
    let start = since.map_or(0, |since| usize::try_from(since + 1).unwrap_or(usize::MAX));
    Ok(Json(LogPage {
        entries: entries.get(start..).unwrap_or_default().to_vec(),
    }))
}

/// Applies what the server does beyond storing an accepted entry.
async fn after_append(state: &AppState, org: Id, entry: &Entry) -> ApiResult<()> {
    match &entry.body {
        Body::DeviceAdd { device } => {
            sqlx::query(
                state
                    .db
                    .sql("DELETE FROM enrollments WHERE org = $1 AND device = $2"),
            )
            .bind(org.to_string())
            .bind(device.id.to_string())
            .execute(state.db.pool())
            .await?;
        }
        Body::DeviceRevoke { device } => {
            let log = state.logs.get(&state.db, org).await?;
            if let Some(revoked) = log.state().devices.get(device) {
                revoke_sessions_of_key(state, &revoked.record.signing_key).await?;
            }
        }
        Body::MemberRole { user, role } => {
            sqlx::query(
                state
                    .db
                    .sql("DELETE FROM join_requests WHERE org = $1 AND user_id = $2"),
            )
            .bind(org.to_string())
            .bind(user.to_string())
            .execute(state.db.pool())
            .await?;
            if role.role().is_none() {
                let log = state.logs.get(&state.db, org).await?;
                for device in log
                    .state()
                    .devices
                    .values()
                    .filter(|device| device.record.user == *user)
                {
                    revoke_sessions_of_key(state, &device.record.signing_key).await?;
                }
            }
        }
        Body::StoreCreate { store } => {
            sqlx::query(state.db.sql("INSERT INTO stores (id, org, version, index_hash, lookup, created) VALUES ($1, $2, 0, NULL, NULL, $3)"))
                .bind(store.to_string())
                .bind(org.to_string())
                .bind(to_i64(entry.time))
                .execute(state.db.pool())
                .await?;
        }
        _ => {}
    }
    Ok(())
}

/// How far an entry's or index version's time may lie from the server's
/// clock.
///
/// The log checks grants and expiries at the time an entry names, so a
/// backdated entry could act on a grant that has already ended.
pub const MAX_ENTRY_SKEW: Millis = 5 * 60_000;

/// Refuses an entry whose time is far from the server's clock.
///
/// # Errors
///
/// Fails with `bad_request`.
pub fn require_fresh(time: Millis, now: Millis) -> ApiResult<()> {
    if time.abs_diff(now) > MAX_ENTRY_SKEW {
        return Err(ApiError::bad_request(
            "the entry's time is too far from the server's clock",
        ));
    }
    Ok(())
}

async fn append_entry(
    state: &AppState,
    authed: &Authed,
    org: Id,
    envelope: Envelope,
) -> ApiResult<Appended> {
    let entry: Entry = envelope.open()?;
    let kind = entry.body.kind();
    let store = entry.body.store();
    let result = state.logs.append(&state.db, org, envelope).await;
    let mut event = match &result {
        Ok(_) => Event::ok("log.append"),
        Err(error) => Event::denied("log.append").detail(error.message.clone()),
    }
    .org(org);
    event.detail = Some(
        event
            .detail
            .map_or_else(|| kind.to_owned(), |detail| format!("{kind}: {detail}")),
    );
    if let Some(store) = store {
        event = event.store(store);
    }
    audit::record(state, Some(&authed.ctx), event).await?;
    let (_, seq, head) = result?;
    after_append(state, org, &entry).await?;
    Ok(Appended { seq, head })
}

/// `POST /v1/orgs/{org}/log`
pub async fn append(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Appended>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?;
    let request: Append = authed.json()?;
    let entry: Entry = request.entry.open()?;
    require_fresh(entry.time, state.now())?;
    if entry.author != device.id {
        return Err(ApiError::forbidden(
            "entries must be authored by this device",
        ));
    }
    Ok(Json(
        append_entry(&state, &authed, org, request.entry).await?,
    ))
}

/// `POST /v1/orgs/{org}/log/proposals`
pub async fn propose(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<(StatusCode, Json<Proposal>)> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?;
    let request: Append = authed.json()?;
    let entry: Entry = request.entry.open()?;
    require_fresh(entry.time, state.now())?;
    if entry.author != device.id {
        return Err(ApiError::forbidden(
            "entries must be authored by this device",
        ));
    }
    let mut trial = (*access.log).clone();
    match trial.append(request.entry.clone()) {
        Err(Rejection::SecondAdminRequired) => {}
        Ok(_) => {
            return Err(ApiError::bad_request(
                "this entry needs no second admin; append it",
            ));
        }
        Err(rejection) => return Err(rejection.into()),
    }
    let proposal = Proposal {
        id: Id::random(),
        entry: request.entry,
        created: state.now(),
    };
    sqlx::query(
        state
            .db
            .sql("INSERT INTO proposals (id, org, envelope, created) VALUES ($1, $2, $3, $4)"),
    )
    .bind(proposal.id.to_string())
    .bind(org.to_string())
    .bind(serde_json::to_string(&proposal.entry)?)
    .bind(to_i64(proposal.created))
    .execute(state.db.pool())
    .await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("log.propose").org(org).detail(entry.body.kind()),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(proposal)))
}

async fn load_proposals(state: &AppState, org: Id) -> ApiResult<Vec<Proposal>> {
    let rows = sqlx::query(
        state
            .db
            .sql("SELECT id, envelope, created FROM proposals WHERE org = $1 ORDER BY created"),
    )
    .bind(org.to_string())
    .fetch_all(state.db.pool())
    .await?;
    rows.iter()
        .map(|row| {
            let text: String = row.try_get("envelope")?;
            Ok(Proposal {
                id: db::id(row, "id")?,
                entry: serde_json::from_str(&text)?,
                created: db::uint(row, "created")?,
            })
        })
        .collect()
}

/// `GET /v1/orgs/{org}/log/proposals`
pub async fn proposals(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Proposals>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.require(&[Role::Admin])?;
    Ok(Json(Proposals {
        proposals: load_proposals(&state, org).await?,
    }))
}

/// `POST /v1/orgs/{org}/log/proposals/{id}/cosign`
pub async fn cosign(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<Appended>> {
    let org = parse_id(&org)?;
    let id = parse_id(&id)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.require(&[Role::Admin])?.clone();
    let request: Cosign = authed.json()?;
    if request.signature.device != device.id {
        return Err(ApiError::forbidden("co-sign with this device"));
    }
    let proposal = load_proposals(&state, org)
        .await?
        .into_iter()
        .find(|proposal| proposal.id == id)
        .ok_or_else(|| ApiError::not_found("no such proposal"))?;
    let mut envelope = proposal.entry;
    verify(
        Context::LogEntry,
        &envelope.payload.0,
        &device.signing_key.0,
        &request.signature.signature.0,
    )?;
    envelope
        .signatures
        .retain(|signature| signature.device != device.id);
    envelope.signatures.push(request.signature);
    let mut trial = (*access.log).clone();
    match trial.append(envelope.clone()) {
        Err(Rejection::SecondAdminRequired) => {
            sqlx::query(
                state
                    .db
                    .sql("UPDATE proposals SET envelope = $1 WHERE id = $2"),
            )
            .bind(serde_json::to_string(&envelope)?)
            .bind(id.to_string())
            .execute(state.db.pool())
            .await?;
            Err(ApiError::new(
                ErrorCode::Rejected,
                "still needs another admin's signature",
            ))
        }
        Err(Rejection::WrongSequence | Rejection::WrongPredecessor) => {
            sqlx::query(state.db.sql("DELETE FROM proposals WHERE id = $1"))
                .bind(id.to_string())
                .execute(state.db.pool())
                .await?;
            Err(ApiError::new(
                ErrorCode::Conflict,
                "the log moved on; propose the entry again",
            ))
        }
        _ => {
            let appended = append_entry(&state, &authed, org, envelope).await?;
            sqlx::query(state.db.sql("DELETE FROM proposals WHERE id = $1"))
                .bind(id.to_string())
                .execute(state.db.pool())
                .await?;
            Ok(Json(appended))
        }
    }
}

/// Pending enrollments of `org`: requests whose device is not in the log,
/// from active users SCIM has not deactivated.
///
/// # Errors
///
/// Fails when the database does.
pub async fn pending_enrollments(state: &AppState, org: Id) -> ApiResult<Vec<Enrollment>> {
    let log = state.logs.get(&state.db, org).await?;
    let rows = sqlx::query(state.db.sql("SELECT e.device, e.user_id, e.kind, e.recipient, e.signing_key, e.label, e.requested, u.name, u.email, u.active FROM enrollments e JOIN users u ON u.id = e.user_id WHERE e.org = $1 ORDER BY e.requested"))
        .bind(org.to_string())
        .fetch_all(state.db.pool())
        .await?;
    let mut enrollments = Vec::new();
    for row in rows {
        let device = db::id(&row, "device")?;
        let user = db::id(&row, "user_id")?;
        if log.state().devices.contains_key(&device)
            || db::uint(&row, "active")? == 0
            || crate::auth::scim_suspended(state, org, user).await?
        {
            continue;
        }
        let kind: String = row.try_get("kind")?;
        let record = DeviceRecord {
            id: device,
            user,
            kind: if kind == "hpke" {
                DeviceKind::Hpke
            } else {
                DeviceKind::Age
            },
            recipient: row.try_get("recipient")?,
            signing_key: Fixed(db::bytes32(&row, "signing_key")?),
            label: row.try_get("label")?,
        };
        enrollments.push(Enrollment {
            fingerprint: record.fingerprint(),
            device: record,
            requested: db::uint(&row, "requested")?,
            user_name: row.try_get("name")?,
            user_email: row.try_get("email")?,
        });
    }
    Ok(enrollments)
}

/// `POST /v1/orgs/{org}/enrollments`
pub async fn enroll(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<(StatusCode, Json<Enrollment>)> {
    let org = parse_id(&org)?;
    let user = authed.user()?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    if access.device.is_some() {
        return Err(ApiError::bad_request("this device is already active"));
    }
    if crate::auth::scim_suspended(&state, org, user).await? {
        return Err(ApiError::forbidden(
            "your account is deactivated in this organization",
        ));
    }
    let request: EnrollmentRequest = authed.json()?;
    let record = DeviceRecord {
        id: request.id,
        user,
        kind: request.kind,
        recipient: request.recipient,
        signing_key: authed.session.signing_key,
        label: request.label,
    };
    record.validate()?;
    if access.log.state().devices.contains_key(&record.id) {
        return Err(ApiError::new(
            ErrorCode::Conflict,
            "the device identifier is taken",
        ));
    }
    let now = state.now();
    let inserted = sqlx::query(state.db.sql("INSERT INTO enrollments (org, device, user_id, kind, recipient, signing_key, label, requested) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"))
        .bind(org.to_string())
        .bind(record.id.to_string())
        .bind(user.to_string())
        .bind(record.kind.label())
        .bind(record.recipient.clone())
        .bind(record.signing_key.0.to_vec())
        .bind(record.label.clone())
        .bind(to_i64(now))
        .execute(state.db.pool())
        .await;
    match inserted {
        Ok(_) => {}
        Err(error) if crate::error::is_unique_violation(&error) => {
            return Err(ApiError::new(
                ErrorCode::Conflict,
                "the device identifier is taken",
            ));
        }
        Err(error) => return Err(error.into()),
    }
    if access.log.state().settings.auto_join && !access.log.state().members.contains_key(&user) {
        let _ = sqlx::query(
            state
                .db
                .sql("INSERT INTO join_requests (org, user_id, created) VALUES ($1, $2, $3)"),
        )
        .bind(org.to_string())
        .bind(user.to_string())
        .bind(to_i64(now))
        .execute(state.db.pool())
        .await;
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("device.enroll")
            .org(org)
            .detail(record.id.to_string()),
    )
    .await?;
    let summary = users::summary(&state, user)
        .await?
        .ok_or_else(ApiError::internal)?;
    Ok((
        StatusCode::CREATED,
        Json(Enrollment {
            fingerprint: record.fingerprint(),
            device: record,
            requested: now,
            user_name: summary.name,
            user_email: summary.email,
        }),
    ))
}

/// `GET /v1/orgs/{org}/enrollments`
pub async fn enrollments(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Enrollments>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let all = pending_enrollments(&state, org).await?;
    let enrollments = if access.has_role(&[Role::Admin, Role::Approver]) {
        all
    } else {
        let user = authed.user()?;
        all.into_iter()
            .filter(|enrollment| enrollment.device.user == user)
            .collect()
    };
    Ok(Json(Enrollments { enrollments }))
}

/// `GET /v1/orgs/{org}/enrollments/{device}`
pub async fn enrollment_status(
    State(state): State<AppState>,
    Path((org, device)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<EnrollmentStatus>> {
    let org = parse_id(&org)?;
    let device = parse_id(&device)?;
    let user = authed.user()?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let active = access
        .log
        .state()
        .active_device(device)
        .is_some_and(|record| record.user == user);
    let pending = pending_enrollments(&state, org)
        .await?
        .iter()
        .any(|enrollment| enrollment.device.id == device && enrollment.device.user == user);
    if !active && !pending {
        return Err(ApiError::not_found("no such enrollment"));
    }
    Ok(Json(EnrollmentStatus { pending, active }))
}

/// `GET /v1/orgs/{org}/users`
pub async fn org_users(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Users>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let mut ids: BTreeSet<Id> = access.log.state().members.keys().copied().collect();
    for enrollment in pending_enrollments(&state, org).await? {
        ids.insert(enrollment.device.user);
    }
    for table in [
        "SELECT user_id FROM join_requests WHERE org = $1",
        "SELECT user_id FROM scim_users WHERE org = $1 AND deleted = 0",
    ] {
        for row in sqlx::query(state.db.sql(table))
            .bind(org.to_string())
            .fetch_all(state.db.pool())
            .await?
        {
            ids.insert(db::id(&row, "user_id")?);
        }
    }
    Ok(Json(Users {
        users: users::summaries(&state, ids).await?,
    }))
}

/// `POST /v1/orgs/{org}/scim-tokens`
pub async fn create_scim_token(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<(StatusCode, Json<ScimTokenCreated>)> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.require(&[Role::Admin])?;
    let id = Id::random();
    let token = format!("pasejo-scim-{}", b64(&random::<32>()));
    sqlx::query(state.db.sql(
        "INSERT INTO scim_tokens (id, org, hash, created, revoked) VALUES ($1, $2, $3, $4, 0)",
    ))
    .bind(id.to_string())
    .bind(org.to_string())
    .bind(sha256(token.as_bytes()).to_vec())
    .bind(to_i64(state.now()))
    .execute(state.db.pool())
    .await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("scim.token.create")
            .org(org)
            .detail(id.to_string()),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(ScimTokenCreated { id, token })))
}

/// `GET /v1/orgs/{org}/scim-tokens`
pub async fn list_scim_tokens(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<ScimTokens>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.require(&[Role::Admin])?;
    let rows = sqlx::query(state.db.sql(
        "SELECT id, created FROM scim_tokens WHERE org = $1 AND revoked = 0 ORDER BY created",
    ))
    .bind(org.to_string())
    .fetch_all(state.db.pool())
    .await?;
    let tokens = rows
        .iter()
        .map(|row| {
            Ok(ScimTokenInfo {
                id: db::id(row, "id")?,
                created: db::uint(row, "created")?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(Json(ScimTokens { tokens }))
}

/// `DELETE /v1/orgs/{org}/scim-tokens/{id}`
pub async fn delete_scim_token(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    let org = parse_id(&org)?;
    let id = parse_id(&id)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.require(&[Role::Admin])?;
    let revoked = sqlx::query(
        state
            .db
            .sql("UPDATE scim_tokens SET revoked = 1 WHERE id = $1 AND org = $2 AND revoked = 0"),
    )
    .bind(id.to_string())
    .bind(org.to_string())
    .execute(state.db.pool())
    .await?
    .rows_affected();
    if revoked == 0 {
        return Err(ApiError::not_found("no such token"));
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("scim.token.revoke")
            .org(org)
            .detail(id.to_string()),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Users who asked to join `org` and are not members yet.
///
/// # Errors
///
/// Fails when the database does.
pub async fn join_requests(state: &AppState, org: Id) -> ApiResult<BTreeMap<Id, Millis>> {
    let rows = sqlx::query(
        state
            .db
            .sql("SELECT user_id, created FROM join_requests WHERE org = $1"),
    )
    .bind(org.to_string())
    .fetch_all(state.db.pool())
    .await?;
    rows.iter()
        .map(|row| Ok((db::id(row, "user_id")?, db::uint(row, "created")?)))
        .collect()
}
