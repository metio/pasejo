// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The break-glass account and the emergency sign-in window.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Client, Harness, Options, totp_now};
use ed25519_dalek::SigningKey;
use pasejo_protocol::api::{
    AuditPage, BreakGlassLogin, BreakGlassToken, Checkpoints, EmergencyState, EmergencyWindow, Me,
    OrgList, ServerInfo, TokenRequest, TokenResponse, Users,
};
use pasejo_protocol::encoding::Fixed;
use pasejo_protocol::log::Settings;
use pasejo_server::audit::{self, SERVER_CHAIN};
use pasejo_server::auth::break_glass::{self, Created};
use rand_core::OsRng;
use reqwest::{Method, StatusCode};
use serde_json::Value;

async fn account(harness: &Harness) -> Created {
    break_glass::create(&harness.state, "admin").await.unwrap()
}

async fn login(
    harness: &Harness,
    key: &SigningKey,
    username: &str,
    password: &str,
    totp: &str,
) -> common::Reply {
    harness
        .raw(
            Method::POST,
            "/v1/break-glass/login",
            Some(
                serde_json::to_value(BreakGlassLogin {
                    username: username.into(),
                    password: password.into(),
                    totp: totp.into(),
                    signing_key: Fixed(key.verifying_key().to_bytes()),
                })
                .unwrap(),
            ),
        )
        .await
}

async fn session(harness: &Harness, created: &Created) -> Client {
    let key = SigningKey::generate(&mut OsRng);
    let token: BreakGlassToken = login(
        harness,
        &key,
        "admin",
        &created.password,
        &totp_now(harness, &created.totp_secret, 0),
    )
    .await
    .ok();
    harness.client(
        key,
        TokenResponse {
            token: token.token,
            expires: token.expires,
            user: None,
        },
    )
}

