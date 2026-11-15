// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Operational surface: health checks, metrics, security headers, the
//! console, body limits, rate limits and client addresses behind proxies.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Harness, Options};
use pasejo_protocol::Id;
use pasejo_protocol::log::{Body, Settings};
use pasejo_server::audit::{self, SERVER_CHAIN};
use pasejo_server::console::CONSOLE_CSP;
use reqwest::{Method, StatusCode};
use serde_json::Value;

#[tokio::test]
async fn health_checks_answer_without_authentication() {
    let harness = Harness::start().await;
    let health = harness.raw(Method::GET, "/healthz", None).await;
    assert_eq!(
        (health.status, health.text().as_str()),
        (StatusCode::OK, "ok")
    );
    assert_eq!(
        harness.raw(Method::GET, "/readyz", None).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn the_main_listener_never_serves_metrics() {
    let harness = Harness::start().await;
    assert_eq!(
        harness.raw(Method::GET, "/metrics", None).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn the_metrics_listener_counts_requests_refusals_and_commits() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    alice
        .append(
            org,
            Body::StoreCreate {
                store: Id::random(),
            },
        )
        .await
        .ok::<Value>();
    harness.raw(Method::GET, "/v1/me", None).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = pasejo_server::app::metrics_router(harness.state.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let response = reqwest::get(format!("http://{address}/metrics"))
        .await
        .unwrap();
    assert_eq!(
        response.headers()["content-type"],
        "text/plain; version=0.0.4; charset=utf-8"
    );
    let text = response.text().await.unwrap();
    let value = |name: &str| -> u64 {
        text.lines()
            .find(|line| line.starts_with(name))
            .and_then(|line| line.rsplit(' ').next())
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("{name} missing from\n{text}"))
    };
    assert!(value("pasejo_requests_total{class=\"2xx\"}") > 0);
    assert!(value("pasejo_requests_total{class=\"4xx\"}") >= 1);
    assert!(value("pasejo_refusals_total") >= 1);
    assert!(value("pasejo_audit_events_total") > 0);
    assert_eq!(value("pasejo_commits_total"), 0);
}

#[tokio::test]
async fn api_responses_are_not_cached_or_sniffed() {
    let harness = Harness::start().await;
    for path in ["/v1/server", "/v1/me", "/healthz", "/nothing-here"] {
        let reply = harness.raw(Method::GET, path, None).await;
        assert_eq!(
            reply.header("cache-control").as_deref(),
            Some("no-store"),
            "{path}"
        );
        assert_eq!(
            reply.header("x-content-type-options").as_deref(),
            Some("nosniff"),
            "{path}"
        );
        // The test server speaks plain HTTP, so no HSTS.
        assert_eq!(reply.header("strict-transport-security"), None, "{path}");
    }
}

#[tokio::test]
async fn the_console_is_locked_down_by_its_policy() {
    let harness = Harness::start().await;
    let root = harness.raw(Method::GET, "/", None).await;
    assert!(root.status.is_redirection());
    assert_eq!(root.location().as_deref(), Some("/console/"));
    let index = harness.raw(Method::GET, "/console/", None).await;
    assert_eq!(index.status, StatusCode::OK);
    assert_eq!(
        index.header("content-security-policy").as_deref(),
        Some(CONSOLE_CSP)
    );
    assert!(CONSOLE_CSP.contains("frame-ancestors 'none'") && !CONSOLE_CSP.contains("unsafe"));
    assert_eq!(
        index.header("cross-origin-opener-policy").as_deref(),
        Some("same-origin")
    );
    assert_eq!(
        index.header("referrer-policy").as_deref(),
        Some("no-referrer")
    );
    assert_eq!(index.header("cache-control").as_deref(), Some("no-cache"));
    assert!(
        index
            .header("content-type")
            .unwrap()
            .starts_with("text/html")
    );
    let style = harness.raw(Method::GET, "/console/style.css", None).await;
    assert_eq!(
        style.header("content-type").as_deref(),
        Some("text/css; charset=utf-8")
    );
    for path in [
        "/console/missing.js",
        "/console/..%2fCargo.toml",
        "/console/package.json",
        "/console/tests/console.test.js",
    ] {
        assert_eq!(
            harness.raw(Method::GET, path, None).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn sign_in_pages_cannot_be_framed() {
    let harness = Harness::start().await;
    let page = harness
        .raw(Method::GET, "/auth/verify?code=ABCD-EFGH", None)
        .await;
    let policy = page.header("content-security-policy").unwrap();
    assert!(policy.contains("frame-ancestors 'none'"), "{policy}");
    assert!(policy.contains("default-src 'none'"), "{policy}");
}

#[tokio::test]
async fn bodies_beyond_the_limit_are_refused() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let big = serde_json::json!({"padding": "x".repeat(1_100_000)});
    let reply = alice.client.post("/v1/orgs", &big).await;
    reply.expect(StatusCode::PAYLOAD_TOO_LARGE, "too_large");
}

#[tokio::test]
async fn forwarded_addresses_count_only_from_trusted_proxies() {
    let untrusted = Harness::start().await;
    untrusted
        .http
        .get(format!("{}/v1/me", untrusted.url))
        .header("x-forwarded-for", "203.0.113.7")
        .send()
        .await
        .unwrap();
    let events = audit::events(&untrusted.state, SERVER_CHAIN, 0, 100)
        .await
        .unwrap();
    let refused = events
        .iter()
        .rfind(|event| event.action == "request.refused")
        .unwrap();
    assert_eq!(refused.ip.as_deref(), Some("127.0.0.1"));

    let trusted = Harness::start_with(Options {
        top: "trusted_proxies = [\"127.0.0.0/8\"]\n".into(),
        ..Options::default()
    })
    .await;
    trusted
        .http
        .get(format!("{}/v1/me", trusted.url))
        .header("x-forwarded-for", "198.51.100.1, 203.0.113.7")
        .header("user-agent", "pasejo-test")
        .send()
        .await
        .unwrap();
    let events = audit::events(&trusted.state, SERVER_CHAIN, 0, 100)
        .await
        .unwrap();
    let refused = events
        .iter()
        .rfind(|event| event.action == "request.refused")
        .unwrap();
    assert_eq!(refused.ip.as_deref(), Some("203.0.113.7"));
    assert_eq!(refused.user_agent.as_deref(), Some("pasejo-test"));
}

#[tokio::test]
async fn unknown_routes_are_not_audited() {
    let harness = Harness::start().await;
    let before = audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
        .await
        .unwrap()
        .len();
    for path in ["/.env", "/wp-admin/", "/console/../../etc/passwd"] {
        harness.raw(Method::GET, path, None).await;
    }
    assert_eq!(
        audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
            .await
            .unwrap()
            .len(),
        before
    );
}

#[tokio::test]
async fn maintenance_prunes_expired_state() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let count = || async {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions")
            .fetch_one(harness.state.db.pool())
            .await
            .unwrap()
    };
    // Expired sessions stay a day, so a late request still reads as expired.
    harness.advance(13 * 3_600_000);
    pasejo_server::app::maintain(&harness.state).await.unwrap();
    assert_eq!(count().await, 1);
    alice
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    harness.advance(24 * 3_600_000);
    pasejo_server::app::maintain(&harness.state).await.unwrap();
    assert_eq!(count().await, 0);
}
