// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Remote stores: epochs, wrappings, the index, objects, commits and
//! rotation lists.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use common::stores::{Target, Write, commit, draft, wrappings};
use common::{Harness, Member};
use pasejo_protocol::Id;
use pasejo_protocol::api::{
    Committed, IndexResponse, Keys, LookupResponse, ObjectRef, ObjectsRequest, ObjectsResponse,
    Rotation, Stores, WrappingsUpload,
};
use pasejo_protocol::crypto::{EpochKeys, check_commitments, open_chain};
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::{Bytes, Fixed, Hash, b64};
use pasejo_protocol::log::{Body, Principal, RoleChange, Settings, StoreRole, StoreRoleChange};
use pasejo_protocol::store::{IndexPlain, ObjectPlain, open_index, open_object};
use pasejo_protocol::wrapping::{hpke_keypair, unwrap_age, unwrap_hpke};
use reqwest::{Method, StatusCode};
use serde_json::Value;

struct Setup {
    harness: Harness,
    alice: Member,
    org: Id,
    store: Id,
    keys: EpochKeys,
}

/// Alice founds an organization and creates a store in epoch 0.
async fn setup() -> Setup {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let store = alice.create_store(org).await;
    let keys = EpochKeys::generate();
    assert_eq!(
        alice
            .start_epoch(org, store, 0, &keys, None, &[Target::Age(&alice)])
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    Setup {
        harness,
        alice,
        org,
        store,
        keys,
    }
}

fn path(org: Id, store: Id, rest: &str) -> String {
    format!("/v1/orgs/{org}/stores/{store}/{rest}")
}

async fn grant(admin: &Member, org: Id, store: Id, user: Id, role: StoreRoleChange) {
    admin
        .append(
            org,
            Body::StoreGrant {
                store,
                principal: Principal::User(user),
                role,
                expires: None,
            },
        )
        .await
        .ok::<Value>();
}

async fn wrap_for(
    admin: &Member,
    org: Id,
    store: Id,
    epoch: u64,
    keys: &EpochKeys,
    targets: &[Target<'_>],
) {
    let reply = admin
        .client
        .post(
            &path(org, store, "wrappings"),
            &WrappingsUpload {
                epoch,
                wrappings: wrappings(store, epoch, keys, targets),
            },
        )
        .await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
}

#[tokio::test]
async fn a_written_secret_reaches_another_reader() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let id = Id::random();
    let draft = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "db/prod",
            id,
            version: 1,
            value: "hunter2",
        }],
        &[],
    )
    .await;
    let committed: Committed = commit(&alice, org, store, &draft.sign(&alice)).await.ok();
    assert_eq!(committed.version, 1);

    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    grant(&alice, org, store, bob.user(), StoreRoleChange::Reader).await;
    wrap_for(&alice, org, store, 0, &keys, &[Target::Age(&bob)]).await;

    // Bob unwraps his keys and checks them against the log's commitments.
    let fetched: Keys = bob.client.get(&path(org, store, "keys")).await.ok();
    assert_eq!(fetched.wrappings.len(), 1);
    let unwrapped = unwrap_age(
        store,
        0,
        &fetched.wrappings[0].ciphertext.0,
        &bob.identities(),
    )
    .unwrap();
    let log = bob.log(org).await;
    let record = log.epoch(store, 0).unwrap();
    check_commitments(
        store,
        0,
        &unwrapped,
        &record.names_commit,
        &record.data_commit,
    )
    .unwrap();

    let index: IndexResponse = bob.client.get(&path(org, store, "index")).await.ok();
    let envelope = index.index.unwrap();
    let payload = log.check_index(&envelope).unwrap();
    let plain = open_index(
        store,
        0,
        payload.version,
        &unwrapped.names,
        &payload.index.0,
    )
    .unwrap();
    let entry = &plain.secrets["db/prod"];
    let objects: ObjectsResponse = bob
        .client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest {
                objects: vec![ObjectRef {
                    id: entry.id,
                    version: entry.version,
                }],
            },
        )
        .await
        .ok();
    assert_eq!(Hash::of(&objects.objects[0].ciphertext.0), entry.hash);
    let opened = open_object(
        store,
        0,
        entry.id,
        entry.version,
        unwrapped.data().unwrap(),
        &objects.objects[0].ciphertext.0,
    )
    .unwrap();
    assert!(matches!(&opened, ObjectPlain::Secret { secret } if secret == "hunter2"));

    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    let read = events
        .iter()
        .find(|event| event.action == "object.read")
        .unwrap();
    assert_eq!(
        (read.actor_user, read.store, read.object, read.version),
        (Some(bob.user()), Some(store), Some(id), Some(1))
    );
    assert!(events.iter().any(|event| event.action == "object.write"
        && event.object == Some(id)
        && event.actor_user == Some(alice.user())));
    assert!(
        events
            .iter()
            .any(|event| event.action == "keys.read" && event.actor_user == Some(bob.user()))
    );
    assert!(
        events
            .iter()
            .any(|event| event.action == "index.read" && event.actor_user == Some(bob.user()))
    );
}