#[tokio::test]
async fn creation_prints_strong_credentials_once() {
    let harness = Harness::start().await;
    let created = account(&harness).await;
    assert_eq!(created.password.len(), 32);
    assert!(
        created
            .totp_uri
            .starts_with("otpauth://totp/127.0.0.1:admin?secret=")
    );
    assert_eq!(common::base32_decode(&created.totp_secret).len(), 20);
    // The database holds neither in clear.
    let row: (String, Vec<u8>) = sqlx::query_as("SELECT password_hash, totp FROM break_glass")
        .fetch_one(harness.state.db.pool())
        .await
        .unwrap();
    assert!(row.0.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"));
    assert!(!row.0.contains(created.password.as_str()));
    assert!(
        !row.1
            .windows(20)
            .any(|window| window == common::base32_decode(&created.totp_secret))
    );
}

#[tokio::test]
async fn a_login_needs_password_and_current_code() {
    let harness = Harness::start().await;
    let created = account(&harness).await;
    let key = SigningKey::generate(&mut OsRng);
    let code = totp_now(&harness, &created.totp_secret, 0);
    login(&harness, &key, "admin", "wrong", &code)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    login(&harness, &key, "admin", &created.password, "000000")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    login(&harness, &key, "nobody", &created.password, &code)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let stale = totp_now(&harness, &created.totp_secret, -3);
    login(&harness, &key, "admin", &created.password, &stale)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let before = harness.now();
    let token: BreakGlassToken = login(&harness, &key, "admin", &created.password, &code)
        .await
        .ok();
    assert!((before + 15 * 60_000..=harness.now() + 15 * 60_000).contains(&token.expires));
    // The same code is not accepted twice.
    login(&harness, &key, "admin", &created.password, &code)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    // The next step's code is.
    harness.advance(30_000);
    login(
        &harness,
        &key,
        "admin",
        &created.password,
        &totp_now(&harness, &created.totp_secret, 0),
    )
    .await
    .ok::<BreakGlassToken>();
}

#[tokio::test]
async fn five_failures_lock_the_account_for_fifteen_minutes() {
    let harness = Harness::start().await;
    let created = account(&harness).await;
    let key = SigningKey::generate(&mut OsRng);
    for _ in 0..5 {
        login(&harness, &key, "admin", "wrong", "000000")
            .await
            .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    let code = totp_now(&harness, &created.totp_secret, 0);
    login(&harness, &key, "admin", &created.password, &code)
        .await
        .expect(StatusCode::LOCKED, "locked");
    harness.advance(15 * 60_000 + 1);
    login(
        &harness,
        &key,
        "admin",
        &created.password,
        &totp_now(&harness, &created.totp_secret, 0),
    )
    .await
    .ok::<BreakGlassToken>();
}

#[tokio::test]
async fn break_glass_is_off_unless_enabled() {
    let harness = Harness::start_with(Options {
        break_glass: false,
        ..Options::default()
    })
    .await;
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    assert!(!info.break_glass);
    let created = account(&harness).await;
    let key = SigningKey::generate(&mut OsRng);
    login(
        &harness,
        &key,
        "admin",
        &created.password,
        &totp_now(&harness, &created.totp_secret, 0),
    )
    .await
    .expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn logins_come_only_from_allowed_networks() {
    let harness = Harness::start_with(Options {
        break_glass: false,
        extra: "[break_glass]\nenabled = true\nnetworks = [\"10.0.0.0/8\"]\n".into(),
        ..Options::default()
    })
    .await;
    let created = account(&harness).await;
    let key = SigningKey::generate(&mut OsRng);
    login(
        &harness,
        &key,
        "admin",
        &created.password,
        &totp_now(&harness, &created.totp_secret, 0),
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    let events = audit::events(&harness.state, SERVER_CHAIN, 0, 100)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.action == "break-glass.login"
                && event.detail.as_deref() == Some("network not allowed"))
    );
}

#[tokio::test]
async fn a_disabled_account_cannot_log_in() {
    let harness = Harness::start().await;
    let created = account(&harness).await;
    break_glass::disable(&harness.state, "admin").await.unwrap();
    assert!(
        break_glass::disable(&harness.state, "nobody")
            .await
            .is_err()
    );
    let key = SigningKey::generate(&mut OsRng);
    login(
        &harness,
        &key,
        "admin",
        &created.password,
        &totp_now(&harness, &created.totp_secret, 0),
    )
    .await
    .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    // Creating it again replaces the credentials and enables it.
    let renewed = account(&harness).await;
    assert_ne!(renewed.password, created.password);
    login(
        &harness,
        &key,
        "admin",
        &renewed.password,
        &totp_now(&harness, &renewed.totp_secret, 0),
    )
    .await
    .ok::<BreakGlassToken>();
}

#[tokio::test]
async fn sessions_last_fifteen_minutes() {
    let harness = Harness::start().await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    admin.get("/v1/break-glass/users").await.ok::<Users>();
    harness.advance(15 * 60_000 + 1);
    admin
        .get("/v1/break-glass/users")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn break_glass_manages_users_but_reads_no_organization() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    let orgs: OrgList = admin.get("/v1/break-glass/orgs").await.ok();
    assert_eq!(orgs.orgs.len(), 1);
    let users: Users = admin.get("/v1/break-glass/users").await.ok();
    assert!(
        users
            .users
            .iter()
            .any(|user| user.email == "alice@example.com")
    );
    // No user, so no organization access and no log entries.
    admin
        .get("/v1/me")
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    admin
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
    admin
        .get(&format!("/v1/orgs/{org}/log"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    // It reads audit trails.
    let page: AuditPage = admin.get(&format!("/v1/orgs/{org}/audit")).await.ok();
    assert!(!page.events.is_empty());
    let server: AuditPage = admin.get("/v1/audit").await.ok();
    assert!(
        server
            .events
            .iter()
            .any(|event| event.action == "break-glass.login" && event.outcome == "ok")
    );
    // People cannot use break-glass endpoints.
    alice
        .client
        .get("/v1/break-glass/users")
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn suspending_a_user_ends_their_sessions_and_sign_ins() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    let user = alice.user.unwrap();
    assert_eq!(
        admin
            .post_empty(&format!("/v1/break-glass/users/{user}/suspend"))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    alice
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    // A new sign-in is refused at the callback.
    let key = SigningKey::generate(&mut OsRng);
    let authorization: pasejo_protocol::api::DeviceAuthorization = harness
        .raw(Method::POST, "/v1/auth/device", Some(serde_json::json!({"signing_key": pasejo_protocol::encoding::b64(key.verifying_key().as_bytes()), "label": "x"})))
        .await
        .ok();
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let page = harness
        .follow_to_callback(&authorization.verification_uri_complete)
        .await;
    assert_eq!(page.status, StatusCode::BAD_REQUEST);
    assert!(page.text().contains("suspended"));
    // Reactivation lets them back in.
    assert_eq!(
        admin
            .post_empty(&format!("/v1/break-glass/users/{user}/reactivate"))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let again = harness.sign_in("alice").await;
    again.get("/v1/me").await.ok::<Me>();
    admin
        .post_empty(&format!(
            "/v1/break-glass/users/{}/suspend",
            pasejo_protocol::Id::random()
        ))
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn ending_sessions_keeps_the_account() {
    let harness = Harness::start().await;
    let alice = harness.sign_in("alice").await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    admin
        .post_empty(&format!(
            "/v1/break-glass/users/{}/sessions/revoke",
            alice.user.unwrap()
        ))
        .await;
    alice
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    harness
        .sign_in("alice")
        .await
        .get("/v1/me")
        .await
        .ok::<Me>();
}

#[tokio::test]
async fn the_emergency_window_admits_existing_devices_without_sso() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    harness.org(&alice, Settings::default()).await;
    let stranger = SigningKey::generate(&mut OsRng);
    let emergency = |key: &SigningKey| {
        let claim = harness.claim(key, "emergency", "");
        harness.raw(
            Method::POST,
            "/v1/auth/emergency",
            Some(serde_json::to_value(TokenRequest { claim, jwt: None }).unwrap()),
        )
    };
    emergency(&alice.client.key)
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    admin
        .post(
            "/v1/break-glass/emergency",
            &EmergencyWindow {
                duration: 24 * 3_600_000 + 1,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let before = harness.now();
    let opened: EmergencyState = admin
        .post(
            "/v1/break-glass/emergency",
            &EmergencyWindow {
                duration: 30 * 60_000,
            },
        )
        .await
        .ok();
    assert!((before + 30 * 60_000..=harness.now() + 30 * 60_000).contains(&opened.until.unwrap()));
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    assert_eq!(info.emergency_until, opened.until);

    let response: TokenResponse = emergency(&alice.client.key).await.ok();
    assert_eq!(response.user, alice.client.user);
    // The session ends with the window, not after an hour.
    assert!(
        response.expires <= opened.until.unwrap(),
        "{} <= {:?}",
        response.expires,
        opened.until
    );
    emergency(&stranger)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    let rescued = harness.client(alice.client.key.clone(), response);
    rescued.get("/v1/me").await.ok::<Me>();

    // Closing the window ends emergency sessions.
    let closed: EmergencyState = admin
        .post(
            "/v1/break-glass/emergency",
            &EmergencyWindow { duration: 0 },
        )
        .await
        .ok();
    assert_eq!(closed.until, None);
    rescued
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    alice.client.get("/v1/me").await.ok::<Me>();
    emergency(&alice.client.key)
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let events = audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
        .await
        .unwrap();
    for action in ["emergency.open", "auth.emergency", "emergency.close"] {
        assert!(
            events.iter().any(|event| event.action == action),
            "{action}"
        );
    }
}

#[tokio::test]
async fn emergency_windows_expire() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    harness.org(&alice, Settings::default()).await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    admin
        .post(
            "/v1/break-glass/emergency",
            &EmergencyWindow { duration: 60_000 },
        )
        .await
        .ok::<Value>();
    harness.advance(61_000);
    let claim = harness.claim(&alice.client.key, "emergency", "");
    harness
        .raw(
            Method::POST,
            "/v1/auth/emergency",
            Some(serde_json::to_value(TokenRequest { claim, jwt: None }).unwrap()),
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn break_glass_reads_signed_checkpoints_of_the_server_chain() {
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 2\n".into(),
        ..Options::default()
    })
    .await;
    let alice = harness.sign_in("alice").await;
    let created = account(&harness).await;
    let admin = session(&harness, &created).await;
    for _ in 0..3 {
        admin.get("/v1/break-glass/users").await.ok::<Value>();
    }
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    let listed: Checkpoints = admin.get("/v1/audit/checkpoints").await.ok();
    assert!(
        listed.checkpoints.len() >= 2,
        "{}",
        listed.checkpoints.len()
    );
    let events = audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
        .await
        .unwrap();
    for entry in &listed.checkpoints {
        let payload = entry
            .checkpoint
            .verify_server(&info.checkpoint_key.0)
            .unwrap();
        assert_eq!(payload.org, None);
        assert_eq!(payload.log_head, None);
        assert_eq!(payload.seq, entry.seq);
        assert_eq!(
            payload.hash,
            events[usize::try_from(entry.seq).unwrap() - 1].hash
        );
        assert_eq!(entry.checkpoint.cosignatures, []);
    }
    let first = listed.checkpoints[0].seq;
    let after: Checkpoints = admin
        .get(&format!("/v1/audit/checkpoints?since={first}"))
        .await
        .ok();
    assert!(after.checkpoints.iter().all(|entry| entry.seq > first));
    assert_eq!(after.checkpoints.len(), listed.checkpoints.len() - 1);
    // People cannot read the server chain's checkpoints.
    alice
        .get("/v1/audit/checkpoints")
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}
