// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pending actions: the difference between the state the server wants and
//! the state the membership log describes, computed on every request.

use std::collections::BTreeSet;

use axum::Json;
use axum::extract::{Path, State};
use pasejo_protocol::api::{Action, Actions, Decision};
use pasejo_protocol::log::{OrgState, Role, StoreRole};
use pasejo_protocol::{Id, Millis};
use sqlx::Row;

use crate::api::orgs::{join_requests, pending_enrollments};
use crate::api::reviews::load_reviews;
use crate::auth::{Authed, OrgAccess};
use crate::db;
use crate::error::ApiResult;
use crate::state::{AppState, parse_id};
use crate::users;

/// Every pending action of `org`.
///
/// # Errors
///
/// Fails when the database does.
#[allow(clippy::too_many_lines)]
pub async fn compute(state: &AppState, org: Id, now: Millis) -> ApiResult<Vec<Action>> {
    let log = state.logs.get(&state.db, org).await?;
    let org_state = log.state();
    let db = &state.db;
    let mut actions = Vec::new();

    for enrollment in pending_enrollments(state, org).await? {
        actions.push(Action::ApproveDevice { enrollment });
    }

    let mut joining: BTreeSet<Id> = join_requests(state, org).await?.into_keys().collect();
    let scim_rows =
        sqlx::query(db.sql("SELECT user_id, active, deleted FROM scim_users WHERE org = $1"))
            .bind(org.to_string())
            .fetch_all(db.pool())
            .await?;
    let mut leaving = BTreeSet::new();
    for row in &scim_rows {
        let user = db::id(row, "user_id")?;
        let active = db::uint(row, "active")? != 0 && db::uint(row, "deleted")? == 0;
        if active {
            joining.insert(user);
        } else if org_state.members.contains_key(&user) {
            leaving.insert(user);
        }
    }
    for user in joining {
        if org_state.members.contains_key(&user) || leaving.contains(&user) {
            continue;
        }
        if let Some(summary) = users::summary(state, user).await?
            && summary.active
        {
            actions.push(Action::JoinMember {
                user,
                name: summary.name,
                email: summary.email,
            });
        }
    }
    for user in leaving {
        actions.push(Action::RemoveMember { user });
    }

    let groups = sqlx::query(
        db.sql("SELECT id, display_name, members, deleted FROM scim_groups WHERE org = $1"),
    )
    .bind(org.to_string())
    .fetch_all(db.pool())
    .await?;
    for row in groups {
        let team = db::id(&row, "id")?;
        let deleted = db::uint(&row, "deleted")? != 0;
        let text: String = row.try_get("members")?;
        let wanted: BTreeSet<Id> = if deleted {
            BTreeSet::new()
        } else {
            serde_json::from_str::<Vec<Id>>(&text)?
                .into_iter()
                .filter(|user| org_state.members.contains_key(user))
                .collect()
        };
        let existing = org_state.teams.get(&team);
        let current: BTreeSet<Id> =
            existing.map_or_else(BTreeSet::new, |team| team.members.clone());
        let add: Vec<Id> = wanted.difference(&current).copied().collect();
        let remove: Vec<Id> = current.difference(&wanted).copied().collect();
        let needs_team = existing.is_none() && !deleted;
        if needs_team || !add.is_empty() || !remove.is_empty() {
            actions.push(Action::SyncTeam {
                team,
                name: row.try_get("display_name")?,
                exists: existing.is_some(),
                add,
                remove,
            });
        }
    }

    for (store, store_state) in &org_state.stores {
        if store_state.deleted {
            continue;
        }
        let entitled = org_state.entitled(*store, now);
        if let Some(epoch) = store_state.current_epoch() {
            let record = &store_state.epochs[usize::try_from(epoch).unwrap_or(usize::MAX)];
            // Devices named in the epoch entry and devices that received a
            // wrapping later both hold the keys.
            let wrapped = wrapped_devices(state, *store, epoch).await?;
            let holds_data = record.full.iter().copied().chain(
                wrapped
                    .iter()
                    .filter(|(_, full)| **full)
                    .map(|(device, _)| *device),
            );
            let holds_names = record
                .full
                .iter()
                .chain(&record.names_only)
                .chain(wrapped.keys())
                .copied();
            let stale_full = holds_data
                .clone()
                .any(|device| !entitled.full.contains(&device));
            let stale_names = holds_names.clone().any(|device| {
                !entitled.full.contains(&device) && !entitled.names_only.contains(&device)
            });
            if stale_full || stale_names {
                actions.push(Action::RotateEpoch {
                    store: *store,
                    epoch: epoch + 1,
                });
            } else {
                let missing: Vec<Id> = entitled
                    .full
                    .iter()
                    .filter(|device| wrapped.get(device) != Some(&true))
                    .chain(
                        entitled
                            .names_only
                            .iter()
                            .filter(|device| !wrapped.contains_key(device)),
                    )
                    .copied()
                    .collect();
                if !missing.is_empty() {
                    actions.push(Action::WrapKeys {
                        store: *store,
                        epoch,
                        devices: missing,
                    });
                }
            }
        } else {
            actions.push(Action::RotateEpoch {
                store: *store,
                epoch: 0,
            });
        }
        for (principal, grant) in &store_state.grants {
            if !grant.effective(now) {
                actions.push(Action::RevokeExpired {
                    store: *store,
                    principal: *principal,
                });
            }
        }
    }

    for review in load_reviews(state, org).await? {
        for item in &review.items {
            let still_granted = org_state
                .stores
                .get(&review.store)
                .is_some_and(|store| store.grants.contains_key(&item.principal));
            if item.decision == Some(Decision::Revoke) && still_granted {
                actions.push(Action::RevokeGrant {
                    store: review.store,
                    principal: item.principal,
                    review: review.id,
                });
            }
        }
    }

    let checkpoints = sqlx::query(
        db.sql("SELECT seq, cosignatures FROM checkpoints WHERE chain = $1 ORDER BY seq"),
    )
    .bind(org.to_string())
    .fetch_all(db.pool())
    .await?;
    for row in checkpoints {
        let text: String = row.try_get("cosignatures")?;
        if text.trim() == "[]" {
            actions.push(Action::CosignCheckpoint {
                seq: db::uint(&row, "seq")?,
            });
        }
    }
    Ok(actions)
}

