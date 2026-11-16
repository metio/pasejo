// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The HTTP API external-secrets-operator's webhook provider calls.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::bridge::{Bridge, Failure, Part};

#[derive(Clone)]
pub struct AppState {
    pub bridge: Arc<Mutex<Bridge>>,
    /// `(token hash, tenant)` pairs.
    pub credentials: Arc<Vec<([u8; 32], String)>>,
}

/// The bridge's routes.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/secrets/{store}/{*name}", get(secret))
        .route(
            "/healthz",
            get(|| async { reply(StatusCode::OK, &json!({ "ok": true })) }),
        )
        .route("/readyz", get(ready))
        .with_state(state)
}

fn reply(status: StatusCode, body: &serde_json::Value) -> Response {
    let mut response = (status, axum::Json(body)).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn error(status: StatusCode, message: &str) -> Response {
    reply(status, &json!({ "error": message }))
}

/// The tenant whose credential the request presents.
fn tenant(state: &AppState, headers: &HeaderMap) -> Option<String> {
    let token = headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;
    let hash: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    let mut found = None;
    for (candidate, tenant) in state.credentials.iter() {
        let difference = candidate
            .iter()
            .zip(hash)
            .fold(0, |difference, (left, right)| difference | (left ^ right));
        if difference == 0 {
            found = Some(tenant.clone());
        }
    }
    found
}

fn part(query: &HashMap<String, String>) -> Result<Part, &'static str> {
    if query.keys().any(|key| key != "line" && key != "field") {
        return Err("only the query parameters line and field are known");
    }
    match (query.get("line"), query.get("field")) {
        (Some(_), Some(_)) => Err("line and field exclude each other"),
        (Some(line), None) => line
            .parse::<isize>()
            .ok()
            .filter(|line| *line != 0)
            .map(Part::Line)
            .ok_or("line must be a non-zero integer"),
        (None, Some(field)) if !field.is_empty() => Ok(Part::Field(field.clone())),
        (None, Some(_)) => Err("field must not be empty"),
        (None, None) => Ok(Part::All),
    }
}

async fn secret(
    State(state): State<AppState>,
    Path((store, name)): Path<(String, String)>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let Some(tenant) = tenant(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "unknown or missing credential");
    };
    let part = match part(&query) {
        Ok(part) => part,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let bridge = Arc::clone(&state.bridge);
    let outcome = tokio::task::spawn_blocking(move || {
        let mut bridge = bridge
            .lock()
            .map_err(|_| Failure::Unavailable("the bridge failed earlier".to_owned()))?;
        bridge.read(&store, &name, &part, &tenant)
    })
    .await;
    match outcome {
        Ok(Ok(value)) => reply(StatusCode::OK, &json!({ "value": value.as_str() })),
        Ok(Err(Failure::NotFound)) => error(StatusCode::NOT_FOUND, "no such secret"),
        Ok(Err(Failure::NoSuchPart)) => error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "the secret has no such line or field",
        ),
        Ok(Err(Failure::Unavailable(message))) => {
            tracing::warn!(%message, "read unavailable");
            error(StatusCode::SERVICE_UNAVAILABLE, &message)
        }
        Err(join) => {
            tracing::error!(error = %join, "read failed");
            error(StatusCode::SERVICE_UNAVAILABLE, "the read failed")
        }
    }
}

async fn ready(State(state): State<AppState>) -> Response {
    let bridge = Arc::clone(&state.bridge);
    let outcome = tokio::task::spawn_blocking(move || {
        let mut bridge = bridge
            .lock()
            .map_err(|_| Failure::Unavailable("the bridge failed earlier".to_owned()))?;
        bridge.ready().map(|_| ())
    })
    .await;
    match outcome {
        Ok(Ok(())) => reply(StatusCode::OK, &json!({ "ready": true })),
        Ok(Err(Failure::Unavailable(message))) => error(StatusCode::SERVICE_UNAVAILABLE, &message),
        Ok(Err(_)) | Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "not ready"),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn query(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn query_parameters_select_one_part() {
        assert_eq!(part(&query(&[])), Ok(Part::All));
        assert_eq!(part(&query(&[("line", "2")])), Ok(Part::Line(2)));
        assert_eq!(part(&query(&[("line", "-1")])), Ok(Part::Line(-1)));
        assert_eq!(
            part(&query(&[("field", "user")])),
            Ok(Part::Field("user".into()))
        );
        for bad in [
            query(&[("line", "0")]),
            query(&[("line", "one")]),
            query(&[("field", "")]),
            query(&[("line", "1"), ("field", "user")]),
            query(&[("lines", "1")]),
        ] {
            assert!(part(&bad).is_err(), "{bad:?}");
        }
    }
}
