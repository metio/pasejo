// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use pasejo_protocol::api::{
    ChainLink, Commit, Committed, EpochUpload, ErrorCode, IndexResponse, Keys, LookupResponse,
    ObjectsRequest, ObjectsResponse, OfflineReadsRequest, RecoveryExport, RecoveryStore, Rotation,
    RotationItem, StoreSummary, Stores, Wrapping, WrappingUpload, WrappingsUpload,
};
use pasejo_protocol::encoding::Bytes;
use pasejo_protocol::envelope::Envelope;
use pasejo_protocol::log::{OrgLog, Recipients, Role, StoreRole};
use pasejo_protocol::store::{IndexPayload, SealedObject};
use pasejo_protocol::{Hash, Id, Millis};
use sqlx::Row;

use crate::audit::{self, Event};
use crate::auth::{Authed, OrgAccess, SessionKind};
use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, parse_id};

/// Object versions kept per identifier.
pub const HISTORY: u64 = 10;
/// Largest accepted object ciphertext.
pub const MAX_OBJECT: usize = 1 << 20;

fn unknown_store() -> ApiError {
    ApiError::new(ErrorCode::UnknownStore, "unknown store")
}

/// The devices entitled to `store` now, or `unknown_store`.
fn entitled(access: &OrgAccess, store: Id, now: Millis) -> ApiResult<Recipients> {
    match access.log.state().stores.get(&store) {
        Some(state) if !state.deleted => Ok(access.log.state().entitled(store, now)),
        _ => Err(unknown_store()),
    }
}

/// Requires this device to hold `store`'s keys now: full keys, or at least
/// the names key.
fn require_keys(access: &OrgAccess, store: Id, now: Millis, full: bool) -> ApiResult<Id> {
    let device = access.device()?.id;
    let recipients = entitled(access, store, now)?;
    let allowed =
        recipients.full.contains(&device) || (!full && recipients.names_only.contains(&device));
    if allowed {
        Ok(device)
    } else {
        Err(ApiError::forbidden("this device may not read this store"))
    }
}

struct StoreRow {
    version: u64,
    index_hash: Option<Hash>,
}

async fn store_row(state: &AppState, org: Id, store: Id) -> ApiResult<StoreRow> {
    let row = sqlx::query(
        state
            .db
            .sql("SELECT version, index_hash FROM stores WHERE id = $1 AND org = $2"),
    )
    .bind(store.to_string())
    .bind(org.to_string())
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(unknown_store)?;
    let hash: Option<Vec<u8>> = row.try_get("index_hash")?;
    Ok(StoreRow {
        version: db::uint(&row, "version")?,
        index_hash: hash
            .map(|hash| hash.try_into().map(Hash))
            .transpose()
            .map_err(|_| ApiError::internal())?,
    })
}

/// `GET /v1/orgs/{org}/stores`
pub async fn list(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Stores>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?.id;
    let now = state.now();
    let overseer = access.has_role(&[Role::Admin, Role::Auditor]);
    let mut stores = Vec::new();
    for (id, store) in &access.log.state().stores {
        if store.deleted {
            continue;
        }
        let recipients = access.log.state().entitled(*id, now);
        if !(overseer
            || recipients.full.contains(&device)
            || recipients.names_only.contains(&device))
        {
            continue;
        }
        stores.push(StoreSummary {
            id: *id,
            epoch: store.current_epoch(),
            version: store_row(&state, org, *id).await?.version,
            role: access
                .user
                .and_then(|user| access.log.state().store_role(*id, user, now)),
        });
    }
    Ok(Json(Stores { stores }))
}

