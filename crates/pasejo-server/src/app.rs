// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The router, the middleware every request passes and the background
//! tasks of a serving process.

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use pasejo_protocol::Id;
use pasejo_protocol::api::ErrorCode;

use crate::api::{audit as audit_api, orgs, reviews, stores};
use crate::audit::{self, Event};
use crate::auth::{break_glass, flows};
use crate::db::to_i64;
use crate::state::{AppState, RequestCtx};
use crate::{actions, console, net, scim};

/// Builds the complete router.
#[allow(clippy::too_many_lines)]
pub fn router(state: AppState) -> Router {
    let limits = &state.config.limits;
    let api = Router::new()
        .route("/server", get(orgs::server))
        .route("/auth/device", post(flows::device))
        .route("/auth/token", post(flows::token))
        .route("/auth/web", post(flows::web))
        .route("/auth/workload", post(flows::workload))
        .route("/auth/emergency", post(flows::emergency))
        .route("/auth/logout", post(flows::logout))
        .route("/me", get(orgs::me))
        .route("/orgs", post(orgs::create))
        .route("/orgs/{org}/log", get(orgs::log).post(orgs::append))
        .route(
            "/orgs/{org}/log/proposals",
            get(orgs::proposals).post(orgs::propose),
        )
        .route("/orgs/{org}/log/proposals/{id}/cosign", post(orgs::cosign))
        .route(
            "/orgs/{org}/enrollments",
            get(orgs::enrollments).post(orgs::enroll),
        )
        .route(
            "/orgs/{org}/enrollments/{device}",
            get(orgs::enrollment_status),
        )
        .route("/orgs/{org}/users", get(orgs::org_users))
        .route("/orgs/{org}/actions", get(actions::list))
        .route("/orgs/{org}/stores", get(stores::list))
        .route("/orgs/{org}/recovery", get(stores::recovery))
        .route("/orgs/{org}/stores/{store}/keys", get(stores::keys))
        .route(
            "/orgs/{org}/stores/{store}/epochs/{epoch}",
            put(stores::upload_epoch),
        )
        .route(
            "/orgs/{org}/stores/{store}/wrappings",
            post(stores::upload_wrappings),
        )
        .route("/orgs/{org}/stores/{store}/index", get(stores::index))
        .route("/orgs/{org}/stores/{store}/lookup", get(stores::lookup))
        .route("/orgs/{org}/stores/{store}/objects", post(stores::objects))
        .route(
            "/orgs/{org}/stores/{store}/commits",
            post(stores::commit).layer(DefaultBodyLimit::max(limits.commit_body_bytes)),
        )
        .route("/orgs/{org}/stores/{store}/rotation", get(stores::rotation))
        .route("/orgs/{org}/audit", get(audit_api::events))
        .route("/orgs/{org}/audit/checkpoints", get(audit_api::checkpoints))
        .route(
            "/orgs/{org}/audit/checkpoints/{seq}/cosign",
            post(audit_api::cosign),
        )
        .route(
            "/orgs/{org}/reviews",
            get(reviews::list).post(reviews::create),
        )
        .route("/orgs/{org}/reviews/{id}/decisions", post(reviews::decide))
        .route(
            "/orgs/{org}/scim-tokens",
            get(orgs::list_scim_tokens).post(orgs::create_scim_token),
        )
        .route(
            "/orgs/{org}/scim-tokens/{id}",
            delete(orgs::delete_scim_token),
        )
        .route("/audit", get(audit_api::server_events))
        .route("/break-glass/login", post(break_glass::login))
        .route("/break-glass/emergency", post(break_glass::emergency))
        .route("/break-glass/users", get(break_glass::list_users))
        .route("/break-glass/orgs", get(break_glass::list_orgs))
        .route(
            "/break-glass/users/{id}/suspend",
            post(break_glass::suspend),
        )
        .route(
            "/break-glass/users/{id}/reactivate",
            post(break_glass::reactivate),
        )
        .route(
            "/break-glass/users/{id}/sessions/revoke",
            post(break_glass::end_sessions),
        );
    let scim = Router::new()
        .route(
            "/{org}/ServiceProviderConfig",
            get(scim::service_provider_config),
        )
        .route("/{org}/ResourceTypes", get(scim::resource_types))
        .route("/{org}/Schemas", get(scim::schemas))
        .route(
            "/{org}/Users",
            get(scim::list_users).post(scim::create_user),
        )
        .route(
            "/{org}/Users/{id}",
            get(scim::get_user)
                .put(scim::replace_user)
                .patch(scim::patch_user_handler)
                .delete(scim::delete_user),
        )
        .route(
            "/{org}/Groups",
            get(scim::list_groups).post(scim::create_group),
        )
        .route(
            "/{org}/Groups/{id}",
            get(scim::get_group)
                .put(scim::replace_group)
                .patch(scim::patch_group)
                .delete(scim::delete_group),
        );
    Router::new()
        .nest("/v1", api)
        .nest("/scim/v2", scim)
        .route("/auth/verify", get(flows::verify))
        .route("/auth/web/start", get(flows::web_start))
        .route("/auth/callback", get(flows::callback))
        .route("/auth/verify/approve", post(flows::approve))
        .route("/", get(console::root))
        .route("/console/", get(console::index))
        .route("/console/{*path}", get(console::file))
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .layer(DefaultBodyLimit::max(limits.body_bytes))
        .layer(middleware::from_fn_with_state(state.clone(), wrap))
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<AppState>) -> StatusCode {
    if state.db.ping().await.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

/// The router of the separate metrics listener.
pub fn metrics_router(state: AppState) -> Router {
    Router::new()
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn metrics(State(state): State<AppState>) -> Response {
    let mut response = state.metrics.render().into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
    );
    response
}

/// Whether a refused request on `path` is audited. Paths outside the API
/// are left out so that scanners probing for files cannot fill the chain.
fn audited_path(path: &str) -> bool {
    path.starts_with("/v1/") || path.starts_with("/scim/") || path.starts_with("/auth/")
}

/// The organization a path names, for attributing a refusal.
fn org_in_path(path: &str) -> Option<Id> {
    let rest = path
        .strip_prefix("/v1/orgs/")
        .or_else(|| path.strip_prefix("/scim/v2/"))?;
    Id::from_str(rest.split('/').next()?).ok()
}

async fn known_org(state: &AppState, org: Id) -> bool {
    sqlx::query(state.db.sql("SELECT 1 AS present FROM orgs WHERE id = $1"))
        .bind(org.to_string())
        .fetch_optional(state.db.pool())
        .await
        .ok()
        .flatten()
        .is_some()
}

/// Request context, refusal auditing, security headers and metrics.
async fn wrap(State(state): State<AppState>, mut request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map_or_else(|| std::net::Ipv4Addr::LOCALHOST.into(), |info| info.0.ip());
    let ip = net::client_address(peer, request.headers(), &state.config.trusted_proxies);
    let user_agent = request
        .headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|agent| agent.chars().take(256).collect());
    let ctx = Arc::new(RequestCtx::new(ip, user_agent));
    request.extensions_mut().insert(Arc::clone(&ctx));
    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let mut response = next.run(request).await;
    let status = response.status();
    let code = response.extensions().get::<ErrorCode>().copied();
    let polling = matches!(
        code,
        Some(ErrorCode::AuthorizationPending | ErrorCode::SlowDown)
    );
    if (status.is_client_error() || status.is_server_error())
        && !ctx.audited()
        && !polling
        && audited_path(&path)
    {
        let label = code.map_or_else(
            || status.as_u16().to_string(),
            |code| {
                serde_json::to_value(code)
                    .ok()
                    .and_then(|value| value.as_str().map(ToOwned::to_owned))
                    .unwrap_or_default()
            },
        );
        let mut event = Event::denied("request.refused")
            .detail(format!("{method} {path} {} {label}", status.as_u16()));
        if let Some(org) = org_in_path(&path)
            && known_org(&state, org).await
        {
            event = event.org(org);
        }
        if let Err(error) = audit::record(&state, Some(&ctx), event).await {
            tracing::error!(message = %error.message, "cannot audit a refused request");
        }
    }
    state.metrics.request(status.as_u16());

    let headers = response.headers_mut();
    if !headers.contains_key(header::CACHE_CONTROL) {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    if state.config.https() {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=63072000; includeSubDomains"),
        );
    }
    tracing::info!(method = %method, path = %path, status = status.as_u16(), "request");
    response
}