#[tokio::test]
async fn the_index_answers_not_modified_for_the_current_version() {
    let Setup {
        alice,
        org,
        store,
        keys,
        harness: _harness,
        ..
    } = setup().await;
    let empty: IndexResponse = alice.client.get(&path(org, store, "index")).await.ok();
    assert!(empty.index.is_none());
    let d = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .ok::<Committed>();
    let reply = alice.client.get(&path(org, store, "index")).await;
    assert_eq!(reply.header("etag").as_deref(), Some("\"1\""));
    let proof = pasejo_protocol::request::prove(
        "GET",
        &path(org, store, "index"),
        alice.client.clock.now(),
        b"",
        &alice.client.token,
        &alice.client.key,
    );
    let cached = alice
        .client
        .send_with(
            Method::GET,
            &path(org, store, "index"),
            vec![],
            &[
                ("authorization", proof.authorization),
                (pasejo_protocol::request::TIMESTAMP_HEADER, proof.timestamp),
                (pasejo_protocol::request::NONCE_HEADER, proof.nonce),
                (pasejo_protocol::request::PROOF_HEADER, proof.proof),
                ("if-none-match", "\"1\"".into()),
            ],
        )
        .await;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn commits_are_checked_against_the_store_and_the_log() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let id = Id::random();
    let first = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id,
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    let first_commit = first.sign(&alice);
    commit(&alice, org, store, &first_commit)
        .await
        .ok::<Committed>();
    let head = Some(first_commit.index.hash());
    let next = |version, prev, writes: &'static [(&'static str, u64)]| {
        let alice = &alice;
        let keys = &keys;
        let base = first.plain.clone();
        async move {
            let writes: Vec<Write> = writes
                .iter()
                .map(|(name, v)| Write {
                    name,
                    id,
                    version: *v,
                    value: "x",
                })
                .collect();
            draft(
                alice,
                org,
                store,
                0,
                keys,
                version,
                prev,
                &base,
                &writes,
                &[],
            )
            .await
        }
    };

    // A stale version and a wrong predecessor conflict.
    let stale = commit(
        &alice,
        org,
        store,
        &next(1, None, &[("a", 2)]).await.sign(&alice),
    )
    .await;
    stale.expect(StatusCode::CONFLICT, "conflict");
    assert_eq!(stale.json::<Value>()["current_version"], 1);
    commit(
        &alice,
        org,
        store,
        &next(2, Some(Hash([0; 32])), &[("a", 2)]).await.sign(&alice),
    )
    .await
    .expect(StatusCode::CONFLICT, "conflict");
    commit(
        &alice,
        org,
        store,
        &next(3, head, &[("a", 2)]).await.sign(&alice),
    )
    .await
    .expect(StatusCode::CONFLICT, "conflict");

    // Object versions must grow.
    commit(
        &alice,
        org,
        store,
        &next(2, head, &[("a", 1)]).await.sign(&alice),
    )
    .await
    .expect(StatusCode::BAD_REQUEST, "bad_request");

    // Changed identifiers must match the uploaded objects.
    let mut d = next(2, head, &[("a", 2)]).await;
    d.objects.clear();
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.changed.push(Id::random());
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.changed.push(id);
    d.objects.push(d.objects[0].clone());
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.deleted.push(id);
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // Objects carry the index's epoch and a sane size.
    let mut d = next(2, head, &[("a", 2)]).await;
    d.objects[0].epoch = 1;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.objects[0].ciphertext = Bytes(Vec::new());
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // The lookup object must match its hash, and be present when named.
    let mut d = next(2, head, &[("a", 2)]).await;
    d.lookup = Some(b"other".to_vec());
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.lookup = None;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // The index must use the current epoch and name this store.
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.epoch = 1;
    for object in &mut d.objects {
        object.epoch = 1;
    }
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.store = Id::random();
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // A backdated index is refused.
    let mut d = next(2, head, &[("a", 2)]).await;
    d.payload.time -= 301_000;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // Nothing of the refused commits was stored.
    let lookup: LookupResponse = alice.client.get(&path(org, store, "lookup")).await.ok();
    assert_eq!(lookup.version, 1);
    commit(
        &alice,
        org,
        store,
        &next(2, head, &[("a", 2)]).await.sign(&alice),
    )
    .await
    .ok::<Committed>();

    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .filter(|event| event.action == "commit" && event.outcome == "denied")
            .count()
            >= 10
    );
}

