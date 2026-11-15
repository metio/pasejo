// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The audit trail: hash chains, checkpoints, co-signatures, access to
//! events, refused requests, pruning and the webhook sink.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::routing::post;
use common::{Harness, Options};
use pasejo_protocol::Id;
use pasejo_protocol::api::{AuditPage, Checkpoints, Cosign, ServerInfo};
use pasejo_protocol::checkpoint::verify_chain;
use pasejo_protocol::encoding::{Fixed, Hash};
use pasejo_protocol::envelope::{Context, Signature, sign};
use pasejo_protocol::log::{Body, RoleChange, Settings};
use pasejo_server::audit::{self, Event, SERVER_CHAIN};
use reqwest::{Method, StatusCode};
use serde_json::Value;

fn checkpoints_every(count: u64) -> Options {
    Options {
        extra: format!("[audit]\ncheckpoint_every = {count}\n"),
        ..Options::default()
    }
}

#[tokio::test]
async fn every_chain_verifies_from_its_first_event() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    for _ in 0..5 {
        alice
            .append(
                org,
                Body::StoreCreate {
                    store: Id::random(),
                },
            )
            .await
            .ok::<Value>();
    }
    for chain in [SERVER_CHAIN.to_owned(), org.to_string()] {
        let events = audit::events(&harness.state, &chain, 0, 1000)
            .await
            .unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[0].prev_hash, Hash::default());
        verify_chain(Hash::default(), 1, &events).unwrap();
        // Altering any event breaks the chain.
        let mut altered = events.clone();
        altered[0].detail = Some("rewritten".into());
        assert!(verify_chain(Hash::default(), 1, &altered).is_err());
        let mut dropped = events.clone();
        dropped.remove(0);
        assert!(verify_chain(Hash::default(), 1, &dropped).is_err());
    }
}

#[tokio::test]
async fn checkpoints_are_signed_by_the_server_key() {
    let harness = Harness::start_with(checkpoints_every(3)).await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    for _ in 0..6 {
        alice
            .append(
                org,
                Body::StoreCreate {
                    store: Id::random(),
                },
            )
            .await
            .ok::<Value>();
    }
    let info: ServerInfo = harness.raw(Method::GET, "/v1/server", None).await.ok();
    let listed: Checkpoints = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit/checkpoints"))
        .await
        .ok();
    assert!(
        listed.checkpoints.len() >= 2,
        "{}",
        listed.checkpoints.len()
    );
    let events = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    let log = alice.log(org).await;
    for entry in &listed.checkpoints {
        let payload = entry
            .checkpoint
            .verify_server(&info.checkpoint_key.0)
            .unwrap();
        assert_eq!(payload.org, Some(org));
        assert_eq!(payload.seq, entry.seq);
        assert_eq!(
            payload.hash,
            events[usize::try_from(entry.seq).unwrap() - 1].hash
        );
        assert!(log.seq_of(&payload.log_head.unwrap()).is_some());
        assert!(entry.checkpoint.verify_server(&[9; 32]).is_err());
    }
    // Paging after a sequence number.
    let after: Checkpoints = alice
        .client
        .get(&format!(
            "/v1/orgs/{org}/audit/checkpoints?since={}",
            listed.checkpoints[0].seq
        ))
        .await
        .ok();
    assert_eq!(after.checkpoints.len(), listed.checkpoints.len() - 1);
}

#[tokio::test]
async fn idle_chains_get_an_hourly_checkpoint() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    assert_eq!(audit::checkpoint_due(&harness.state).await.unwrap(), 0);
    harness.advance(3_600_001);
    // The server chain and the organization's chain.
    assert_eq!(audit::checkpoint_due(&harness.state).await.unwrap(), 2);
    assert_eq!(audit::checkpoint_due(&harness.state).await.unwrap(), 0);
    let listed: Checkpoints = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit/checkpoints"))
        .await
        .ok();
    assert_eq!(listed.checkpoints.len(), 1);
    let server = audit::checkpoints(&harness.state, SERVER_CHAIN, 0)
        .await
        .unwrap();
    let payload = server[0]
        .1
        .verify_server(&harness.state.keys.checkpoint.verifying_key().to_bytes())
        .unwrap();
    assert_eq!(payload.org, None);
    assert_eq!(payload.log_head, None);
}

