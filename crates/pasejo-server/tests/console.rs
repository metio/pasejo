// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The browser console's modules, run by Node against the real server.
//! Without Node the test is skipped locally and fails when `CI` is set.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use std::path::Path;
use std::process::Command;

use common::stores::{Target, Write, commit, draft};
use common::{Harness, Member, Options, totp_now};
use ed25519_dalek::SigningKey;
use pasejo_protocol::Id;
use pasejo_protocol::api::{Committed, LogPage};
use pasejo_protocol::crypto::EpochKeys;
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::{Fixed, b64};
use pasejo_protocol::log::{Body, RoleChange, Settings};
use pasejo_protocol::store::IndexPlain;
use pasejo_protocol::wrapping::hpke_keypair;
use rand_core::{OsRng, RngCore};
use reqwest::StatusCode;
use serde_json::{Value, json};

fn node() -> Option<String> {
    let node = std::env::var("PASEJO_NODE").unwrap_or_else(|_| "node".into());
    let found = Command::new(&node)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if found {
        return Some(node);
    }
    assert!(
        std::env::var_os("CI").is_none(),
        "Node is required to test the console in CI"
    );
    eprintln!("skipping the console test: Node is not installed");
    None
}

#[tokio::test]
async fn the_console_signs_in_opens_stores_approves_devices_and_verifies_the_audit_log() {
    let Some(node) = node() else {
        return;
    };
    // Frequent checkpoints, so the break-glass page verifies signed ones.
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 2\n".into(),
        ..Options::default()
    })
    .await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;

    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let browser_key = SigningKey::from_bytes(&seed);
    let (hpke_private, hpke_public) = hpke_keypair();
    let browser = DeviceRecord {
        id: Id::random(),
        user: alice.user(),
        kind: DeviceKind::Hpke,
        recipient: b64(&hpke_public),
        signing_key: Fixed(browser_key.verifying_key().to_bytes()),
        label: "Firefox console".into(),
    };
    alice
        .append(
            org,
            Body::DeviceAdd {
                device: browser.clone(),
            },
        )
        .await
        .ok::<Value>();

    let store = alice.create_store(org).await;
    let keys = EpochKeys::generate();
    let uploaded = alice
        .start_epoch(
            org,
            store,
            0,
            &keys,
            None,
            &[Target::Age(&alice), Target::Hpke(browser.id, hpke_public)],
        )
        .await;
    assert_eq!(
        uploaded.status,
        StatusCode::NO_CONTENT,
        "{}",
        uploaded.text()
    );
    let base = IndexPlain {
        label: "Production — prod".into(),
        ..IndexPlain::default()
    };
    let version = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &base,
        &[Write {
            name: "db/password",
            id: Id::random(),
            version: 1,
            value: "hunter2",
        }],
        &[],
    )
    .await;
    commit(&alice, org, store, &version.sign(&alice))
        .await
        .ok::<Committed>();

    let phone = Member::new(harness.sign_in("bob").await);
    phone.enroll(org).await.ok::<Value>();

    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let reply = harness
        .follow_to_callback(&format!("{}/auth/web/start", harness.url))
        .await;
    let cookie = reply
        .header("set-cookie")
        .expect("a console sign-in cookie")
        .split(';')
        .next()
        .unwrap()
        .to_owned();

    let break_glass = pasejo_server::auth::break_glass::create(&harness.state, "operator")
        .await
        .unwrap();
    let config = json!({
        "url": harness.url,
        "break_glass": {
            "username": "operator",
            "password": break_glass.password.as_str(),
            "totp": totp_now(&harness, &break_glass.totp_secret, 0),
        },
        "cookie": cookie,
        "org": org,
        "store": store,
        "label": "Production — prod",
        "bob": bob.user(),
        "approve": phone.record.id,
        "device": {
            "id": browser.id,
            "signing_seed": b64(&seed),
            "hpke_private": b64(&hpke_private),
            "hpke_public": b64(&hpke_public),
        },
    });
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("web/tests/e2e.js");
    let output = tokio::task::spawn_blocking(move || {
        Command::new(node)
            .arg(script)
            .env("PASEJO_CONSOLE_E2E", config.to_string())
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the console failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("signed out"), "{stdout}");
    assert!(
        stdout.contains("verified the server audit chain"),
        "{stdout}"
    );

    let log = alice.log(org).await;
    let approved = log.state().devices.get(&phone.record.id).unwrap();
    assert!(approved.active);
    let page: LogPage = alice.client.get(&format!("/v1/orgs/{org}/log")).await.ok();
    let last = page.entries.last().unwrap();
    assert_eq!(last.signatures[0].device, browser.id);
}