#[tokio::test]
async fn only_writers_commit_and_only_as_their_own_device() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    grant(&alice, org, store, bob.user(), StoreRoleChange::Reader).await;
    let d = draft(
        &bob,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    commit(&bob, org, store, &d.sign(&bob))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    // Alice's signature uploaded by bob, and bob's signature uploaded by alice.
    commit(&bob, org, store, &d.sign(&alice))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    grant(&alice, org, store, bob.user(), StoreRoleChange::Writer).await;
    commit(&alice, org, store, &d.sign(&bob))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let d = draft(
        &bob,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    commit(&bob, org, store, &d.sign(&bob))
        .await
        .ok::<Committed>();
}

#[tokio::test]
async fn concurrent_commits_to_one_version_conflict() {
    let Setup {
        alice,
        org,
        store,
        keys,
        harness: _harness,
        ..
    } = setup().await;
    let a = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    let b = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "b",
            id: Id::random(),
            version: 1,
            value: "2",
        }],
        &[],
    )
    .await;
    let (a, b) = (a.sign(&alice), b.sign(&alice));
    let (first, second) = tokio::join!(
        commit(&alice, org, store, &a),
        commit(&alice, org, store, &b)
    );
    let mut statuses = [first.status, second.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::CONFLICT],
        "{} / {}",
        first.text(),
        second.text()
    );
}