#[tokio::test]
async fn devices_co_sign_checkpoints() {
    let harness = Harness::start_with(checkpoints_every(2)).await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let listed: Checkpoints = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit/checkpoints"))
        .await
        .ok();
    let first = &listed.checkpoints[0];
    let signature = Signature {
        device: alice.record.id,
        signature: Fixed(sign(
            Context::Checkpoint,
            &first.checkpoint.payload.0,
            &alice.client.key,
        )),
    };
    let path = format!("/v1/orgs/{org}/audit/checkpoints/{}/cosign", first.seq);
    assert_eq!(
        alice
            .client
            .post(
                &path,
                &Cosign {
                    signature: signature.clone()
                }
            )
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    // A co-signature under another device's name, or by the wrong key, is refused.
    bob.client
        .post(&path, &Cosign { signature })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let wrong = Signature {
        device: bob.record.id,
        signature: Fixed(sign(
            Context::Checkpoint,
            &first.checkpoint.payload.0,
            &alice.client.key,
        )),
    };
    bob.client
        .post(&path, &Cosign { signature: wrong })
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let other_context = Signature {
        device: bob.record.id,
        signature: Fixed(sign(
            Context::LogEntry,
            &first.checkpoint.payload.0,
            &bob.client.key,
        )),
    };
    bob.client
        .post(
            &path,
            &Cosign {
                signature: other_context,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    bob.client
        .post(
            &format!("/v1/orgs/{org}/audit/checkpoints/9999/cosign"),
            &Cosign {
                signature: Signature {
                    device: bob.record.id,
                    signature: Fixed([0; 64]),
                },
            },
        )
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");

    let listed: Checkpoints = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit/checkpoints"))
        .await
        .ok();
    let cosigned = &listed.checkpoints[0].checkpoint;
    cosigned
        .verify_cosignature(alice.record.id, &alice.record.signing_key.0)
        .unwrap();
    assert!(
        cosigned
            .verify_cosignature(bob.record.id, &bob.record.signing_key.0)
            .is_err()
    );
    let events = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.action == "checkpoint.cosign")
    );
}

#[tokio::test]
async fn only_admins_and_auditors_read_events() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    harness.join(&alice, &carol, org, RoleChange::Auditor).await;
    bob.client
        .get(&format!("/v1/orgs/{org}/audit"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let page: AuditPage = carol
        .client
        .get(&format!("/v1/orgs/{org}/audit"))
        .await
        .ok();
    assert!(!page.events.is_empty());
    verify_chain(Hash::default(), 1, &page.events).unwrap();
    let admin: AuditPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit"))
        .await
        .ok();
    assert!(
        admin.events.len() > page.events.len(),
        "reading the audit log is audited itself"
    );
    assert!(
        admin
            .events
            .iter()
            .any(|event| event.action == "audit.read" && event.actor_user == Some(carol.user()))
    );
    // People cannot read the server's chain.
    alice
        .client
        .get("/v1/audit")
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn audit_pages_continue_where_they_stopped() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    for _ in 0..7 {
        alice
            .append(
                org,
                Body::StoreCreate {
                    store: Id::random(),
                },
            )
            .await
            .ok::<Value>();
    }
    let first: AuditPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit?limit=3"))
        .await
        .ok();
    assert_eq!(first.events.len(), 3);
    assert_eq!(first.next, Some(3));
    let second: AuditPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit?since=3&limit=3"))
        .await
        .ok();
    assert_eq!(second.events[0].seq, 4);
    let mut all = first.events.clone();
    all.extend(second.events);
    verify_chain(Hash::default(), 1, &all).unwrap();
    let rest: AuditPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/audit?since=6"))
        .await
        .ok();
    assert_eq!(rest.next, None);
}

#[tokio::test]
async fn refused_requests_are_audited() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    bob.client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
    harness
        .raw(Method::GET, "/v1/me", None)
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    harness.raw(Method::GET, "/wp-login.php", None).await;
    let events = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    let refused = events
        .iter()
        .find(|event| event.action == "request.refused")
        .unwrap();
    assert_eq!(refused.outcome, "denied");
    assert_eq!(refused.actor_user, Some(bob.user()));
    assert!(
        refused
            .detail
            .as_deref()
            .unwrap()
            .contains("403 pending_device"),
        "{:?}",
        refused.detail
    );
    let server = audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
        .await
        .unwrap();
    assert!(server.iter().any(|event| event.action == "request.refused"
        && event.detail.as_deref().unwrap().contains("/v1/me 401")));
    assert!(!server.iter().any(|event| {
        event
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("wp-login"))
    }));
    // Requests for unknown organizations land in the server chain.
    let stranger = Id::random();
    bob.client
        .get(&format!("/v1/orgs/{stranger}/log"))
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_org");
    assert!(
        audit::events(&harness.state, &stranger.to_string(), 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn pruning_keeps_the_chain_verifiable_from_a_checkpoint() {
    let harness = Harness::start_with(checkpoints_every(4)).await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    for _ in 0..6 {
        alice
            .append(
                org,
                Body::StoreCreate {
                    store: Id::random(),
                },
            )
            .await
            .ok::<Value>();
    }
    harness.advance(10 * 86_400_000);
    audit::record(&harness.state, None, Event::ok("test.event").org(org))
        .await
        .unwrap();
    let before = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    let removed = audit::prune(&harness.state, harness.now() - 86_400_000)
        .await
        .unwrap();
    assert!(removed > 0);
    let after = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(after.len() < before.len());
    let checkpoints = audit::checkpoints(&harness.state, &org.to_string(), 0)
        .await
        .unwrap();
    let (seq, checkpoint) = &checkpoints[0];
    let payload = checkpoint
        .verify_server(&harness.state.keys.checkpoint.verifying_key().to_bytes())
        .unwrap();
    assert_eq!(after[0].seq, seq + 1);
    verify_chain(payload.hash, seq + 1, &after).unwrap();
}

#[tokio::test]
async fn retention_runs_with_maintenance() {
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 2\nretention_days = 1\n".into(),
        ..Options::default()
    })
    .await;
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
    let before = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap()
        .len();
    harness.advance(2 * 86_400_000);
    pasejo_server::app::maintain(&harness.state).await.unwrap();
    let after = audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap()
        .len();
    assert!(after < before, "{after} < {before}");
}

#[derive(Default)]
struct Received {
    requests: Vec<(HeaderMap, Bytes)>,
    fail: bool,
}

async fn receiver() -> (String, Arc<Mutex<Received>>) {
    let received = Arc::new(Mutex::new(Received::default()));
    let state = Arc::clone(&received);
    let router = Router::new().route(
        "/hook",
        post(move |headers: HeaderMap, body: Bytes| {
            let state = Arc::clone(&state);
            async move {
                let mut received = state.lock().unwrap();
                if received.fail {
                    return StatusCode::INTERNAL_SERVER_ERROR;
                }
                received.requests.push((headers, body));
                StatusCode::OK
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (format!("http://{address}/hook"), received)
}

#[tokio::test]
async fn the_webhook_receives_signed_batches_and_keeps_its_position() {
    let (url, received) = receiver().await;
    let secret_dir = tempfile::tempdir().unwrap();
    let secret_file = secret_dir.path().join("secret");
    std::fs::write(&secret_file, "s3cret\n").unwrap();
    let harness = Harness::start_with(Options {
        extra: format!(
            "[audit.webhook]\nurl = \"{url}\"\nsecret_file = \"{}\"\nbatch = 2\n",
            secret_file.display()
        ),
        ..Options::default()
    })
    .await;
    harness.sign_in("alice").await;
    let mut delivered = 0;
    loop {
        let count = audit::deliver_webhook(&harness.state, b"s3cret")
            .await
            .unwrap();
        if count == 0 {
            break;
        }
        delivered += count;
    }
    let total = audit::events(&harness.state, SERVER_CHAIN, 0, 1000)
        .await
        .unwrap()
        .len();
    assert_eq!(delivered, total);
    let requests = std::mem::take(&mut received.lock().unwrap().requests);
    let mut seen = Vec::new();
    for (headers, body) in &requests {
        let timestamp: u64 = headers["pasejo-timestamp"]
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            headers["pasejo-signature"].to_str().unwrap(),
            audit::webhook_signature(b"s3cret", timestamp, body)
        );
        let batch: Value = serde_json::from_slice(body).unwrap();
        assert!(batch["events"].as_array().unwrap().len() <= 2);
        seen.extend(
            batch["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| event["seq"].as_u64().unwrap()),
        );
    }
    assert_eq!(seen, (1..=total as u64).collect::<Vec<_>>());
    // A failing receiver keeps the position; nothing is skipped.
    audit::record(&harness.state, None, Event::ok("test.event"))
        .await
        .unwrap();
    received.lock().unwrap().fail = true;
    assert!(
        audit::deliver_webhook(&harness.state, b"s3cret")
            .await
            .is_err()
    );
    received.lock().unwrap().fail = false;
    assert_eq!(
        audit::deliver_webhook(&harness.state, b"s3cret")
            .await
            .unwrap(),
        1
    );
    let last: Value = serde_json::from_slice(&received.lock().unwrap().requests[0].1).unwrap();
    assert_eq!(last["events"][0]["action"], "test.event");
}

#[tokio::test]
async fn break_glass_logins_raise_a_webhook_alert() {
    let (url, received) = receiver().await;
    let secret_dir = tempfile::tempdir().unwrap();
    let secret_file = secret_dir.path().join("secret");
    std::fs::write(&secret_file, "s3cret").unwrap();
    let harness = Harness::start_with(Options {
        extra: format!(
            "[audit.webhook]\nurl = \"{url}\"\nsecret_file = \"{}\"\n",
            secret_file.display()
        ),
        ..Options::default()
    })
    .await;
    audit::record(&harness.state, None, Event::ok("break-glass.login"))
        .await
        .unwrap();
    audit::deliver_webhook(&harness.state, b"s3cret")
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&received.lock().unwrap().requests[0].1).unwrap();
    assert_eq!(body["alert"], true);
    assert_eq!(body["chain"], SERVER_CHAIN);
}
