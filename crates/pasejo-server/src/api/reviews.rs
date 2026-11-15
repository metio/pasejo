// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use pasejo_protocol::Id;
use pasejo_protocol::api::{Review, ReviewDecision, ReviewItem, ReviewRequest, Reviews};
use pasejo_protocol::log::{Principal, Role, StoreRole};
use sqlx::Row;

use crate::audit::{self, Event};
use crate::auth::{Authed, OrgAccess};
use crate::db::{self, to_i64};
use crate::error::{ApiError, ApiResult};
use crate::state::{AppState, parse_id};

fn from_row(row: &sqlx::any::AnyRow) -> ApiResult<Review> {
    let reviewers: String = row.try_get("reviewers")?;
    let items: String = row.try_get("items")?;
    let items: Vec<ReviewItem> = serde_json::from_str(&items)?;
    Ok(Review {
        id: db::id(row, "id")?,
        store: db::id(row, "store")?,
        reviewers: serde_json::from_str(&reviewers)?,
        due: db::uint(row, "due")?,
        created: db::uint(row, "created")?,
        created_by: db::id(row, "created_by")?,
        complete: items.iter().all(|item| item.decision.is_some()),
        items,
    })
}

/// Every review of `org`.
///
/// # Errors
///
/// Fails when the database does.
pub async fn load_reviews(state: &AppState, org: Id) -> ApiResult<Vec<Review>> {
    let rows = sqlx::query(state.db.sql("SELECT id, store, reviewers, due, created, created_by, items FROM reviews WHERE org = $1 ORDER BY created"))
        .bind(org.to_string())
        .fetch_all(state.db.pool())
        .await?;
    rows.iter().map(from_row).collect()
}

/// `POST /v1/orgs/{org}/reviews`
pub async fn create(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<(StatusCode, Json<Review>)> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let user = authed.user()?;
    let request: ReviewRequest = authed.json()?;
    let now = state.now();
    let org_state = access.log.state();
    let store = org_state
        .stores
        .get(&request.store)
        .filter(|store| !store.deleted)
        .ok_or_else(|| {
            ApiError::new(
                pasejo_protocol::api::ErrorCode::UnknownStore,
                "unknown store",
            )
        })?;
    let store_admin = org_state.store_role(request.store, user, now) == Some(StoreRole::Admin);
    if !(store_admin || access.has_role(&[Role::Admin])) {
        return Err(ApiError::forbidden(
            "only admins and store admins open reviews",
        ));
    }
    if request.reviewers.is_empty()
        || request
            .reviewers
            .iter()
            .any(|reviewer| !org_state.members.contains_key(reviewer))
    {
        return Err(ApiError::bad_request("reviewers must be members"));
    }
    if request.due <= now {
        return Err(ApiError::bad_request("the due date lies in the past"));
    }
    let items: Vec<ReviewItem> = store
        .grants
        .iter()
        .map(|(principal, grant)| ReviewItem {
            principal: *principal,
            role: grant.role,
            decision: None,
            decided_by: None,
        })
        .collect();
    let review = Review {
        id: Id::random(),
        store: request.store,
        reviewers: request.reviewers,
        due: request.due,
        created: now,
        created_by: user,
        complete: items.is_empty(),
        items,
    };
    sqlx::query(state.db.sql("INSERT INTO reviews (id, org, store, reviewers, due, created, created_by, items) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"))
        .bind(review.id.to_string())
        .bind(org.to_string())
        .bind(review.store.to_string())
        .bind(serde_json::to_string(&review.reviewers)?)
        .bind(to_i64(review.due))
        .bind(to_i64(now))
        .bind(user.to_string())
        .bind(serde_json::to_string(&review.items)?)
        .execute(state.db.pool())
        .await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("review.create")
            .org(org)
            .store(review.store)
            .detail(review.id.to_string()),
    )
    .await?;
    Ok((StatusCode::CREATED, Json(review)))
}

/// `GET /v1/orgs/{org}/reviews`
pub async fn list(
    State(state): State<AppState>,
    Path(org): Path<String>,
    authed: Authed,
) -> ApiResult<Json<Reviews>> {
    let org = parse_id(&org)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let user = authed.user()?;
    let now = state.now();
    let overseer = access.has_role(&[Role::Admin, Role::Auditor]);
    let reviews = load_reviews(&state, org)
        .await?
        .into_iter()
        .filter(|review| {
            overseer
                || review.reviewers.contains(&user)
                || access.log.state().store_role(review.store, user, now) == Some(StoreRole::Admin)
        })
        .collect();
    Ok(Json(Reviews { reviews }))
}

/// `POST /v1/orgs/{org}/reviews/{id}/decisions`
pub async fn decide(
    State(state): State<AppState>,
    Path((org, id)): Path<(String, String)>,
    authed: Authed,
) -> ApiResult<Json<Review>> {
    let org = parse_id(&org)?;
    let id = parse_id(&id)?;
    let access = OrgAccess::load(&state, &authed, org).await?;
    access.device()?;
    let user = authed.user()?;
    let request: ReviewDecision = authed.json()?;
    let db = &state.db;
    let mut transaction = db.begin().await?;
    let statement = if db.is_sqlite() {
        db.sql("SELECT id, store, reviewers, due, created, created_by, items FROM reviews WHERE id = $1 AND org = $2")
    } else {
        "SELECT id, store, reviewers, due, created, created_by, items FROM reviews WHERE id = $1 AND org = $2 FOR UPDATE"
    };
    let row = sqlx::query(statement)
        .bind(id.to_string())
        .bind(org.to_string())
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| ApiError::not_found("no such review"))?;
    let mut review = from_row(&row)?;
    if !review.reviewers.contains(&user) {
        return Err(ApiError::forbidden("you are not a reviewer"));
    }
    let item = review
        .items
        .iter_mut()
        .find(|item| item.principal == request.principal)
        .ok_or_else(|| ApiError::not_found("the review has no such grant"))?;
    let benefits = match item.principal {
        Principal::User(holder) => holder == user,
        Principal::Team(team) => access
            .log
            .state()
            .teams
            .get(&team)
            .is_some_and(|team| team.members.contains(&user)),
    };
    if benefits {
        return Err(ApiError::forbidden(
            "reviewers do not decide on their own access",
        ));
    }
    item.decision = Some(request.decision);
    item.decided_by = Some(user);
    review.complete = review.items.iter().all(|item| item.decision.is_some());
    sqlx::query(db.sql("UPDATE reviews SET items = $1 WHERE id = $2"))
        .bind(serde_json::to_string(&review.items)?)
        .bind(id.to_string())
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    audit::record(
        &state,
        Some(&authed.ctx),
        Event::ok("review.decide")
            .org(org)
            .store(review.store)
            .detail(format!(
                "{id}: {:?} {:?}",
                request.principal, request.decision
            )),
    )
    .await?;
    Ok(Json(review))
}