#[tokio::test]
async fn ten_versions_of_each_object_are_kept() {
    let Setup {
        alice,
        org,
        store,
        keys,
        harness: _harness,
        ..
    } = setup().await;
    let id = Id::random();
    let mut base = IndexPlain::default();
    let mut prev = None;
    for version in 1..=12 {
        let d = draft(
            &alice,
            org,
            store,
            0,
            &keys,
            version,
            prev,
            &base,
            &[Write {
                name: "a",
                id,
                version,
                value: "v",
            }],
            &[],
        )
        .await;
        let signed = d.sign(&alice);
        commit(&alice, org, store, &signed).await.ok::<Committed>();
        prev = Some(signed.index.hash());
        base = d.plain;
    }
    let fetch = |version| {
        let alice = &alice;
        async move {
            alice
                .client
                .post(
                    &path(org, store, "objects"),
                    &ObjectsRequest {
                        objects: vec![ObjectRef { id, version }],
                    },
                )
                .await
        }
    };
    fetch(2).await.expect(StatusCode::NOT_FOUND, "not_found");
    fetch(3).await.ok::<ObjectsResponse>();
    fetch(12).await.ok::<ObjectsResponse>();
    fetch(13).await.expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn deletions_are_committed_and_audited() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let id = Id::random();
    let first = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id,
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    let signed = first.sign(&alice);
    commit(&alice, org, store, &signed).await.ok::<Committed>();
    let second = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        2,
        Some(signed.index.hash()),
        &first.plain,
        &[],
        &["a"],
    )
    .await;
    assert_eq!(second.payload.deleted, vec![id]);
    commit(&alice, org, store, &second.sign(&alice))
        .await
        .ok::<Committed>();
    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.action == "object.delete" && event.object == Some(id))
    );
}