/// `GET /v1/orgs/{org}/stores/{store}/keys`
pub async fn keys(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<Keys>> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = require_keys(&access, store, state.now(), false)?;
    let wrappings = sqlx::query(state.db.sql("SELECT epoch, full_keys, ciphertext FROM wrappings WHERE store = $1 AND device = $2 ORDER BY epoch"))
        .bind(store.to_string())
        .bind(device.to_string())
        .fetch_all(state.db.pool())
        .await?
        .iter()
        .map(|row| {
            Ok(Wrapping {
                epoch: db::uint(row, "epoch")?,
                device,
                full: db::uint(row, "full_keys")? != 0,
                ciphertext: Bytes(row.try_get("ciphertext")?),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let chain = sqlx::query(state.db.sql(
        "SELECT epoch, prev_names, prev_data FROM chain_links WHERE store = $1 ORDER BY epoch",
    ))
    .bind(store.to_string())
    .fetch_all(state.db.pool())
    .await?
    .iter()
    .map(|row| {
        Ok(ChainLink {
            epoch: db::uint(row, "epoch")?,
            prev_names: Bytes(row.try_get("prev_names")?),
            prev_data: Bytes(row.try_get("prev_data")?),
        })
    })
    .collect::<ApiResult<Vec<_>>>()?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("keys.read").org(org).store(store),
    )
    .await?;
    Ok(Json(Keys { wrappings, chain }))
}

/// `GET /v1/orgs/{org}/recovery`: what the offline recovery identity needs
/// to decrypt every store.
///
/// Only admins may export, and every export is audited; the wrappings open
/// only with the offline identity.
pub async fn recovery(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<RecoveryExport>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    if !access.has_role(&[Role::Admin]) {
        audit::record(
            &state,
            Some(&authed.ctx),
            Event::denied("recovery.export").org(org),
        )
        .await?;
        return Err(ApiError::forbidden("only admins export for recovery"));
    }
    let Some(recipient) = access.log.state().recovery.clone() else {
        return Err(ApiError::new(
            ErrorCode::NotFound,
            "the organization has no recovery recipient",
        ));
    };
    let device = pasejo_protocol::device::recovery_id(&recipient);
    let mut stores = Vec::new();
    for (id, store) in &access.log.state().stores {
        if store.deleted {
            continue;
        }
        let wrappings = sqlx::query(state.db.sql("SELECT epoch, full_keys, ciphertext FROM wrappings WHERE store = $1 AND device = $2 ORDER BY epoch"))
            .bind(id.to_string())
            .bind(device.to_string())
            .fetch_all(state.db.pool())
            .await?
            .iter()
            .map(|row| {
                Ok(Wrapping {
                    epoch: db::uint(row, "epoch")?,
                    device,
                    full: db::uint(row, "full_keys")? != 0,
                    ciphertext: Bytes(row.try_get("ciphertext")?),
                })
            })
            .collect::<ApiResult<Vec<_>>>()?;
        let chain = sqlx::query(state.db.sql(
            "SELECT epoch, prev_names, prev_data FROM chain_links WHERE store = $1 ORDER BY epoch",
        ))
        .bind(id.to_string())
        .fetch_all(state.db.pool())
        .await?
        .iter()
        .map(|row| {
            Ok(ChainLink {
                epoch: db::uint(row, "epoch")?,
                prev_names: Bytes(row.try_get("prev_names")?),
                prev_data: Bytes(row.try_get("prev_data")?),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
        let version = store_row(&state, org, *id).await?.version;
        let objects = sqlx::query(state.db.sql(
            "SELECT o.id, o.version, o.epoch, o.ciphertext FROM objects o WHERE o.store = $1 AND o.version = (SELECT MAX(l.version) FROM objects l WHERE l.store = o.store AND l.id = o.id)",
        ))
        .bind(id.to_string())
        .fetch_all(state.db.pool())
        .await?
        .iter()
        .map(|row| {
            let object: String = row.try_get("id")?;
            Ok(SealedObject {
                id: parse_id(&object)?,
                version: db::uint(row, "version")?,
                epoch: db::uint(row, "epoch")?,
                ciphertext: Bytes(row.try_get("ciphertext")?),
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
        stores.push(RecoveryStore {
            id: *id,
            keys: Keys { wrappings, chain },
            index: current_index(&state, *id, version).await?,
            objects,
        });
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("recovery.export")
            .org(org)
            .detail(format!("{} stores", stores.len())),
    )
    .await?;
    Ok(Json(RecoveryExport { stores }))
}

/// Checks that each wrapping goes to a device entitled to what it carries.
fn check_targets(recipients: &Recipients, wrappings: &[WrappingUpload]) -> ApiResult<()> {
    let mut seen = BTreeSet::new();
    for wrapping in wrappings {
        if !seen.insert(wrapping.device) {
            return Err(ApiError::bad_request("a device appears twice"));
        }
        let allowed = recipients.full.contains(&wrapping.device)
            || (!wrapping.full && recipients.names_only.contains(&wrapping.device));
        if !allowed {
            return Err(ApiError::forbidden(
                "a wrapping goes to a device not entitled to it",
            ));
        }
        if wrapping.ciphertext.0.is_empty() || wrapping.ciphertext.0.len() > 64 * 1024 {
            return Err(ApiError::bad_request("a wrapping has an invalid size"));
        }
    }
    Ok(())
}

async fn store_wrappings(
    state: &AppState,
    store: Id,
    epoch: u64,
    uploader: Id,
    replace: bool,
    wrappings: &[WrappingUpload],
) -> ApiResult<usize> {
    let db = &state.db;
    let mut transaction = db.begin().await?;
    let mut stored = 0;
    for wrapping in wrappings {
        let exists = sqlx::query(db.sql(
            "SELECT 1 AS present FROM wrappings WHERE store = $1 AND epoch = $2 AND device = $3",
        ))
        .bind(store.to_string())
        .bind(to_i64(epoch))
        .bind(wrapping.device.to_string())
        .fetch_optional(&mut *transaction)
        .await?
        .is_some();
        if exists {
            if !replace {
                continue;
            }
            sqlx::query(
                db.sql("DELETE FROM wrappings WHERE store = $1 AND epoch = $2 AND device = $3"),
            )
            .bind(store.to_string())
            .bind(to_i64(epoch))
            .bind(wrapping.device.to_string())
            .execute(&mut *transaction)
            .await?;
        }
        sqlx::query(db.sql("INSERT INTO wrappings (store, epoch, device, full_keys, ciphertext, uploaded_by) VALUES ($1, $2, $3, $4, $5, $6)"))
            .bind(store.to_string())
            .bind(to_i64(epoch))
            .bind(wrapping.device.to_string())
            .bind(i64::from(wrapping.full))
            .bind(wrapping.ciphertext.0.clone())
            .bind(uploader.to_string())
            .execute(&mut *transaction)
            .await?;
        stored += 1;
    }
    transaction.commit().await?;
    Ok(stored)
}

fn store_admin(access: &OrgAccess, store: Id, now: Millis) -> bool {
    access.user.is_some_and(|user| {
        access.log.state().store_role(store, user, now) == Some(StoreRole::Admin)
    })
}

/// `PUT /v1/orgs/{org}/stores/{store}/epochs/{epoch}`
pub async fn upload_epoch(
    State(state): State<AppState>,
    Path((org, store, epoch)): Path<(String, String, u64)>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?.id;
    let now = state.now();
    let recipients = entitled(&access, store, now)?;
    if !store_admin(&access, store, now) {
        return Err(ApiError::forbidden("only store admins upload epochs"));
    }
    if access.log.epoch(store, epoch).is_none() {
        return Err(ApiError::not_found("the log has no such epoch"));
    }
    let request: EpochUpload = authed.json()?;
    check_targets(&recipients, &request.wrappings)?;
    match (&request.chain, epoch) {
        (None, 0) => {}
        (Some(chain), _) if epoch > 0 && chain.epoch == epoch => {
            let existing = sqlx::query(state.db.sql(
                "SELECT prev_names, prev_data FROM chain_links WHERE store = $1 AND epoch = $2",
            ))
            .bind(store.to_string())
            .bind(to_i64(epoch))
            .fetch_optional(state.db.pool())
            .await?;
            match existing {
                Some(row) => {
                    let names: Vec<u8> = row.try_get("prev_names")?;
                    let data: Vec<u8> = row.try_get("prev_data")?;
                    if names != chain.prev_names.0 || data != chain.prev_data.0 {
                        return Err(ApiError::new(
                            ErrorCode::Conflict,
                            "the epoch's chain is already stored",
                        ));
                    }
                }
                None => {
                    sqlx::query(state.db.sql("INSERT INTO chain_links (store, epoch, prev_names, prev_data) VALUES ($1, $2, $3, $4)"))
                        .bind(store.to_string())
                        .bind(to_i64(epoch))
                        .bind(chain.prev_names.0.clone())
                        .bind(chain.prev_data.0.clone())
                        .execute(state.db.pool())
                        .await?;
                }
            }
        }
        _ => {
            return Err(ApiError::bad_request(
                "epochs after the first need exactly their chain link",
            ));
        }
    }
    let stored = store_wrappings(&state, store, epoch, device, true, &request.wrappings).await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("epoch.upload")
            .org(org)
            .store(store)
            .detail(format!("epoch {epoch}, {stored} wrappings")),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /v1/orgs/{org}/stores/{store}/wrappings`
pub async fn upload_wrappings(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    let now = state.now();
    let request: WrappingsUpload = authed.json()?;
    let carries_data = request.wrappings.iter().any(|wrapping| wrapping.full);
    let device = require_keys(&access, store, now, carries_data)?;
    let current = access
        .log
        .state()
        .stores
        .get(&store)
        .and_then(pasejo_protocol::log::StoreState::current_epoch);
    if current != Some(request.epoch) {
        return Err(ApiError::bad_request("wrappings go to the current epoch"));
    }
    check_targets(&entitled(&access, store, now)?, &request.wrappings)?;
    // Only a device holding the epoch's keys can wrap them; anything else
    // would hand others a wrapping that fails their commitment check.
    let held =
        sqlx::query(state.db.sql(
            "SELECT full_keys FROM wrappings WHERE store = $1 AND epoch = $2 AND device = $3",
        ))
        .bind(store.to_string())
        .bind(to_i64(request.epoch))
        .bind(device.to_string())
        .fetch_optional(state.db.pool())
        .await?
        .map(|row| db::uint(&row, "full_keys"))
        .transpose()?;
    match held {
        Some(full) if full != 0 || !carries_data => {}
        _ => {
            return Err(ApiError::forbidden(
                "this device holds no wrapping of these keys",
            ));
        }
    }
    let stored = store_wrappings(
        &state,
        store,
        request.epoch,
        device,
        store_admin(&access, store, now),
        &request.wrappings,
    )
    .await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("wrappings.upload")
            .org(org)
            .store(store)
            .detail(format!("epoch {}, {stored} wrappings", request.epoch)),
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn current_index(state: &AppState, store: Id, version: u64) -> ApiResult<Option<Envelope>> {
    if version == 0 {
        return Ok(None);
    }
    let row = sqlx::query(
        state
            .db
            .sql("SELECT envelope FROM index_versions WHERE store = $1 AND version = $2"),
    )
    .bind(store.to_string())
    .bind(to_i64(version))
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(ApiError::internal)?;
    let text: String = row.try_get("envelope")?;
    Ok(Some(serde_json::from_str(&text)?))
}

/// `GET /v1/orgs/{org}/stores/{store}/index`
pub async fn index(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    headers: HeaderMap,
    authed: Authed,
) -> ApiResult<Response> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    require_keys(&access, store, state.now(), false)?;
    let row = store_row(&state, org, store).await?;
    let etag = format!("\"{}\"", row.version);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == etag)
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    let envelope = current_index(&state, store, row.version).await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("index.read")
            .org(org)
            .store(store)
            .detail(format!("version {}", row.version)),
    )
    .await?;
    let mut response = Json(IndexResponse { index: envelope }).into_response();
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    Ok(response)
}

/// `GET /v1/orgs/{org}/stores/{store}/lookup`
pub async fn lookup(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<LookupResponse>> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    require_keys(&access, store, state.now(), true)?;
    let row = sqlx::query(
        state
            .db
            .sql("SELECT version, lookup FROM stores WHERE id = $1 AND org = $2"),
    )
    .bind(store.to_string())
    .bind(org.to_string())
    .fetch_optional(state.db.pool())
    .await?
    .ok_or_else(unknown_store)?;
    let version = db::uint(&row, "version")?;
    let lookup: Option<Vec<u8>> = row.try_get("lookup")?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("lookup.read")
            .org(org)
            .store(store)
            .detail(format!("version {version}")),
    )
    .await?;
    Ok(Json(LookupResponse {
        version,
        lookup: lookup.map(Bytes),
    }))
}

/// `POST /v1/orgs/{org}/stores/{store}/objects`
pub async fn objects(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<ObjectsResponse>> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    require_keys(&access, store, state.now(), true)?;
    let request: ObjectsRequest = authed.json()?;
    if request.objects.len() > 10_000 {
        return Err(ApiError::bad_request("too many objects in one request"));
    }
    let tenant = match &request.tenant {
        None => None,
        Some(_) if authed.session.kind != SessionKind::Machine => {
            return Err(ApiError::bad_request(
                "only machine sessions read on behalf of a tenant",
            ));
        }
        Some(label) if !pasejo_protocol::api::valid_tenant(label) => {
            return Err(ApiError::bad_request("malformed tenant label"));
        }
        Some(label) => Some(format!("tenant {label}")),
    };
    let mut objects = Vec::with_capacity(request.objects.len());
    for reference in request.objects {
        let row = sqlx::query(state.db.sql(
            "SELECT epoch, ciphertext FROM objects WHERE store = $1 AND id = $2 AND version = $3",
        ))
        .bind(store.to_string())
        .bind(reference.id.to_string())
        .bind(to_i64(reference.version))
        .fetch_optional(state.db.pool())
        .await?;
        let Some(row) = row else {
            audit::record(
                &state,
                Some(&authed.ctx),
                Event::denied("object.read")
                    .org(org)
                    .store(store)
                    .object(reference.id, reference.version)
                    .detail(tenant.as_ref().map_or_else(
                        || "no such version".to_owned(),
                        |tenant| format!("no such version; {tenant}"),
                    )),
            )
            .await?;
            return Err(ApiError::not_found("no such object version"));
        };
        let mut event = Event::ok("object.read")
            .org(org)
            .store(store)
            .object(reference.id, reference.version);
        if let Some(tenant) = &tenant {
            event = event.detail(tenant.clone());
        }
        audit::record(&state, Some(&authed.ctx), event).await?;
        state.metrics.object_reads.fetch_add(1, Ordering::Relaxed);
        objects.push(SealedObject {
            id: reference.id,
            version: reference.version,
            epoch: db::uint(&row, "epoch")?,
            ciphertext: Bytes(row.try_get("ciphertext")?),
        });
    }
    Ok(Json(ObjectsResponse { objects }))
}

/// `POST /v1/orgs/{org}/stores/{store}/offline-reads`: records the reads a
/// device made from its offline copy.
///
/// Every read must fall in a period in
/// which the membership log allowed offline reads; the time is the device's
/// own and the server cannot check it further. Any active device may report,
/// including one that has since lost access to the store, because a report
/// only ever names its own device.
pub async fn offline_reads(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<StatusCode> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let request: OfflineReadsRequest = authed.json()?;
    if request.reads.len() > 10_000 {
        return Err(ApiError::bad_request("too many reads in one request"));
    }
    store_row(&state, org, store).await?;
    let latest = state.now() + pasejo_protocol::request::MAX_SKEW_MILLIS;
    for read in &request.reads {
        if read.time > latest {
            return Err(ApiError::bad_request("a reported read lies in the future"));
        }
        if !access.log.offline_reads_allowed_at(read.time) {
            return Err(ApiError::forbidden(
                "the organization did not allow offline reads at that time",
            ));
        }
        sqlx::query(
            state
                .db
                .sql("SELECT 1 FROM objects WHERE store = $1 AND id = $2 AND version = $3"),
        )
        .bind(store.to_string())
        .bind(read.id.to_string())
        .bind(to_i64(read.version))
        .fetch_optional(state.db.pool())
        .await?
        .ok_or_else(|| ApiError::not_found("no such object version"))?;
    }
    for read in request.reads {
        audit::record(
            &state,
            Some(&authed.ctx),
            Event::ok("object.read.offline")
                .org(org)
                .store(store)
                .object(read.id, read.version)
                .detail(format!("read at {}", crate::time::rfc3339(read.time))),
        )
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Checks a commit against the log and the stored store, before anything
/// is written.
fn check_commit(
    access: &OrgAccess,
    store: Id,
    device: Id,
    stored: &StoreRow,
    commit: &Commit,
    now: Millis,
) -> ApiResult<IndexPayload> {
    let user = access
        .user
        .ok_or_else(|| ApiError::forbidden("this session has no user"))?;
    match access.log.state().store_role(store, user, now) {
        Some(StoreRole::Writer | StoreRole::Admin) => {}
        _ => return Err(ApiError::forbidden("you may not write this store")),
    }
    if commit
        .index
        .signatures
        .first()
        .map(|signature| signature.device)
        != Some(device)
    {
        return Err(ApiError::forbidden(
            "the index must be signed by this device",
        ));
    }
    let payload = access.log.check_index(&commit.index)?;
    crate::api::orgs::require_fresh(payload.time, now)?;
    if payload.store != store {
        return Err(ApiError::bad_request("the index names another store"));
    }
    let current_epoch = access
        .log
        .state()
        .stores
        .get(&store)
        .and_then(pasejo_protocol::log::StoreState::current_epoch);
    if Some(payload.epoch) != current_epoch {
        return Err(ApiError::bad_request(
            "the index must use the current epoch",
        ));
    }
    if payload.version != stored.version + 1 || payload.prev != stored.index_hash {
        return Err(ApiError::conflict(stored.version));
    }
    let changed: BTreeSet<Id> = payload.changed.iter().copied().collect();
    let uploaded: BTreeSet<Id> = commit.objects.iter().map(|object| object.id).collect();
    if changed.len() != payload.changed.len()
        || uploaded.len() != commit.objects.len()
        || changed != uploaded
    {
        return Err(ApiError::bad_request(
            "changed identifiers must match the uploaded objects",
        ));
    }
    let deleted: BTreeSet<Id> = payload.deleted.iter().copied().collect();
    if !deleted.is_disjoint(&changed) {
        return Err(ApiError::bad_request(
            "an identifier cannot change and be deleted",
        ));
    }
    for object in &commit.objects {
        if object.epoch != payload.epoch {
            return Err(ApiError::bad_request("objects must use the index's epoch"));
        }
        if object.ciphertext.0.is_empty() || object.ciphertext.0.len() > MAX_OBJECT {
            return Err(ApiError::bad_request("an object has an invalid size"));
        }
    }
    match (&payload.lookup_hash, &commit.lookup) {
        (Some(hash), Some(lookup)) if Hash::of(&lookup.0) == *hash => {}
        (None, None) => {}
        _ => {
            return Err(ApiError::bad_request(
                "the lookup object does not match its hash",
            ));
        }
    }
    Ok(payload)
}

enum Written {
    Done,
    Moved,
    ObjectVersions,
}

/// Writes a checked commit in one transaction. The store row is updated
/// first: on SQLite that takes the write lock before anything is read, and
/// on PostgreSQL it locks the row, so concurrent commits to one version
/// serialize and all but one find the version moved.
async fn write_commit(
    state: &AppState,
    store: Id,
    device: Id,
    stored_version: u64,
    request: &Commit,
    payload: &IndexPayload,
    now: Millis,
) -> Result<Written, sqlx::Error> {
    let db = &state.db;
    let mut transaction = db.begin().await?;
    let moved = sqlx::query(db.sql("UPDATE stores SET version = $1, index_hash = $2, lookup = $3 WHERE id = $4 AND version = $5"))
        .bind(to_i64(payload.version))
        .bind(request.index.hash().0.to_vec())
        .bind(request.lookup.as_ref().map(|lookup| lookup.0.clone()))
        .bind(store.to_string())
        .bind(to_i64(stored_version))
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    if moved == 0 {
        return Ok(Written::Moved);
    }
    for object in &request.objects {
        let latest = sqlx::query(
            db.sql("SELECT MAX(version) AS version FROM objects WHERE store = $1 AND id = $2"),
        )
        .bind(store.to_string())
        .bind(object.id.to_string())
        .fetch_one(&mut *transaction)
        .await?
        .try_get::<Option<i64>, _>("version")?
        .map_or(0, db::to_u64);
        if object.version <= latest {
            return Ok(Written::ObjectVersions);
        }
        sqlx::query(db.sql("INSERT INTO objects (store, id, version, epoch, ciphertext, created) VALUES ($1, $2, $3, $4, $5, $6)"))
            .bind(store.to_string())
            .bind(object.id.to_string())
            .bind(to_i64(object.version))
            .bind(to_i64(object.epoch))
            .bind(object.ciphertext.0.clone())
            .bind(to_i64(now))
            .execute(&mut *transaction)
            .await?;
        sqlx::query(db.sql("DELETE FROM objects WHERE store = $1 AND id = $2 AND version <= $3"))
            .bind(store.to_string())
            .bind(object.id.to_string())
            .bind(to_i64(object.version.saturating_sub(HISTORY)))
            .execute(&mut *transaction)
            .await?;
    }
    sqlx::query(db.sql("INSERT INTO index_versions (store, version, envelope, time, author) VALUES ($1, $2, $3, $4, $5)"))
        .bind(store.to_string())
        .bind(to_i64(payload.version))
        .bind(serde_json::to_string(&request.index).unwrap_or_default())
        .bind(to_i64(now))
        .bind(device.to_string())
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(Written::Done)
}

/// `POST /v1/orgs/{org}/stores/{store}/commits`
#[allow(clippy::too_many_lines)]
pub async fn commit(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<Committed>> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let access = OrgAccess::load(&state, &authed, org).await?;
    let device = access.device()?.id;
    let now = state.now();
    entitled(&access, store, now)?;
    let request: Commit = authed.json()?;
    let stored = store_row(&state, org, store).await?;
    let payload = match check_commit(&access, store, device, &stored, &request, now) {
        Ok(payload) => payload,
        Err(error) => {
            audit::record(
                &state,
                Some(&authed.ctx),
                Event::denied("commit")
                    .org(org)
                    .store(store)
                    .detail(error.message.clone()),
            )
            .await?;
            return Err(error);
        }
    };
    match write_commit(
        &state,
        store,
        device,
        stored.version,
        &request,
        &payload,
        now,
    )
    .await
    {
        Ok(Written::Done) => {}
        Ok(Written::ObjectVersions) => {
            return Err(ApiError::bad_request("object versions must grow"));
        }
        Ok(Written::Moved) => {
            return Err(ApiError::conflict(
                store_row(&state, org, store).await?.version,
            ));
        }
        Err(error) if crate::error::is_unique_violation(&error) => {
            return Err(ApiError::conflict(
                store_row(&state, org, store).await?.version,
            ));
        }
        Err(error) => return Err(error.into()),
    }
    state.metrics.commits.fetch_add(1, Ordering::Relaxed);
    for object in &request.objects {
        audit::record(
            &state,
            Some(&authed.ctx),
            Event::ok("object.write")
                .org(org)
                .store(store)
                .object(object.id, object.version),
        )
        .await?;
    }
    for id in &payload.deleted {
        let mut event = Event::ok("object.delete").org(org).store(store);
        event.object = Some(*id);
        audit::record(&state, Some(&authed.ctx), event).await?;
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("commit")
            .org(org)
            .store(store)
            .detail(format!("version {}", payload.version)),
    )
    .await?;
    Ok(Json(Committed {
        version: payload.version,
    }))
}

/// When `user`'s access to `store` ended, replaying the log entry by entry:
/// the time of the entry that took it away, or of an expiry. `None` while
/// the access lasts or when the user never had any.
#[must_use]
pub fn access_ended(log: &OrgLog, store: Id, user: Id, now: Millis) -> Option<Option<Millis>> {
    let entries = log.entries();
    let mut replay = OrgLog::genesis(entries.first()?.clone()).ok()?;
    let mut had = false;
    let mut ended = None;
    let mut last_expiry = None;
    for envelope in &entries[1..] {
        let entry = replay.append(envelope.clone()).ok()?;
        let state = replay.state();
        let role = state.store_role(store, user, entry.time);
        if role.is_some() {
            had = true;
            ended = None;
            last_expiry = state
                .stores
                .get(&store)
                .and_then(|store| {
                    store
                        .grants
                        .values()
                        .filter(|grant| grant.effective(entry.time))
                        .map(|grant| grant.expires)
                        .max()
                })
                .flatten();
        } else if had && ended.is_none() {
            ended = Some(
                last_expiry
                    .filter(|expiry| *expiry <= entry.time)
                    .unwrap_or(entry.time),
            );
        }
    }
    if !had {
        return None;
    }
    if ended.is_none() && log.state().store_role(store, user, now).is_none() {
        ended = Some(last_expiry.unwrap_or(now).min(now));
    }
    Some(ended)
}

/// `GET /v1/orgs/{org}/stores/{store}/rotation?user=`
pub async fn rotation(
    State(state): State<AppState>,
    Path((org, store)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    authed: Authed,
) -> ApiResult<Json<Rotation>> {
    let (org, store) = (parse_id(&org)?, parse_id(&store)?);
    let user = parse_id(query.get("user").map_or("", String::as_str))?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let now = state.now();
    if !(store_admin(&access, store, now) || access.has_role(&[Role::Admin, Role::Auditor])) {
        return Err(ApiError::forbidden(
            "only store admins, admins and auditors see rotation lists",
        ));
    }
    if !access.log.state().stores.contains_key(&store) {
        return Err(unknown_store());
    }
    let ended = access_ended(&access.log, store, user, now);
    let cutoff = ended.flatten().unwrap_or(now);
    let db = &state.db;
    let versions = sqlx::query(
        db.sql("SELECT envelope, time FROM index_versions WHERE store = $1 ORDER BY version"),
    )
    .bind(store.to_string())
    .fetch_all(db.pool())
    .await?;
    let mut existing = BTreeSet::new();
    for row in versions {
        if db::uint(&row, "time")? > cutoff {
            break;
        }
        let text: String = row.try_get("envelope")?;
        let envelope: Envelope = serde_json::from_str(&text)?;
        let payload: IndexPayload = envelope.open()?;
        for id in &payload.changed {
            existing.insert(*id);
        }
        for id in &payload.deleted {
            existing.remove(id);
        }
    }
    let fetched: BTreeSet<Id> = sqlx::query(db.sql("SELECT DISTINCT object FROM audit_events WHERE chain = $1 AND actor_user = $2 AND store = $3 AND action = 'object.read' AND object IS NOT NULL"))
        .bind(org.to_string())
        .bind(user.to_string())
        .bind(store.to_string())
        .fetch_all(db.pool())
        .await?
        .iter()
        .map(|row| db::id(row, "object"))
        .collect::<anyhow::Result<_>>()?;
    let mut items = Vec::new();
    if ended.flatten().is_some() {
        for id in existing.union(&fetched) {
            let rewritten = sqlx::query(db.sql(
                "SELECT 1 AS present FROM objects WHERE store = $1 AND id = $2 AND created > $3",
            ))
            .bind(store.to_string())
            .bind(id.to_string())
            .bind(to_i64(cutoff))
            .fetch_optional(db.pool())
            .await?
            .is_some();
            items.push(RotationItem {
                id: *id,
                fetched: fetched.contains(id),
                rewritten,
            });
        }
    }
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("rotation.read")
            .org(org)
            .store(store)
            .detail(format!("user {user}")),
    )
    .await?;
    Ok(Json(Rotation {
        user,
        ended: ended.flatten(),
        items,
    }))
}