async fn wrapped_devices(
    state: &AppState,
    store: Id,
    epoch: u64,
) -> ApiResult<std::collections::BTreeMap<Id, bool>> {
    let rows = sqlx::query(
        state
            .db
            .sql("SELECT device, full_keys FROM wrappings WHERE store = $1 AND epoch = $2"),
    )
    .bind(store.to_string())
    .bind(db::to_i64(epoch))
    .fetch_all(state.db.pool())
    .await?;
    rows.iter()
        .map(|row| Ok((db::id(row, "device")?, db::uint(row, "full_keys")? != 0)))
        .collect()
}

/// Whether `user` could resolve `action`. Admins see every action.
fn visible(
    org_state: &OrgState,
    user: Id,
    role: Option<Role>,
    action: &Action,
    now: Millis,
) -> bool {
    if matches!(role, Some(Role::Admin)) {
        return true;
    }
    let store_admin =
        |store: &Id| org_state.store_role(*store, user, now) == Some(StoreRole::Admin);
    match action {
        Action::ApproveDevice { enrollment } => {
            matches!(role, Some(Role::Approver)) || enrollment.device.user == user
        }
        Action::JoinMember { .. } | Action::RemoveMember { .. } | Action::SyncTeam { .. } => false,
        Action::RotateEpoch { store, .. } | Action::RevokeExpired { store, .. } => {
            store_admin(store)
        }
        Action::WrapKeys { store, .. } => org_state.store_role(*store, user, now).is_some(),
        Action::RevokeGrant { store, .. } => store_admin(store),
        Action::CosignCheckpoint { .. } => matches!(role, Some(Role::Auditor)),
    }
}

/// `GET /v1/orgs/{org}/actions`
pub async fn list(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Actions>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let user = authed.user()?;
    let now = state.now();
    let actions = compute(&state, org, now)
        .await?
        .into_iter()
        .filter(|action| visible(access.log.state(), user, access.role, action, now))
        .collect();
    Ok(Json(Actions { actions }))
}
