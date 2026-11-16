// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sign-in flows, sessions and request proofs.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::atomic::Ordering;

use common::{Harness, Options, Reply, between};
use ed25519_dalek::SigningKey;
use pasejo_protocol::api::{Me, ServerInfo, TokenRequest, TokenResponse};
use pasejo_protocol::encoding::{b64, random, sha256};
use pasejo_protocol::request::{NONCE_HEADER, PROOF_HEADER, TIMESTAMP_HEADER, prove};
use rand_core::OsRng;
use reqwest::{Method, StatusCode};
use serde_json::json;

fn key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

#[tokio::test]
async fn a_device_grant_signs_a_person_in() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let me: Me = alice.get("/v1/me").await.ok();
    assert_eq!(me.email, "alice@example.com");
    assert_eq!(me.name, "Alice");
    assert!(!me.machine);
    assert!(me.orgs.is_empty());
}

#[tokio::test]
async fn signing_in_again_keeps_the_same_user() {
    let harness = Harness::start().await;
    let first = harness.sign_in("alice").await;
    let second = harness.sign_in("alice").await;
    assert_eq!(first.user, second.user);
    assert_ne!(first.token, second.token);
    let bob = harness.sign_in("bob").await;
    assert_ne!(first.user, bob.user);
}

#[tokio::test]
async fn the_server_describes_itself() {
    let harness = Harness::start().await;
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    assert!(info.oidc);
    assert!(info.break_glass);
    assert_eq!(info.emergency_until, None);
    assert_eq!(
        info.checkpoint_key.0,
        harness.state.keys.checkpoint.verifying_key().to_bytes()
    );
}

#[tokio::test]
async fn polling_reports_pending_and_slow_down_until_approval() {
    let harness = Harness::start().await;
    let key = key();
    let (authorization, approval) = harness.device_grant(&key, "alice").await;
    assert_eq!(authorization.user_code.len(), 9);
    assert_eq!(authorization.interval, 5);
    assert_eq!(authorization.expires_in, 600);
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "authorization_pending");
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "slow_down");
    harness.advance(5_000);
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "authorization_pending");
    harness.approve(&approval, "approve").await;
    let response: TokenResponse = harness.poll(&key, &authorization.device_code).await.ok();
    assert!(response.user.is_some());
    // The device code is spent.
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "expired_token");
}

#[tokio::test]
async fn a_denied_grant_never_yields_a_session() {
    let harness = Harness::start().await;
    let key = key();
    let (authorization, approval) = harness.device_grant(&key, "alice").await;
    let page = harness.approve(&approval, "deny").await;
    assert!(page.text().contains("Device denied"));
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "access_denied");
}

#[tokio::test]
async fn an_approval_token_works_once() {
    let harness = Harness::start().await;
    let key = key();
    let (_, approval) = harness.device_grant(&key, "alice").await;
    assert!(
        harness
            .approve(&approval, "approve")
            .await
            .text()
            .contains("Device approved")
    );
    let again = harness.approve(&approval, "approve").await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert!(again.text().contains("unknown or used approval"));
}

#[tokio::test]
async fn grants_expire_after_ten_minutes() {
    let harness = Harness::start().await;
    let key = key();
    let (authorization, approval) = harness.device_grant(&key, "alice").await;
    harness.advance(601_000);
    let page = harness.approve(&approval, "approve").await;
    assert_eq!(page.status, StatusCode::BAD_REQUEST);
    harness
        .poll(&key, &authorization.device_code)
        .await
        .expect(StatusCode::BAD_REQUEST, "expired_token");
}

#[tokio::test]
async fn only_the_key_that_started_a_grant_can_redeem_it() {
    let harness = Harness::start().await;
    let key = key();
    let (authorization, approval) = harness.device_grant(&key, "alice").await;
    harness.approve(&approval, "approve").await;
    harness
        .poll(&self::key(), &authorization.device_code)
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    harness
        .poll(&key, "made-up")
        .await
        .expect(StatusCode::BAD_REQUEST, "expired_token");
}