#[tokio::test]
async fn names_only_devices_read_the_index_but_never_secrets() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let d = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .ok::<Committed>();
    // Alice's browser: same user, an HPKE device.
    let browser_client = harness.sign_in("alice").await;
    let (private, public) = hpke_keypair();
    let browser = Member {
        record: DeviceRecord {
            id: Id::random(),
            user: alice.user(),
            kind: DeviceKind::Hpke,
            recipient: b64(&public),
            signing_key: Fixed(browser_client.key.verifying_key().to_bytes()),
            label: "browser".into(),
        },
        client: browser_client,
        identity: age::x25519::Identity::generate(),
    };
    alice
        .append(
            org,
            Body::DeviceAdd {
                device: browser.record.clone(),
            },
        )
        .await
        .ok::<Value>();
    // A full wrapping to a browser is refused; a names-only one is accepted.
    let mut full = wrappings(store, 0, &keys, &[Target::Hpke(browser.record.id, public)]);
    full[0].full = true;
    alice
        .client
        .post(
            &path(org, store, "wrappings"),
            &WrappingsUpload {
                epoch: 0,
                wrappings: full,
            },
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    wrap_for(
        &alice,
        org,
        store,
        0,
        &keys,
        &[Target::Hpke(browser.record.id, public)],
    )
    .await;

    let fetched: Keys = browser.client.get(&path(org, store, "keys")).await.ok();
    assert!(!fetched.wrappings[0].full);
    let names = unwrap_hpke(
        store,
        0,
        browser.record.id,
        &private,
        &fetched.wrappings[0].ciphertext.0,
    )
    .unwrap();
    assert!(names.data.is_none());
    let index: IndexResponse = browser.client.get(&path(org, store, "index")).await.ok();
    let payload: pasejo_protocol::store::IndexPayload = index.index.unwrap().open().unwrap();
    let plain = open_index(store, 0, 1, &names.names, &payload.index.0).unwrap();
    assert!(plain.secrets.contains_key("a"));
    browser
        .client
        .get(&path(org, store, "lookup"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    browser
        .client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest {
                objects: vec![ObjectRef {
                    id: plain.secrets["a"].id,
                    version: 1,
                }],
            },
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn devices_without_a_grant_read_nothing() {
    let Setup {
        harness,
        alice,
        org,
        store,
        ..
    } = setup().await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    for rest in ["keys", "index", "lookup"] {
        bob.client
            .get(&path(org, store, rest))
            .await
            .expect(StatusCode::FORBIDDEN, "forbidden");
    }
    bob.client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest { objects: vec![] },
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let listed: Stores = bob.client.get(&format!("/v1/orgs/{org}/stores")).await.ok();
    assert!(listed.stores.is_empty());
    let listed: Stores = alice
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok();
    assert_eq!(listed.stores.len(), 1);
    assert_eq!(listed.stores[0].role, Some(StoreRole::Admin));
    assert_eq!(listed.stores[0].epoch, Some(0));
    // Unknown stores.
    alice
        .client
        .get(&path(org, Id::random(), "keys"))
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_store");
}

#[tokio::test]
async fn auditors_see_every_store_without_its_keys() {
    let Setup {
        harness,
        alice,
        org,
        store,
        ..
    } = setup().await;
    let carol = harness.member("carol").await;
    harness.join(&alice, &carol, org, RoleChange::Auditor).await;
    let listed: Stores = carol
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok();
    assert_eq!(listed.stores.len(), 1);
    assert_eq!(listed.stores[0].role, None);
    // Auditors hold the names key, to resolve names in the audit log, and
    // nothing more.
    let log = alice.log(org).await;
    assert!(
        log.state()
            .entitled(store, harness.now())
            .names_only
            .contains(&carol.record.id)
    );
    carol
        .client
        .get(&path(org, store, "keys"))
        .await
        .ok::<Keys>();
    carol
        .client
        .get(&path(org, store, "index"))
        .await
        .ok::<IndexResponse>();
    carol
        .client
        .get(&path(org, store, "lookup"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn deleted_stores_are_gone() {
    let Setup {
        alice,
        org,
        store,
        harness: _harness,
        ..
    } = setup().await;
    alice
        .append(org, Body::StoreDelete { store })
        .await
        .ok::<Value>();
    alice
        .client
        .get(&path(org, store, "keys"))
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_store");
    let listed: Stores = alice
        .client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok();
    assert!(listed.stores.is_empty());
}

#[tokio::test]
async fn epoch_uploads_are_checked() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    harness.join(&alice, &carol, org, RoleChange::Member).await;
    grant(&alice, org, store, bob.user(), StoreRoleChange::Writer).await;
    // Only store admins upload epochs.
    bob.upload_epoch(org, store, 0, &keys, None, &[Target::Age(&bob)])
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    // The epoch must be in the log.
    alice
        .upload_epoch(org, store, 1, &keys, Some(&keys), &[Target::Age(&alice)])
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
    // Wrappings go only to entitled devices, once each.
    alice
        .upload_epoch(org, store, 0, &keys, None, &[Target::Age(&carol)])
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    alice
        .upload_epoch(
            org,
            store,
            0,
            &keys,
            None,
            &[Target::Age(&alice), Target::Age(&alice)],
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // Epoch 1 needs its chain link, and the link is stored once.
    let next = EpochKeys::generate();
    alice
        .append(
            org,
            Body::StoreGrant {
                store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::None,
                expires: None,
            },
        )
        .await
        .ok::<Value>();
    let reply = alice
        .start_epoch(org, store, 1, &next, None, &[Target::Age(&alice)])
        .await;
    reply.expect(StatusCode::BAD_REQUEST, "bad_request");
    alice
        .upload_epoch(org, store, 1, &next, Some(&keys), &[Target::Age(&alice)])
        .await;
    let other = EpochKeys::generate();
    alice
        .upload_epoch(org, store, 1, &next, Some(&other), &[Target::Age(&alice)])
        .await
        .expect(StatusCode::CONFLICT, "conflict");
    // A chain on epoch 0 is refused.
    alice
        .upload_epoch(org, store, 0, &keys, Some(&keys), &[Target::Age(&alice)])
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");

    // Alice reaches epoch 0 through the chain.
    let fetched: Keys = alice.client.get(&path(org, store, "keys")).await.ok();
    assert_eq!(fetched.chain.len(), 1);
    let current = unwrap_age(
        store,
        1,
        &fetched
            .wrappings
            .iter()
            .find(|w| w.epoch == 1)
            .unwrap()
            .ciphertext
            .0,
        &alice.identities(),
    )
    .unwrap();
    let chain = pasejo_protocol::crypto::Chain {
        prev_names: fetched.chain[0].prev_names.0.clone(),
        prev_data: fetched.chain[0].prev_data.0.clone(),
    };
    assert_eq!(open_chain(store, 1, &current, &chain).unwrap(), keys);
}

#[tokio::test]
async fn wrapping_uploads_are_checked() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    harness.join(&alice, &carol, org, RoleChange::Member).await;
    grant(&alice, org, store, bob.user(), StoreRoleChange::Reader).await;
    grant(&alice, org, store, carol.user(), StoreRoleChange::Reader).await;
    // Only the current epoch.
    alice
        .client
        .post(
            &path(org, store, "wrappings"),
            &WrappingsUpload {
                epoch: 1,
                wrappings: wrappings(store, 1, &keys, &[Target::Age(&bob)]),
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    // Bob holds no keys yet, so he cannot wrap them for carol.
    bob.client
        .post(
            &path(org, store, "wrappings"),
            &WrappingsUpload {
                epoch: 0,
                wrappings: wrappings(store, 0, &keys, &[Target::Age(&carol)]),
            },
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    wrap_for(&alice, org, store, 0, &keys, &[Target::Age(&bob)]).await;
    // A reader may add a missing wrapping but cannot replace one.
    wrap_for(&bob, org, store, 0, &keys, &[Target::Age(&carol)]).await;
    let before: Keys = carol.client.get(&path(org, store, "keys")).await.ok();
    wrap_for(&bob, org, store, 0, &keys, &[Target::Age(&carol)]).await;
    let after: Keys = carol.client.get(&path(org, store, "keys")).await.ok();
    assert_eq!(before, after);
    // A store admin replaces it.
    wrap_for(&alice, org, store, 0, &keys, &[Target::Age(&carol)]).await;
    let replaced: Keys = carol.client.get(&path(org, store, "keys")).await.ok();
    assert_ne!(before, replaced);
}

#[tokio::test]
async fn object_requests_are_bounded_and_audited_when_missing() {
    let Setup {
        harness,
        alice,
        org,
        store,
        ..
    } = setup().await;
    let refs: Vec<ObjectRef> = (0..10_001)
        .map(|_| ObjectRef {
            id: Id([1; 16]),
            version: 1,
        })
        .collect();
    let reply = alice
        .client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest { objects: refs },
        )
        .await;
    assert!(
        reply.status == StatusCode::BAD_REQUEST || reply.status == StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        reply.status
    );
    let id = Id::random();
    alice
        .client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest {
                objects: vec![ObjectRef { id, version: 1 }],
            },
        )
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(events.iter().any(|event| event.action == "object.read"
        && event.outcome == "denied"
        && event.object == Some(id)));
}

#[tokio::test]
async fn commits_may_exceed_the_general_body_limit() {
    let Setup {
        alice,
        org,
        store,
        keys,
        harness: _harness,
        ..
    } = setup().await;
    let big = "x".repeat(700_000);
    let writes: Vec<Write> = ["a", "b"]
        .iter()
        .map(|name| Write {
            name,
            id: Id::random(),
            version: 1,
            value: &big,
        })
        .collect();
    let d = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &writes,
        &[],
    )
    .await;
    commit(&alice, org, store, &d.sign(&alice))
        .await
        .ok::<Committed>();
    // Other endpoints keep the 1 MiB cap.
    let padding = vec![
        ObjectRef {
            id: Id([2; 16]),
            version: 1
        };
        40_000
    ];
    alice
        .client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest { objects: padding },
        )
        .await
        .expect(StatusCode::PAYLOAD_TOO_LARGE, "too_large");
}

#[tokio::test]
async fn rotation_lists_name_what_a_former_reader_could_read() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let (read_id, unread_id) = (Id::random(), Id::random());
    let first = draft(
        &alice,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[
            Write {
                name: "read",
                id: read_id,
                version: 1,
                value: "1",
            },
            Write {
                name: "unread",
                id: unread_id,
                version: 1,
                value: "2",
            },
        ],
        &[],
    )
    .await;
    let signed = first.sign(&alice);
    commit(&alice, org, store, &signed).await.ok::<Committed>();
    grant(&alice, org, store, bob.user(), StoreRoleChange::Reader).await;
    wrap_for(&alice, org, store, 0, &keys, &[Target::Age(&bob)]).await;
    bob.client
        .post(
            &path(org, store, "objects"),
            &ObjectsRequest {
                objects: vec![ObjectRef {
                    id: read_id,
                    version: 1,
                }],
            },
        )
        .await
        .ok::<ObjectsResponse>();

    let ongoing: Rotation = alice
        .client
        .get(&path(org, store, &format!("rotation?user={}", bob.user())))
        .await
        .ok();
    assert_eq!(ongoing.ended, None);
    assert!(ongoing.items.is_empty());
    // Bob cannot see rotation lists.
    bob.client
        .get(&path(org, store, &format!("rotation?user={}", bob.user())))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");

    harness.advance(1_000);
    grant(&alice, org, store, bob.user(), StoreRoleChange::None).await;
    let ended: Rotation = alice
        .client
        .get(&path(org, store, &format!("rotation?user={}", bob.user())))
        .await
        .ok();
    assert!(ended.ended.is_some());
    let mut items = ended.items.clone();
    items.sort_by_key(|item| item.id);
    let read = items.iter().find(|item| item.id == read_id).unwrap();
    assert!(read.fetched && !read.rewritten);
    let unread = items.iter().find(|item| item.id == unread_id).unwrap();
    assert!(!unread.fetched && !unread.rewritten);

    // Rotating the read secret marks it rewritten.
    harness.advance(1_000);
    let next = EpochKeys::generate();
    alice
        .start_epoch(org, store, 1, &next, Some(&keys), &[Target::Age(&alice)])
        .await;
    let second = draft(
        &alice,
        org,
        store,
        1,
        &next,
        2,
        Some(signed.index.hash()),
        &first.plain,
        &[Write {
            name: "read",
            id: read_id,
            version: 2,
            value: "new",
        }],
        &[],
    )
    .await;
    commit(&alice, org, store, &second.sign(&alice))
        .await
        .ok::<Committed>();
    let after: Rotation = alice
        .client
        .get(&path(org, store, &format!("rotation?user={}", bob.user())))
        .await
        .ok();
    assert!(
        after
            .items
            .iter()
            .find(|item| item.id == read_id)
            .unwrap()
            .rewritten
    );
    assert!(
        !after
            .items
            .iter()
            .find(|item| item.id == unread_id)
            .unwrap()
            .rewritten
    );

    // A user who never had access has no list.
    let never: Rotation = alice
        .client
        .get(&path(
            org,
            store,
            &format!("rotation?user={}", Id::random()),
        ))
        .await
        .ok();
    assert_eq!(never.ended, None);
    assert!(never.items.is_empty());
}

#[tokio::test]
async fn expired_grants_end_access_on_their_own() {
    let Setup {
        harness,
        alice,
        org,
        store,
        keys,
    } = setup().await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let expires = harness.now() + 60_000;
    alice
        .append(
            org,
            Body::StoreGrant {
                store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::Writer,
                expires: Some(expires),
            },
        )
        .await
        .ok::<Value>();
    wrap_for(&alice, org, store, 0, &keys, &[Target::Age(&bob)]).await;
    bob.client.get(&path(org, store, "keys")).await.ok::<Keys>();
    harness.advance(61_000);
    bob.client
        .get(&path(org, store, "keys"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let d = draft(
        &bob,
        org,
        store,
        0,
        &keys,
        1,
        None,
        &IndexPlain::default(),
        &[Write {
            name: "a",
            id: Id::random(),
            version: 1,
            value: "1",
        }],
        &[],
    )
    .await;
    commit(&bob, org, store, &d.sign(&bob))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let rotation: Rotation = alice
        .client
        .get(&path(org, store, &format!("rotation?user={}", bob.user())))
        .await
        .ok();
    assert_eq!(rotation.ended, Some(expires));
}