/// Deletes expired nonces, grants, sign-in flows, cookies and sessions.
///
/// # Errors
///
/// Fails when the database does.
pub async fn prune_expired(state: &AppState) -> anyhow::Result<()> {
    let now = state.now();
    let db = &state.db;
    let statements: [(&'static str, u64); 5] = [
        ("DELETE FROM nonces WHERE expires < $1", now),
        (
            "DELETE FROM device_grants WHERE expires < $1",
            now.saturating_sub(3_600_000),
        ),
        (
            "DELETE FROM oidc_flows WHERE created < $1",
            now.saturating_sub(3_600_000),
        ),
        ("DELETE FROM web_logins WHERE expires < $1", now),
        (
            "DELETE FROM sessions WHERE expires < $1",
            now.saturating_sub(86_400_000),
        ),
    ];
    for (statement, cutoff) in statements {
        sqlx::query(db.sql(statement))
            .bind(to_i64(cutoff))
            .execute(db.pool())
            .await?;
    }
    Ok(())
}

/// One round of periodic maintenance.
///
/// # Errors
///
/// Fails when the database does.
pub async fn maintain(state: &AppState) -> anyhow::Result<()> {
    audit::checkpoint_due(state)
        .await
        .map_err(|error| anyhow::anyhow!(error.message))?;
    prune_expired(state).await?;
    if let Some(days) = state.config.audit.retention_days {
        let before = state.now().saturating_sub(days * 86_400_000);
        audit::prune(state, before)
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
    }
    Ok(())
}

/// Starts the background tasks of a serving process.
pub fn spawn_background(state: &AppState) {
    let maintenance = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(error) = maintain(&maintenance).await {
                tracing::error!(error = %format!("{error:#}"), "maintenance failed");
            }
        }
    });
    tokio::spawn(audit::webhook_loop(state.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refusals_name_their_organization() {
        let org = Id([7; 16]);
        assert_eq!(org_in_path(&format!("/v1/orgs/{org}/stores")), Some(org));
        assert_eq!(org_in_path(&format!("/scim/v2/{org}/Users")), Some(org));
        assert_eq!(org_in_path("/v1/orgs/zzz/stores"), None);
        assert_eq!(org_in_path("/v1/me"), None);
    }

    #[test]
    fn only_api_paths_are_audited() {
        assert!(audited_path("/v1/me"));
        assert!(audited_path("/scim/v2/x/Users"));
        assert!(audited_path("/auth/callback"));
        assert!(!audited_path("/wp-login.php"));
        assert!(!audited_path("/console/x.js"));
    }
}