#[tokio::test]
async fn claims_must_be_fresh_signed_and_for_their_purpose() {
    let harness = Harness::start().await;
    let key = key();
    let (authorization, approval) = harness.device_grant(&key, "alice").await;
    harness.approve(&approval, "approve").await;
    let post = |claim| {
        harness.raw(
            Method::POST,
            "/v1/auth/token",
            Some(
                serde_json::to_value(TokenRequest {
                    claim,
                    jwt: None,
                    verifier: None,
                })
                .unwrap(),
            ),
        )
    };
    post(harness.claim(&key, "web", &authorization.device_code))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut tampered = harness.claim(&key, "device", &authorization.device_code);
    tampered.signatures[0].signature.0[0] ^= 1;
    post(tampered)
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    let mut unsigned = harness.claim(&key, "device", &authorization.device_code);
    unsigned.signatures.clear();
    post(unsigned)
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    let stale = harness.claim(&key, "device", &authorization.device_code);
    harness.advance(61_000);
    post(stale)
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    // A fresh claim still redeems the grant.
    post(harness.claim(&key, "device", &authorization.device_code))
        .await
        .ok::<TokenResponse>();
}

#[tokio::test]
async fn the_verification_page_handles_unknown_and_missing_codes() {
    let harness = Harness::start().await;
    let form = harness.raw(Method::GET, "/auth/verify", None).await;
    assert_eq!(form.status, StatusCode::OK);
    assert!(form.text().contains("name=\"user_code\""));
    assert_eq!(
        form.header("content-security-policy").unwrap(),
        pasejo_server::html::PAGE_CSP
    );
    let unknown = harness
        .raw(Method::GET, "/auth/verify?user_code=BCDF-GHJK", None)
        .await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert!(unknown.text().contains("Unknown code"));
}

#[tokio::test]
async fn user_codes_are_accepted_without_dash_and_in_lower_case() {
    let harness = Harness::start().await;
    let key = key();
    let authorization: pasejo_protocol::api::DeviceAuthorization = harness
        .raw(
            Method::POST,
            "/v1/auth/device",
            Some(json!({"signing_key": b64(key.verifying_key().as_bytes()), "label": "laptop"})),
        )
        .await
        .ok();
    let typed = authorization.user_code.replace('-', "").to_lowercase();
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let page = harness
        .follow_to_callback(&format!("{}/auth/verify?user_code={typed}", harness.url))
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    assert!(page.text().contains("Approve this device?"));
    assert!(page.text().contains("alice@example.com"));
}

#[tokio::test]
async fn the_approval_page_escapes_the_device_label() {
    let harness = Harness::start().await;
    let key = key();
    let authorization: pasejo_protocol::api::DeviceAuthorization = harness
        .raw(
            Method::POST,
            "/v1/auth/device",
            Some(json!({"signing_key": b64(key.verifying_key().as_bytes()), "label": "<script>x</script>"})),
        )
        .await
        .ok();
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let page = harness
        .follow_to_callback(&authorization.verification_uri_complete)
        .await;
    assert!(!page.text().contains("<script>"));
    assert!(page.text().contains("&lt;script&gt;"));
}

#[tokio::test]
async fn device_requests_are_validated() {
    let harness = Harness::start().await;
    let key = b64(key().verifying_key().as_bytes());
    for body in [
        json!({"signing_key": key, "label": ""}),
        json!({"signing_key": key, "label": "a\nb"}),
        json!({"signing_key": key, "label": "x".repeat(201)}),
        json!({"signing_key": b64(&[0u8; 31]), "label": "laptop"}),
        json!({"label": "laptop"}),
    ] {
        harness
            .raw(Method::POST, "/v1/auth/device", Some(body))
            .await
            .expect(StatusCode::BAD_REQUEST, "bad_request");
    }
}

#[tokio::test]
async fn provider_failures_end_the_sign_in() {
    let harness = Harness::start().await;
    for failure in [
        &harness.idp.forge,
        &harness.idp.wrong_nonce,
        &harness.idp.deny,
    ] {
        failure.store(true, Ordering::Relaxed);
        let key = key();
        let authorization: pasejo_protocol::api::DeviceAuthorization = harness
            .raw(
                Method::POST,
                "/v1/auth/device",
                Some(
                    json!({"signing_key": b64(key.verifying_key().as_bytes()), "label": "laptop"}),
                ),
            )
            .await
            .ok();
        let page = harness
            .follow_to_callback(&authorization.verification_uri_complete)
            .await;
        assert_eq!(page.status, StatusCode::BAD_REQUEST, "{}", page.text());
        assert!(page.text().contains("Sign-in failed"));
        assert!(!page.text().contains("name=\"approval\""));
        failure.store(false, Ordering::Relaxed);
    }
}

#[tokio::test]
async fn a_callback_state_cannot_be_replayed() {
    let harness = Harness::start().await;
    let key = key();
    let authorization: pasejo_protocol::api::DeviceAuthorization = harness
        .raw(
            Method::POST,
            "/v1/auth/device",
            Some(json!({"signing_key": b64(key.verifying_key().as_bytes()), "label": "laptop"})),
        )
        .await
        .ok();
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let redirect = harness
        .raw(
            Method::GET,
            &authorization
                .verification_uri_complete
                .replace(&harness.url, ""),
            None,
        )
        .await;
    let provider = redirect.location().unwrap();
    let callback = common::Reply::from(harness.http.get(&provider).send().await.unwrap())
        .await
        .location()
        .unwrap();
    let first = common::Reply::from(harness.http.get(&callback).send().await.unwrap()).await;
    assert_eq!(first.status, StatusCode::OK);
    let second = common::Reply::from(harness.http.get(&callback).send().await.unwrap()).await;
    assert_eq!(second.status, StatusCode::BAD_REQUEST);
    let forged = harness
        .raw(Method::GET, "/auth/callback?state=made-up&code=x", None)
        .await;
    assert_eq!(forged.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn unverified_emails_are_not_recorded() {
    let harness = Harness::start().await;
    let key = key();
    let authorization: pasejo_protocol::api::DeviceAuthorization = harness
        .raw(
            Method::POST,
            "/v1/auth/device",
            Some(json!({"signing_key": b64(key.verifying_key().as_bytes()), "label": "laptop"})),
        )
        .await
        .ok();
    harness
        .idp
        .set_user("mallory", "ceo@example.com", "Mallory");
    harness.idp.set_unverified();
    let page = harness
        .follow_to_callback(&authorization.verification_uri_complete)
        .await;
    let approval = between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
    harness.approve(&approval, "approve").await;
    let response: TokenResponse = harness.poll(&key, &authorization.device_code).await.ok();
    let client = harness.client(key, response);
    let me: Me = client.get("/v1/me").await.ok();
    assert_eq!(me.email, "");
    assert_eq!(me.name, "Mallory");
}

#[tokio::test]
async fn without_an_identity_provider_people_cannot_sign_in() {
    let harness = Harness::start_with(Options {
        oidc: false,
        ..Options::default()
    })
    .await;
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    assert!(!info.oidc);
    harness
        .raw(
            Method::POST,
            "/v1/auth/device",
            Some(json!({"signing_key": b64(key().verifying_key().as_bytes()), "label": "laptop"})),
        )
        .await
        .expect(StatusCode::SERVICE_UNAVAILABLE, "unavailable");
    let start = harness.raw(Method::GET, "/auth/web/start", None).await;
    assert_eq!(start.status, StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn authentication_endpoints_are_rate_limited_per_address() {
    let harness = Harness::start_with(Options {
        extra: "[limits]\nauth_per_minute = 3\n".into(),
        ..Options::default()
    })
    .await;
    let body = json!({"signing_key": b64(key().verifying_key().as_bytes()), "label": "laptop"});
    for _ in 0..3 {
        harness
            .raw(Method::POST, "/v1/auth/device", Some(body.clone()))
            .await
            .ok::<serde_json::Value>();
    }
    harness
        .raw(Method::POST, "/v1/auth/device", Some(body.clone()))
        .await
        .expect(StatusCode::TOO_MANY_REQUESTS, "rate_limited");
    harness.advance(60_000);
    harness
        .raw(Method::POST, "/v1/auth/device", Some(body))
        .await
        .ok::<serde_json::Value>();
}

#[tokio::test]
async fn the_console_sign_in_exchanges_a_single_use_cookie() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let reply = harness
        .follow_to_callback(&format!("{}/auth/web/start", harness.url))
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.location().as_deref(), Some("/console/"));
    let cookie = reply.header("set-cookie").unwrap();
    assert!(cookie.contains("Path=/v1/auth/web"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(!cookie.contains("Secure"), "the test server is plain HTTP");
    let value = cookie.split(';').next().unwrap().to_owned();
    let key = key();
    let exchange = |claim| {
        let value = value.clone();
        let harness = &harness;
        async move {
            common::Reply::from(
                harness
                    .http
                    .post(format!("{}/v1/auth/web", harness.url))
                    .header("cookie", value)
                    .header("content-type", "application/json")
                    .body(
                        serde_json::to_vec(&TokenRequest {
                            claim,
                            jwt: None,
                            verifier: None,
                        })
                        .unwrap(),
                    )
                    .send()
                    .await
                    .unwrap(),
            )
            .await
        }
    };
    exchange(harness.claim(&key, "device", ""))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let response: TokenResponse = exchange(harness.claim(&key, "web", "")).await.ok();
    let me: Me = harness
        .client(key.clone(), response)
        .get("/v1/me")
        .await
        .ok();
    assert_eq!(me.email, "alice@example.com");
    exchange(harness.claim(&key, "web", ""))
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn console_cookies_expire_after_ten_minutes() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let reply = harness
        .follow_to_callback(&format!("{}/auth/web/start", harness.url))
        .await;
    let value = reply
        .header("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    harness.advance(601_000);
    let key = key();
    let reply = common::Reply::from(
        harness
            .http
            .post(format!("{}/v1/auth/web", harness.url))
            .header("cookie", value)
            .body(
                serde_json::to_vec(&TokenRequest {
                    claim: harness.claim(&key, "web", ""),
                    jwt: None,
                    verifier: None,
                })
                .unwrap(),
            )
            .send()
            .await
            .unwrap(),
    )
    .await;
    reply.expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn workloads_sign_in_as_the_machine_their_rule_names() {
    let harness = Harness::start().await;
    let key = key();
    let jwt = harness.workload_jwt("system:serviceaccount:ci:builder", "pasejo-server", 600);
    let response: TokenResponse = harness.workload(&key, &jwt).await.ok();
    let client = harness.client(key.clone(), response.clone());
    let me: Me = client.get("/v1/me").await.ok();
    assert!(me.machine);
    assert_eq!(me.name, "ci");
    // Another subject under the same rule maps to the same machine.
    let other = harness.workload_jwt("system:serviceaccount:ci:deployer", "pasejo-server", 600);
    let second: TokenResponse = harness.workload(&self::key(), &other).await.ok();
    assert_eq!(second.user, response.user);
    // An exact rule.
    let release = harness.workload_jwt(
        "repo:metio/pasejo:ref:refs/heads/main",
        "pasejo-server",
        600,
    );
    let third: TokenResponse = harness.workload(&self::key(), &release).await.ok();
    assert_ne!(third.user, response.user);
}

#[tokio::test]
async fn workload_tokens_are_checked_thoroughly() {
    let harness = Harness::start().await;
    let key = key();
    let wrong_audience =
        harness.workload_jwt("system:serviceaccount:ci:builder", "kubernetes", 600);
    harness
        .workload(&key, &wrong_audience)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let expired = harness.workload_jwt("system:serviceaccount:ci:builder", "pasejo-server", -600);
    harness
        .workload(&key, &expired)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let unmatched = harness.workload_jwt("system:serviceaccount:prod:app", "pasejo-server", 600);
    harness
        .workload(&key, &unmatched)
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let near_miss = harness.workload_jwt(
        "repo:metio/pasejo:ref:refs/heads/mainx",
        "pasejo-server",
        600,
    );
    harness
        .workload(&key, &near_miss)
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let forged = common::sign_jwt(
        &self::key(),
        "w-1",
        &json!({"iss": common::WORKLOAD_ISSUER, "sub": "system:serviceaccount:ci:x", "aud": "pasejo-server", "exp": 4_102_444_800u64}),
    );
    harness
        .workload(&key, &forged)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let foreign = common::sign_jwt(
        &harness.workload_key,
        "w-1",
        &json!({"iss": "https://elsewhere", "sub": "system:serviceaccount:ci:x", "aud": "pasejo-server", "exp": 4_102_444_800u64}),
    );
    harness
        .workload(&key, &foreign)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let unsigned = format!(
        "{}.{}.",
        b64(br#"{"alg":"none","typ":"JWT"}"#),
        b64(json!({"iss": common::WORKLOAD_ISSUER, "sub": "system:serviceaccount:ci:x", "aud": "pasejo-server", "exp": 4_102_444_800u64}).to_string().as_bytes())
    );
    harness
        .workload(&key, &unsigned)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn a_workload_claim_must_name_the_presented_token() {
    let harness = Harness::start().await;
    let key = key();
    let jwt = harness.workload_jwt("system:serviceaccount:ci:builder", "pasejo-server", 600);
    let other = harness.workload_jwt("system:serviceaccount:ci:other", "pasejo-server", 600);
    let claim = harness.claim(
        &key,
        "workload",
        &b64(&pasejo_protocol::encoding::sha256(other.as_bytes())),
    );
    harness
        .raw(
            Method::POST,
            "/v1/auth/workload",
            Some(
                serde_json::to_value(TokenRequest {
                    claim,
                    jwt: Some(jwt),
                    verifier: None,
                })
                .unwrap(),
            ),
        )
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    let claim = harness.claim(&key, "workload", "");
    harness
        .raw(
            Method::POST,
            "/v1/auth/workload",
            Some(
                serde_json::to_value(TokenRequest {
                    claim,
                    jwt: None,
                    verifier: None,
                })
                .unwrap(),
            ),
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
}

#[tokio::test]
async fn sessions_end_on_logout_and_after_twelve_hours() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    assert_eq!(
        alice.post_empty("/v1/auth/logout").await.status,
        StatusCode::NO_CONTENT
    );
    alice
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let bob = harness.sign_in("bob").await;
    harness.advance(12 * 3_600_000 - 120_000);
    bob.get("/v1/me").await.ok::<Me>();
    harness.advance(121_000);
    bob.get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn requests_need_a_token() {
    let harness = Harness::start().await;
    harness
        .raw(Method::GET, "/v1/me", None)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let alice = harness.sign_in("alice").await;
    let mut forged = alice.clone();
    forged.token = b64(&[7; 32]);
    forged
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn request_proofs_bind_method_path_body_time_nonce_and_token() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let now = harness.now();
    let headers = |proof: pasejo_protocol::request::Proof| {
        vec![
            ("authorization", proof.authorization),
            (TIMESTAMP_HEADER, proof.timestamp),
            (NONCE_HEADER, proof.nonce),
            (PROOF_HEADER, proof.proof),
        ]
    };
    // A proof for another path, method or body does not verify.
    let proof = prove("GET", "/v1/orgs", now, b"", &alice.token, &alice.key);
    alice
        .send_with(Method::GET, "/v1/me", vec![], &headers(proof))
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    let proof = prove("POST", "/v1/me", now, b"", &alice.token, &alice.key);
    alice
        .send_with(Method::GET, "/v1/me", vec![], &headers(proof))
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    let proof = prove(
        "POST",
        "/v1/auth/logout",
        now,
        b"{}",
        &alice.token,
        &alice.key,
    );
    alice
        .send_with(
            Method::POST,
            "/v1/auth/logout",
            b"{ }".to_vec(),
            &headers(proof),
        )
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    // A proof by another key does not verify.
    let proof = prove("GET", "/v1/me", now, b"", &alice.token, &key());
    alice
        .send_with(Method::GET, "/v1/me", vec![], &headers(proof))
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    // Stale and future timestamps are refused.
    for skew in [-61_000i64, 61_000] {
        let proof = prove(
            "GET",
            "/v1/me",
            now.saturating_add_signed(skew),
            b"",
            &alice.token,
            &alice.key,
        );
        alice
            .send_with(Method::GET, "/v1/me", vec![], &headers(proof))
            .await
            .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    }
    // A nonce works once.
    let proof = prove("GET", "/v1/me", now, b"", &alice.token, &alice.key);
    alice
        .send_with(Method::GET, "/v1/me", vec![], &headers(proof.clone()))
        .await
        .ok::<Me>();
    alice
        .send_with(Method::GET, "/v1/me", vec![], &headers(proof))
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    // Missing proof headers are refused.
    let proof = prove("GET", "/v1/me", now, b"", &alice.token, &alice.key);
    alice
        .send_with(
            Method::GET,
            "/v1/me",
            vec![],
            &[("authorization", proof.authorization)],
        )
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
    // The query string is covered.
    let proof = prove("GET", "/v1/me?x=1", now, b"", &alice.token, &alice.key);
    alice
        .send_with(Method::GET, "/v1/me?x=2", vec![], &headers(proof))
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
}

#[tokio::test]
async fn a_stolen_token_is_useless_without_its_key() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let bob = harness.sign_in("bob").await;
    let mut thief = bob.clone();
    thief.token = alice.token.clone();
    thief
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
}

#[tokio::test]
async fn sign_ins_are_audited() {
    let harness = Harness::start().await;
    harness.sign_in("alice").await;
    let events =
        pasejo_server::audit::events(&harness.state, pasejo_server::audit::SERVER_CHAIN, 0, 100)
            .await
            .unwrap();
    let actions: Vec<&str> = events.iter().map(|event| event.action.as_str()).collect();
    for expected in [
        "auth.device",
        "auth.oidc",
        "auth.device.approve",
        "auth.session",
    ] {
        assert!(
            actions.contains(&expected),
            "{expected} missing from {actions:?}"
        );
    }
    let session = events
        .iter()
        .find(|event| event.action == "auth.session")
        .unwrap();
    assert_eq!(session.session_kind.as_deref(), Some("person"));
    assert_eq!(session.ip.as_deref(), Some("127.0.0.1"));
    pasejo_protocol::checkpoint::verify_chain(events[0].prev_hash, events[0].seq, &events).unwrap();
}

const APP_REDIRECT: &str = "wtf.metio.pasejo%3A%2Fauth";

/// A PKCE verifier and its S256 challenge.
fn pkce() -> (String, String) {
    let verifier = b64(&random::<32>());
    let challenge = b64(&sha256(verifier.as_bytes()));
    (verifier, challenge)
}

/// Runs the browser side of an app sign-in and returns the redirect.
async fn app_redirect(harness: &Harness, challenge: &str, redirect: &str, state: &str) -> Reply {
    harness
        .follow_to_callback(&format!(
            "{}/auth/app/start?challenge={challenge}&redirect_uri={redirect}&state={state}",
            harness.url
        ))
        .await
}

fn code_of(location: &str) -> String {
    between(location, "code=", "&").unwrap()
}

async fn exchange_app(
    harness: &Harness,
    key: &SigningKey,
    purpose: &str,
    code: &str,
    verifier: &str,
) -> Reply {
    harness
        .raw(
            Method::POST,
            "/v1/auth/app",
            Some(
                serde_json::to_value(TokenRequest {
                    claim: harness.claim(key, purpose, code),
                    jwt: None,
                    verifier: Some(verifier.into()),
                })
                .unwrap(),
            ),
        )
        .await
}

#[tokio::test]
async fn an_app_sign_in_returns_a_single_use_code_bound_to_its_verifier() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let (verifier, challenge) = pkce();
    let reply = app_redirect(&harness, &challenge, APP_REDIRECT, "s1").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let location = reply.location().unwrap();
    assert!(
        location.starts_with("wtf.metio.pasejo:/auth?code="),
        "{location}"
    );
    assert!(location.ends_with("&state=s1"), "{location}");
    let code = code_of(&location);
    let key = key();
    exchange_app(&harness, &key, "web", &code, &verifier)
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    exchange_app(&harness, &key, "app", &code, "short")
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let response: TokenResponse = exchange_app(&harness, &key, "app", &code, &verifier)
        .await
        .ok();
    let me: Me = harness
        .client(key.clone(), response)
        .get("/v1/me")
        .await
        .ok();
    assert_eq!(me.email, "alice@example.com");
    exchange_app(&harness, &key, "app", &code, &verifier)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let events =
        pasejo_server::audit::events(&harness.state, pasejo_server::audit::SERVER_CHAIN, 0, 1000)
            .await
            .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.action == "auth.oidc" && event.detail.as_deref() == Some("app"))
    );
    assert!(
        events
            .iter()
            .any(|event| event.action == "auth.session" && event.detail.as_deref() == Some("app"))
    );
}

#[tokio::test]
async fn a_wrong_verifier_spends_the_code() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let (verifier, challenge) = pkce();
    let reply = app_redirect(&harness, &challenge, APP_REDIRECT, "s1").await;
    let code = code_of(&reply.location().unwrap());
    let key = key();
    let (other, _) = pkce();
    exchange_app(&harness, &key, "app", &code, &other)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    exchange_app(&harness, &key, "app", &code, &verifier)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn the_session_is_bound_to_the_key_that_signed_the_claim() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let (verifier, challenge) = pkce();
    let reply = app_redirect(&harness, &challenge, APP_REDIRECT, "s1").await;
    let code = code_of(&reply.location().unwrap());
    let key = key();
    let response: TokenResponse = exchange_app(&harness, &key, "app", &code, &verifier)
        .await
        .ok();
    // Another key cannot use the token.
    harness
        .client(self::key(), response)
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "invalid_proof");
}

#[tokio::test]
async fn app_codes_expire_after_ten_minutes() {
    let harness = Harness::start().await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let (verifier, challenge) = pkce();
    let reply = app_redirect(&harness, &challenge, APP_REDIRECT, "s1").await;
    let code = code_of(&reply.location().unwrap());
    harness.advance(601_000);
    exchange_app(&harness, &key(), "app", &code, &verifier)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn an_app_sign_in_refuses_unlisted_redirects_and_malformed_parameters() {
    let harness = Harness::start().await;
    let (_, challenge) = pkce();
    let start = |query: String| {
        let harness = &harness;
        async move {
            harness
                .raw(Method::GET, &format!("/auth/app/start?{query}"), None)
                .await
        }
    };
    for query in [
        format!("challenge={challenge}&redirect_uri=https%3A%2F%2Fevil.example%2F&state=s1"),
        format!("challenge={challenge}&state=s1"),
        format!("challenge=abc&redirect_uri={APP_REDIRECT}&state=s1"),
        format!("redirect_uri={APP_REDIRECT}&state=s1"),
        format!("challenge={challenge}&redirect_uri={APP_REDIRECT}&state=has%20space"),
        format!(
            "challenge={challenge}&redirect_uri={APP_REDIRECT}&state={}",
            "s".repeat(129)
        ),
        format!("challenge={challenge}&redirect_uri={APP_REDIRECT}"),
    ] {
        let reply = start(query.clone()).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{query}");
        assert!(reply.location().is_none(), "{query}");
    }
    let accepted = start(format!(
        "challenge={challenge}&redirect_uri={APP_REDIRECT}&state=s1"
    ))
    .await;
    assert_eq!(accepted.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn configured_redirect_uris_replace_the_default() {
    let harness = Harness::start_with(Options {
        extra: "[apps]\nredirect_uris = [\"https://app.example/callback?from=pasejo\"]\n".into(),
        ..Options::default()
    })
    .await;
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let (verifier, challenge) = pkce();
    let refused = harness
        .raw(
            Method::GET,
            &format!("/auth/app/start?challenge={challenge}&redirect_uri={APP_REDIRECT}&state=s1"),
            None,
        )
        .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    let reply = app_redirect(
        &harness,
        &challenge,
        "https%3A%2F%2Fapp.example%2Fcallback%3Ffrom%3Dpasejo",
        "s2",
    )
    .await;
    let location = reply.location().unwrap();
    assert!(
        location.starts_with("https://app.example/callback?from=pasejo&code="),
        "{location}"
    );
    let code = code_of(&location);
    let _: TokenResponse = exchange_app(&harness, &key(), "app", &code, &verifier)
        .await
        .ok();
}
