// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The blocking client against a real server: sign-in, organizations,
//! stores, commits, rotation, pending actions, reviews and audit
//! verification. Client calls run in `block_in_place` so the server keeps
//! serving on the other runtime workers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{Harness, Options, between};
use ed25519_dalek::SigningKey;
use pasejo_client::actions::Outcome;
use pasejo_client::session::Poll;
use pasejo_client::store::{Changes, Kind};
use pasejo_client::{Client, Clock, DeviceIdentity};
use pasejo_protocol::Id;
use pasejo_protocol::api::{Action, Decision, ErrorCode};
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::log::{
    Approval, Body, OrgLog, Principal, RoleChange, Settings, StoreRoleChange,
};
use pasejo_protocol::store::{ObjectPlain, OtpParams};
use rand_core::OsRng;
use reqwest::StatusCode;

fn sync<T>(work: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(work)
}

struct Device {
    client: Client,
    identity: age::x25519::Identity,
}

impl Device {
    fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }

    fn user(&self) -> Id {
        self.client.user().unwrap()
    }
}

fn clock(harness: &Harness) -> Clock {
    let clock = Arc::clone(&harness.state.clock);
    Arc::new(move || clock.now())
}

fn unsigned(harness: &Harness, label: &str) -> Device {
    let identity = age::x25519::Identity::generate();
    let device = DeviceIdentity {
        id: Id::random(),
        signing_key: SigningKey::generate(&mut OsRng),
        kind: DeviceKind::Age,
        recipient: identity.to_public().to_string(),
        label: label.into(),
    };
    Device {
        client: Client::new(
            &harness.url,
            device,
            clock(harness),
            Duration::from_secs(20),
        ),
        identity,
    }
}

async fn approve_in_browser(harness: &Harness, verification: &str, subject: &str) {
    harness.idp.set_user(
        subject,
        &format!("{subject}@example.com"),
        &common::capitalize(subject),
    );
    let page = harness.follow_to_callback(verification).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.text());
    let approval = between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
    assert_eq!(
        harness.approve(&approval, "approve").await.status,
        StatusCode::OK
    );
}

async fn signed_in(harness: &Harness, subject: &str) -> Device {
    let mut device = unsigned(harness, "laptop");
    let authorization = sync(|| device.client.start_login()).unwrap();
    approve_in_browser(harness, &authorization.verification_uri_complete, subject).await;
    match sync(|| device.client.poll_login(&authorization)).unwrap() {
        Poll::Done(session) => assert!(session.user.is_some()),
        other => panic!("{other:?}"),
    }
    device
}

/// Enrolls `member` and has `admin` add them with `role`.
fn join(admin: &Device, log: &mut OrgLog, member: &Device, role: RoleChange) {
    let org = log.state().org;
    member.client.enroll(org).unwrap();
    let enrollment = admin
        .client
        .enrollments(org)
        .unwrap()
        .into_iter()
        .find(|enrollment| enrollment.device.id == member.client.device.id)
        .unwrap();
    assert_eq!(enrollment.fingerprint, member.client.device.fingerprint());
    admin
        .client
        .append_fresh(
            log,
            Body::MemberRole {
                user: member.user(),
                role,
            },
        )
        .unwrap();
    admin
        .client
        .append_fresh(
            log,
            Body::DeviceAdd {
                device: enrollment.device,
            },
        )
        .unwrap();
}

fn secret(value: &str) -> ObjectPlain {
    ObjectPlain::Secret {
        secret: value.into(),
    }
}

fn totp() -> OtpParams {
    OtpParams {
        secret: "JBSWY3DPEHPK3PXP".into(),
        otp_type: "totp".into(),
        algorithm: "sha1".into(),
        digits: 6,
        period: 30,
        counter: 0,
        skew: 1,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn signing_in_waits_for_approval_and_ends_on_logout() {
    let harness = Harness::start().await;
    let mut device = unsigned(&harness, "laptop");
    let info = sync(|| device.client.server_info()).unwrap();
    assert!(info.oidc && info.break_glass);
    assert_eq!(
        info.checkpoint_key.0,
        harness.state.keys.checkpoint.verifying_key().to_bytes()
    );
    let authorization = sync(|| device.client.start_login()).unwrap();
    assert!(matches!(
        sync(|| device.client.poll_login(&authorization)).unwrap(),
        Poll::Pending
    ));
    assert!(matches!(
        sync(|| device.client.poll_login(&authorization)).unwrap(),
        Poll::SlowDown
    ));
    approve_in_browser(&harness, &authorization.verification_uri_complete, "alice").await;
    harness.advance(6_000);
    assert!(matches!(
        sync(|| device.client.poll_login(&authorization)).unwrap(),
        Poll::Done(_)
    ));
    let me = sync(|| device.client.me()).unwrap();
    assert_eq!(me.email, "alice@example.com");
    sync(|| device.client.logout()).unwrap();
    assert!(matches!(
        sync(|| device.client.me()),
        Err(pasejo_client::Error::NoSession)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn apps_sign_in_through_the_system_browser() {
    let harness = Harness::start().await;
    let mut device = unsigned(&harness, "phone");
    let login = device.client.start_app_login("wtf.metio.pasejo:/auth");
    assert!(
        login
            .url
            .starts_with(&format!("{}/auth/app/start?challenge=", harness.url)),
        "{}",
        login.url
    );
    assert!(
        login
            .url
            .contains("redirect_uri=wtf.metio.pasejo%3A%2Fauth")
    );
    assert!(login.url.ends_with(&format!("&state={}", login.state)));
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let redirect = harness
        .follow_to_callback(&login.url)
        .await
        .location()
        .unwrap();
    // A redirect of another sign-in is refused before reaching the server.
    let other = device.client.start_app_login("wtf.metio.pasejo:/auth");
    let error = sync(|| device.client.finish_app_login(&other, &redirect)).unwrap_err();
    assert!(
        matches!(error, pasejo_client::Error::Verification(_)),
        "{error}"
    );
    let session = sync(|| device.client.finish_app_login(&login, &redirect)).unwrap();
    assert!(session.user.is_some());
    assert_eq!(
        sync(|| device.client.me()).unwrap().email,
        "alice@example.com"
    );
    // The code is spent.
    let mut again = unsigned(&harness, "tablet");
    let error = sync(|| again.client.finish_app_login(&login, &redirect)).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Unauthorized), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn machines_sign_in_with_workload_tokens() {
    let harness = Harness::start().await;
    let mut device = unsigned(&harness, "ci");
    let jwt = harness.workload_jwt("system:serviceaccount:ci:deployer", "pasejo-server", 300);
    let session = sync(|| device.client.workload_login(&jwt)).unwrap();
    assert!(session.user.is_some());
    assert!(sync(|| device.client.me()).unwrap().machine);
    let foreign = harness.workload_jwt("system:serviceaccount:other:x", "pasejo-server", 300);
    let error = sync(|| unsigned(&harness, "x").client.workload_login(&foreign)).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Forbidden), "{error}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_store_round_trips_between_two_members() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let genesis = log.genesis_hash();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "production")).unwrap();
    assert_eq!(
        (snapshot.version, snapshot.plain.label.as_str()),
        (1, "production")
    );

    let mut changes = Changes::default();
    changes.secrets.insert(
        "db/prod".into(),
        Some("hunter2\nurl: https://db.example.com\nuser: admin".into()),
    );
    changes.secrets.insert("api".into(), Some("token".into()));
    changes.otp.insert("github.com".into(), Some(totp()));
    let snapshot = sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();
    assert_eq!(snapshot.version, 2);

    // Bob joins, gets read access, and the pending action wraps his keys.
    sync(|| join(&alice, &mut log, &bob, RoleChange::Member));
    let store = ring.store;
    sync(|| {
        alice.client.grant(
            &mut log,
            store,
            Principal::User(bob.user()),
            StoreRoleChange::Reader,
            None,
        )
    })
    .unwrap();
    let pending = sync(|| alice.client.actions(org)).unwrap();
    assert!(pending.iter().any(|action| matches!(action, Action::WrapKeys { devices, .. } if devices.contains(&bob.client.device.id))));
    let outcomes = sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    assert!(
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Done(Action::WrapKeys { .. }))),
        "{outcomes:?}"
    );

    let bob_log = sync(|| bob.client.fetch_log(org, None, Some(genesis))).unwrap();
    assert_eq!(bob_log.head(), log.head());
    let bob_ring = sync(|| bob.client.keys(&bob_log, store, &bob.identities())).unwrap();
    assert!(bob_ring.full());
    let bob_view = sync(|| bob.client.index(&bob_log, &bob_ring, None)).unwrap();
    assert_eq!(bob_view.plain, snapshot.plain);
    let lookup = sync(|| bob.client.lookup(&bob_log, &bob_ring, &bob_view)).unwrap();
    assert_eq!(
        lookup.secrets["db/prod"],
        "url: https://db.example.com\nuser: admin"
    );
    assert_eq!(lookup.secrets["api"], "");
    let values = sync(|| {
        bob.client.objects(
            &bob_log,
            &bob_ring,
            &bob_view,
            &[
                (Kind::Secret, "db/prod".into()),
                (Kind::Otp, "github.com".into()),
            ],
        )
    })
    .unwrap();
    assert_eq!(
        values[&(Kind::Secret, "db/prod".into())],
        secret("hunter2\nurl: https://db.example.com\nuser: admin")
    );
    assert_eq!(
        values[&(Kind::Otp, "github.com".into())],
        ObjectPlain::Otp { otp: totp() }
    );

    // A reader cannot commit.
    let mut write = Changes::default();
    write.secrets.insert("x".into(), Some("y".into()));
    let error = sync(|| bob.client.commit(&bob_log, &bob_ring, &bob_view, &write)).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Forbidden), "{error}");
    // Unknown names and a pinned future version are refused locally.
    assert!(
        sync(|| bob.client.objects(
            &bob_log,
            &bob_ring,
            &bob_view,
            &[(Kind::Secret, "missing".into())]
        ))
        .is_err()
    );
    let mut future = bob_view.pin().unwrap();
    future.version += 1;
    assert!(sync(|| bob.client.index(&bob_log, &bob_ring, Some(future))).is_err());
    // A log pinned to another genesis is refused.
    assert!(
        sync(|| bob
            .client
            .fetch_log(org, None, Some(pasejo_protocol::Hash([7; 32]))))
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_writers_conflict_and_retry_and_renames_keep_identifiers() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let (ring, base) = sync(|| alice.client.create_store(&mut log, "shared")).unwrap();
    let mut first = Changes::default();
    first.secrets.insert("a".into(), Some("1".into()));
    let mut second = Changes::default();
    second.secrets.insert("b".into(), Some("2".into()));
    let after_first = sync(|| alice.client.commit(&log, &ring, &base, &first)).unwrap();
    let error = sync(|| alice.client.commit(&log, &ring, &base, &second)).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Conflict), "{error}");
    let fresh = sync(|| alice.client.index(&log, &ring, base.pin())).unwrap();
    assert_eq!(fresh, after_first);
    let both = sync(|| alice.client.commit(&log, &ring, &fresh, &second)).unwrap();
    assert_eq!(both.plain.secrets.keys().collect::<Vec<_>>(), ["a", "b"]);

    let id = both.plain.secrets["a"].id;
    let renamed = Changes {
        renames: vec![(Kind::Secret, "a".into(), "c".into())],
        ..Changes::default()
    };
    let after = sync(|| alice.client.commit(&log, &ring, &both, &renamed)).unwrap();
    assert_eq!(after.plain.secrets["c"].id, id);
    assert_eq!(after.plain.secrets["c"].version, 1);
    let onto_taken = Changes {
        renames: vec![(Kind::Secret, "b".into(), "c".into())],
        ..Changes::default()
    };
    assert!(sync(|| alice.client.commit(&log, &ring, &after, &onto_taken)).is_err());
    let mut update = Changes::default();
    update.secrets.insert("c".into(), Some("3".into()));
    update.secrets.insert("b".into(), None);
    let updated = sync(|| alice.client.commit(&log, &ring, &after, &update)).unwrap();
    assert_eq!(updated.plain.secrets["c"].version, 2);
    assert!(!updated.plain.secrets.contains_key("b"));
    let values = sync(|| {
        alice
            .client
            .objects(&log, &ring, &updated, &[(Kind::Secret, "c".into())])
    })
    .unwrap();
    assert_eq!(values[&(Kind::Secret, "c".into())], secret("3"));
    let lookup = sync(|| alice.client.lookup(&log, &ring, &updated)).unwrap();
    assert_eq!(lookup.secrets.keys().collect::<Vec<_>>(), ["c"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn removing_a_reader_rotates_the_epoch_and_lists_what_to_rotate() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "shared")).unwrap();
    let store = ring.store;
    let mut changes = Changes::default();
    changes.secrets.insert("read".into(), Some("1".into()));
    changes.secrets.insert("unread".into(), Some("2".into()));
    let snapshot = sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();
    sync(|| join(&alice, &mut log, &bob, RoleChange::Member));
    sync(|| {
        alice.client.grant(
            &mut log,
            store,
            Principal::User(bob.user()),
            StoreRoleChange::Reader,
            None,
        )
    })
    .unwrap();
    sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    let bob_log = sync(|| bob.client.fetch_log(org, None, None)).unwrap();
    let bob_ring = sync(|| bob.client.keys(&bob_log, store, &bob.identities())).unwrap();
    let bob_view = sync(|| bob.client.index(&bob_log, &bob_ring, None)).unwrap();
    sync(|| {
        bob.client.objects(
            &bob_log,
            &bob_ring,
            &bob_view,
            &[(Kind::Secret, "read".into())],
        )
    })
    .unwrap();

    harness.advance(1_000);
    sync(|| {
        alice.client.grant(
            &mut log,
            store,
            Principal::User(bob.user()),
            StoreRoleChange::None,
            None,
        )
    })
    .unwrap();
    let outcomes = sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    assert!(
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Done(Action::RotateEpoch { epoch: 1, .. }))),
        "{outcomes:?}"
    );
    assert_eq!(log.state().stores[&store].current_epoch(), Some(1));
    // Bob's access ended at once.
    let bob_log = sync(|| bob.client.fetch_log(org, Some(bob_log), None)).unwrap();
    assert!(sync(|| bob.client.keys(&bob_log, store, &bob.identities())).is_err());

    // Alice reads old objects through the chain; the index moved to epoch 1.
    let ring = sync(|| alice.client.keys(&log, store, &alice.identities())).unwrap();
    assert_eq!(ring.current, 1);
    let current = sync(|| alice.client.index(&log, &ring, snapshot.pin())).unwrap();
    assert_eq!(current.epoch, 1);
    let values = sync(|| {
        alice
            .client
            .objects(&log, &ring, &current, &[(Kind::Secret, "read".into())])
    })
    .unwrap();
    assert_eq!(values[&(Kind::Secret, "read".into())], secret("1"));

    let rotation = sync(|| alice.client.rotation(&log, store, bob.user())).unwrap();
    assert!(rotation.ended.is_some());
    let read_id = current.plain.secrets["read"].id;
    let item = rotation
        .items
        .iter()
        .find(|item| item.id == read_id)
        .unwrap();
    assert!(item.fetched && !item.rewritten);
    // Rekeying rewrites every object under the new epoch.
    let rekeyed = sync(|| alice.client.rekey(&log, &ring, &current)).unwrap();
    assert_eq!(rekeyed.plain.secrets["read"].version, 2);
    let rotation = sync(|| alice.client.rotation(&log, store, bob.user())).unwrap();
    assert!(rotation.items.iter().all(|item| item.rewritten));
}

#[tokio::test(flavor = "multi_thread")]
async fn automatic_approval_admits_members_devices_only() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let carol = signed_in(&harness, "carol").await;
    let org = Id::random();
    let settings = Settings {
        device_approval: Approval::Automatic,
        ..Settings::default()
    };
    let mut log = sync(|| alice.client.create_org(org, "Acme", settings)).unwrap();
    sync(|| {
        alice.client.append(
            &mut log,
            Body::MemberRole {
                user: bob.user(),
                role: RoleChange::Member,
            },
        )
    })
    .unwrap();
    sync(|| bob.client.enroll(org)).unwrap();
    sync(|| carol.client.enroll(org)).unwrap();
    let outcomes = sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    assert!(log.state().active_device(bob.client.device.id).is_some());
    assert!(log.state().active_device(carol.client.device.id).is_none());
    assert!(
        outcomes
            .iter()
            .any(|outcome| matches!(outcome, Outcome::Skipped(Action::ApproveDevice { .. }, _)))
    );
    assert!(sync(|| bob.client.enrollment_status(org)).unwrap().active);

    // In manual mode the action waits for a fingerprint comparison.
    let dave = signed_in(&harness, "dave").await;
    let manual = Id::random();
    let mut manual_log = sync(|| {
        alice
            .client
            .create_org(manual, "Manual", Settings::default())
    })
    .unwrap();
    sync(|| {
        alice.client.append(
            &mut manual_log,
            Body::MemberRole {
                user: dave.user(),
                role: RoleChange::Member,
            },
        )
    })
    .unwrap();
    sync(|| dave.client.enroll(manual)).unwrap();
    let outcomes = sync(|| {
        alice
            .client
            .run_actions(&mut manual_log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    let reason = outcomes
        .iter()
        .find_map(|outcome| match outcome {
            Outcome::Skipped(Action::ApproveDevice { .. }, reason) => Some(reason.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        reason.contains(&dave.client.device.fingerprint()),
        "{reason}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn two_admin_changes_go_through_proposals() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let carol = signed_in(&harness, "carol").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    sync(|| join(&alice, &mut log, &bob, RoleChange::Admin));
    sync(|| join(&alice, &mut log, &carol, RoleChange::Member));
    let two_admin = Settings {
        two_admin: true,
        ..Settings::default()
    };
    // Switching two-admin mode on or off already takes two admins.
    let enable = Body::OrgSettings {
        settings: two_admin.clone(),
    };
    assert!(sync(|| alice.client.append(&mut log, enable.clone())).is_err());
    let proposal = sync(|| alice.client.propose(&log, enable)).unwrap();
    let mut bob_log = sync(|| bob.client.fetch_log(org, None, None)).unwrap();
    sync(|| bob.client.cosign_proposal(&mut bob_log, &proposal)).unwrap();
    assert!(bob_log.state().settings.two_admin);
    log = sync(|| alice.client.fetch_log(org, Some(log), None)).unwrap();
    // A proposal that needs no second admin is refused.
    let plain = Body::TeamCreate {
        team: Id::random(),
        name: "ops".into(),
    };
    let error = sync(|| alice.client.propose(&log, plain)).unwrap_err();
    assert!(error.code().is_some(), "{error}");

    let promote = Body::MemberRole {
        user: carol.user(),
        role: RoleChange::Admin,
    };
    let error = sync(|| alice.client.append(&mut log, promote.clone())).unwrap_err();
    assert!(
        matches!(error, pasejo_client::Error::NeedsSecondAdmin),
        "{error}"
    );
    let proposal = sync(|| alice.client.propose(&log, promote.clone())).unwrap();
    // Members other than admins cannot co-sign.
    let mut carol_log = sync(|| carol.client.fetch_log(org, None, None)).unwrap();
    let error = sync(|| carol.client.cosign_proposal(&mut carol_log, &proposal)).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Forbidden), "{error}");
    let proposals = sync(|| bob.client.proposals(org)).unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].1.body, promote);
    sync(|| bob.client.cosign_proposal(&mut bob_log, &proposal)).unwrap();
    assert_eq!(
        bob_log.state().members[&carol.user()],
        pasejo_protocol::log::Role::Admin
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_appends_are_retried_on_a_fresh_log() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let mut stale = log.clone();
    sync(|| {
        alice.client.append(
            &mut log,
            Body::TeamCreate {
                team: Id::random(),
                name: "a".into(),
            },
        )
    })
    .unwrap();
    let error = sync(|| {
        alice.client.append(
            &mut stale.clone(),
            Body::TeamCreate {
                team: Id::random(),
                name: "b".into(),
            },
        )
    })
    .unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Conflict), "{error}");
    sync(|| {
        alice.client.append_fresh(
            &mut stale,
            Body::TeamCreate {
                team: Id::random(),
                name: "b".into(),
            },
        )
    })
    .unwrap();
    assert_eq!(stale.state().teams.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn reviews_and_their_revocations_run_through_actions() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let carol = signed_in(&harness, "carol").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    sync(|| join(&alice, &mut log, &bob, RoleChange::Member));
    sync(|| join(&alice, &mut log, &carol, RoleChange::Member));
    let (ring, _) = sync(|| alice.client.create_store(&mut log, "s")).unwrap();
    let store = ring.store;
    sync(|| {
        alice.client.grant(
            &mut log,
            store,
            Principal::User(bob.user()),
            StoreRoleChange::Reader,
            None,
        )
    })
    .unwrap();
    let due = harness.now() + 86_400_000;
    let review = sync(|| {
        alice
            .client
            .open_review(org, store, vec![carol.user()], due)
    })
    .unwrap();
    sync(|| {
        carol.client.decide(
            org,
            review.id,
            Principal::User(bob.user()),
            Decision::Revoke,
        )
    })
    .unwrap();
    assert_eq!(sync(|| carol.client.reviews(org)).unwrap().len(), 1);
    sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    assert!(
        log.state()
            .store_role(store, bob.user(), harness.now())
            .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_audit_trail_verifies_and_detects_tampering() {
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 3\n".into(),
        ..Options::default()
    })
    .await;
    let alice = signed_in(&harness, "alice").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "s")).unwrap();
    let mut changes = Changes::default();
    changes.secrets.insert("a".into(), Some("1".into()));
    sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();
    let key = harness.state.keys.checkpoint.verifying_key().to_bytes();
    let verified = sync(|| alice.client.verify_audit(org, &key, Some(&log), None)).unwrap();
    assert!(
        verified.events > 5 && verified.checkpoints >= 2,
        "{verified:?}"
    );
    assert!(sync(|| alice.client.verify_audit(org, &[1; 32], None, None)).is_err());

    // Co-signing covers every checkpoint once.
    // Co-signing records events, which may produce further checkpoints;
    // repeating it signs only those and never one checkpoint twice.
    let before = sync(|| alice.client.checkpoints(org, 0)).unwrap().len();
    let signed = sync(|| alice.client.cosign_checkpoints(&log, &key)).unwrap();
    assert_eq!(signed, before);
    let mut rounds = 0;
    while sync(|| alice.client.cosign_checkpoints(&log, &key)).unwrap() > 0 {
        rounds += 1;
        assert!(rounds < 10, "co-signing does not converge");
    }
    let checkpoints = sync(|| alice.client.checkpoints(org, 0)).unwrap();
    let alice_key = alice.client.device.signing_key.verifying_key().to_bytes();
    for entry in &checkpoints {
        assert_eq!(
            entry.checkpoint.cosignatures.len(),
            1,
            "checkpoint {}",
            entry.seq
        );
        entry
            .checkpoint
            .verify_cosignature(alice.client.device.id, &alice_key)
            .unwrap();
    }

    // Continuing from the last verified point checks only new events.
    let again = sync(|| {
        alice
            .client
            .verify_audit(org, &key, Some(&log), verified.last)
    })
    .unwrap();
    assert!(again.events < verified.events);
    assert_eq!(again.cosignatures, checkpoints.len(), "{again:?}");

    // A forged co-signature, or one by a device the log does not name,
    // breaks verification.
    let seq = i64::try_from(checkpoints[0].seq).unwrap();
    let mut forged = checkpoints[0].checkpoint.cosignatures.clone();
    forged[0].signature = pasejo_protocol::encoding::Fixed([7; 64]);
    let set_cosignatures = |value: String| {
        let harness = &harness;
        async move {
            sqlx::query(
                &harness
                    .state
                    .db
                    .sql("UPDATE checkpoints SET cosignatures = $1 WHERE chain = $2 AND seq = $3"),
            )
            .bind(value)
            .bind(org.to_string())
            .bind(seq)
            .execute(harness.state.db.pool())
            .await
            .unwrap();
        }
    };
    set_cosignatures(serde_json::to_string(&forged).unwrap()).await;
    let error = sync(|| alice.client.verify_audit(org, &key, Some(&log), None)).unwrap_err();
    assert!(error.to_string().contains("does not verify"), "{error}");
    forged[0].device = Id::random();
    set_cosignatures(serde_json::to_string(&forged).unwrap()).await;
    let error = sync(|| alice.client.verify_audit(org, &key, Some(&log), None)).unwrap_err();
    assert!(error.to_string().contains("unknown device"), "{error}");
    // Without a log there is nothing to check co-signatures against.
    assert!(sync(|| alice.client.verify_audit(org, &key, None, None)).is_ok());
    set_cosignatures(serde_json::to_string(&checkpoints[0].checkpoint.cosignatures).unwrap()).await;

    // Rewriting a stored event breaks verification.
    let (seq, text): (i64, String) =
        sqlx::query_as("SELECT seq, event FROM audit_events WHERE chain = $1 ORDER BY seq LIMIT 1")
            .bind(org.to_string())
            .fetch_one(harness.state.db.pool())
            .await
            .unwrap();
    let mut event: serde_json::Value = serde_json::from_str(&text).unwrap();
    event["detail"] = "rewritten".into();
    sqlx::query(
        &harness
            .state
            .db
            .sql("UPDATE audit_events SET event = $1 WHERE chain = $2 AND seq = $3"),
    )
    .bind(event.to_string())
    .bind(org.to_string())
    .bind(seq)
    .execute(harness.state.db.pool())
    .await
    .unwrap();
    assert!(sync(|| alice.client.verify_audit(org, &key, Some(&log), None)).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn devices_sign_in_through_an_emergency_window() {
    let harness = Harness::start().await;
    let mut alice = signed_in(&harness, "alice").await;
    let org = Id::random();
    sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let error = sync(|| alice.client.emergency_login()).unwrap_err();
    assert_eq!(error.code(), Some(ErrorCode::Forbidden), "{error}");
    sqlx::query(
        &harness
            .state
            .db
            .sql("INSERT INTO settings (name, value) VALUES ($1, $2)"),
    )
    .bind("emergency_until")
    .bind((harness.now() + 600_000).to_string())
    .execute(harness.state.db.pool())
    .await
    .unwrap();
    let session = sync(|| alice.client.emergency_login()).unwrap();
    assert!(session.expires <= harness.now() + 600_000);
    assert_eq!(sync(|| alice.client.me()).unwrap().orgs.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn scim_tokens_are_managed_through_the_client() {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let org = Id::random();
    sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let created = sync(|| alice.client.create_scim_token(org)).unwrap();
    assert!(created.token.starts_with("pasejo-scim-"));
    assert_eq!(sync(|| alice.client.scim_tokens(org)).unwrap().len(), 1);
    sync(|| alice.client.revoke_scim_token(org, created.id)).unwrap();
    assert!(sync(|| alice.client.scim_tokens(org)).unwrap().is_empty());
    let users = sync(|| alice.client.users(org)).unwrap();
    assert_eq!(users.len(), 1);
}

/// A store with two secrets that `bob` reads, for tampering tests.
struct Shared {
    harness: Harness,
    alice: Device,
    bob: Device,
    org: Id,
    store: Id,
    log: OrgLog,
}

async fn shared() -> Shared {
    let harness = Harness::start().await;
    let alice = signed_in(&harness, "alice").await;
    let bob = signed_in(&harness, "bob").await;
    let org = Id::random();
    let mut log = sync(|| alice.client.create_org(org, "Acme", Settings::default())).unwrap();
    let (ring, snapshot) = sync(|| alice.client.create_store(&mut log, "s")).unwrap();
    let mut changes = Changes::default();
    changes
        .secrets
        .insert("one".into(), Some("1\nurl: https://one.example".into()));
    changes.secrets.insert("two".into(), Some("2".into()));
    sync(|| alice.client.commit(&log, &ring, &snapshot, &changes)).unwrap();
    sync(|| join(&alice, &mut log, &bob, RoleChange::Member));
    let store = ring.store;
    sync(|| {
        alice.client.grant(
            &mut log,
            store,
            Principal::User(bob.user()),
            StoreRoleChange::Reader,
            None,
        )
    })
    .unwrap();
    sync(|| {
        alice
            .client
            .run_actions(&mut log, &alice.identities(), &[0; 32])
    })
    .unwrap();
    Shared {
        harness,
        alice,
        bob,
        org,
        store,
        log,
    }
}

async fn execute(harness: &Harness, sql: &'static str, binds: Vec<Bind>) {
    let sql = harness.state.db.sql(sql);
    let mut query = sqlx::query(&sql);
    for bind in binds {
        query = match bind {
            Bind::Text(text) => query.bind(text),
            Bind::Blob(blob) => query.bind(blob),
            Bind::Int(int) => query.bind(int),
        };
    }
    let done = query.execute(harness.state.db.pool()).await.unwrap();
    assert!(done.rows_affected() > 0, "{sql}");
}

enum Bind {
    Text(String),
    Blob(Vec<u8>),
    Int(i64),
}

#[tokio::test(flavor = "multi_thread")]
async fn a_swapped_object_is_detected() {
    let s = shared().await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap();
    let view = sync(|| s.bob.client.index(&log, &ring, None)).unwrap();
    let one = view.plain.secrets["one"].id;
    let two = view.plain.secrets["two"].id;
    // Both objects open under the same keys, so only the hash in the
    // index tells them apart.
    let (other,): (Vec<u8>,) = sqlx::query_as(
        &s.harness
            .state
            .db
            .sql("SELECT ciphertext FROM objects WHERE store = $1 AND id = $2"),
    )
    .bind(s.store.to_string())
    .bind(two.to_string())
    .fetch_one(s.harness.state.db.pool())
    .await
    .unwrap();
    execute(
        &s.harness,
        "UPDATE objects SET ciphertext = $1 WHERE store = $2 AND id = $3",
        vec![
            Bind::Blob(other),
            Bind::Text(s.store.to_string()),
            Bind::Text(one.to_string()),
        ],
    )
    .await;
    let error = sync(|| {
        s.bob
            .client
            .objects(&log, &ring, &view, &[(Kind::Secret, "one".into())])
    })
    .unwrap_err();
    assert!(
        error.to_string().contains("differs from the index"),
        "{error}"
    );
    // The untouched object still reads.
    sync(|| {
        s.bob
            .client
            .objects(&log, &ring, &view, &[(Kind::Secret, "two".into())])
    })
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_replaced_lookup_object_is_detected() {
    let s = shared().await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap();
    let view = sync(|| s.bob.client.index(&log, &ring, None)).unwrap();
    execute(
        &s.harness,
        "UPDATE stores SET lookup = $1 WHERE id = $2",
        vec![Bind::Blob(vec![0; 64]), Bind::Text(s.store.to_string())],
    )
    .await;
    let error = sync(|| s.bob.client.lookup(&log, &ring, &view)).unwrap_err();
    assert!(
        matches!(error, pasejo_client::Error::Verification(_)),
        "{error}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rolled_back_index_is_refused() {
    let s = shared().await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap();
    let view = sync(|| s.bob.client.index(&log, &ring, None)).unwrap();
    assert_eq!(view.version, 2);
    execute(
        &s.harness,
        "UPDATE stores SET version = 1 WHERE id = $1",
        vec![Bind::Text(s.store.to_string())],
    )
    .await;
    // Without a pin the old version verifies; with one it is refused.
    assert_eq!(
        sync(|| s.bob.client.index(&log, &ring, None))
            .unwrap()
            .version,
        1
    );
    assert!(sync(|| s.bob.client.index(&log, &ring, view.pin())).is_err());
    // An index that vanishes entirely is refused too.
    execute(
        &s.harness,
        "UPDATE stores SET version = 0 WHERE id = $1",
        vec![Bind::Text(s.store.to_string())],
    )
    .await;
    assert!(sync(|| s.bob.client.index(&log, &ring, view.pin())).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn another_stores_index_is_refused() {
    let mut s = shared().await;
    let (other, _) = sync(|| s.alice.client.create_store(&mut s.log, "other")).unwrap();
    let (envelope,): (String,) = sqlx::query_as(
        &s.harness
            .state
            .db
            .sql("SELECT envelope FROM index_versions WHERE store = $1 AND version = 1"),
    )
    .bind(other.store.to_string())
    .fetch_one(s.harness.state.db.pool())
    .await
    .unwrap();
    execute(
        &s.harness,
        "UPDATE index_versions SET envelope = $1 WHERE store = $2 AND version = $3",
        vec![
            Bind::Text(envelope),
            Bind::Text(s.store.to_string()),
            Bind::Int(2),
        ],
    )
    .await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap();
    assert!(sync(|| s.bob.client.index(&log, &ring, None)).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_that_differ_from_the_logged_commitments_are_refused() {
    let s = shared().await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    // A wrapping of fresh keys, readable by bob, that the log never
    // committed to.
    let forged = pasejo_protocol::crypto::EpochKeys::generate();
    let recipient = s.bob.identity.to_public();
    let ciphertext = pasejo_protocol::wrapping::wrap_age(s.store, 0, &forged, &recipient).unwrap();
    execute(
        &s.harness,
        "UPDATE wrappings SET ciphertext = $1 WHERE store = $2 AND device = $3",
        vec![
            Bind::Blob(ciphertext),
            Bind::Text(s.store.to_string()),
            Bind::Text(s.bob.client.device.id.to_string()),
        ],
    )
    .await;
    let error = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap_err();
    assert!(error.to_string().contains("commit"), "{error}");
    // A wrapping for a different epoch's binding does not open either.
    let ring = sync(|| s.alice.client.keys(&s.log, s.store, &s.alice.identities())).unwrap();
    let rebound =
        pasejo_protocol::wrapping::wrap_age(Id::random(), 0, ring.current(), &recipient).unwrap();
    execute(
        &s.harness,
        "UPDATE wrappings SET ciphertext = $1 WHERE store = $2 AND device = $3",
        vec![
            Bind::Blob(rebound),
            Bind::Text(s.store.to_string()),
            Bind::Text(s.bob.client.device.id.to_string()),
        ],
    )
    .await;
    assert!(sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_withheld_object_is_detected() {
    let s = shared().await;
    let log = sync(|| s.bob.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| s.bob.client.keys(&log, s.store, &s.bob.identities())).unwrap();
    let view = sync(|| s.bob.client.index(&log, &ring, None)).unwrap();
    let one = view.plain.secrets["one"].id;
    execute(
        &s.harness,
        "DELETE FROM objects WHERE store = $1 AND id = $2",
        vec![Bind::Text(s.store.to_string()), Bind::Text(one.to_string())],
    )
    .await;
    assert!(
        sync(|| s
            .bob
            .client
            .objects(&log, &ring, &view, &[(Kind::Secret, "one".into())]))
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_names_only_device_reads_names_but_not_values() {
    let mut s = shared().await;
    let auditor = signed_in(&s.harness, "audrey").await;
    sync(|| join(&s.alice, &mut s.log, &auditor, RoleChange::Auditor));
    sync(|| {
        s.alice
            .client
            .run_actions(&mut s.log, &s.alice.identities(), &[0; 32])
    })
    .unwrap();
    let log = sync(|| auditor.client.fetch_log(s.org, None, None)).unwrap();
    let ring = sync(|| auditor.client.keys(&log, s.store, &auditor.identities())).unwrap();
    assert!(!ring.full());
    let view = sync(|| auditor.client.index(&log, &ring, None)).unwrap();
    assert_eq!(
        view.plain.secrets.keys().collect::<Vec<_>>(),
        ["one", "two"]
    );
    assert!(sync(|| auditor.client.lookup(&log, &ring, &view)).is_err());
    assert!(
        sync(|| auditor
            .client
            .objects(&log, &ring, &view, &[(Kind::Secret, "one".into())]))
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_changes_rewrite_lookup_texts() {
    let s = shared().await;
    let ring = sync(|| s.alice.client.keys(&s.log, s.store, &s.alice.identities())).unwrap();
    let view = sync(|| s.alice.client.index(&s.log, &ring, None)).unwrap();
    let mut settings = view.plain.settings.clone();
    settings.uri_keys = vec!["homepage".into()];
    let mut changes = Changes {
        settings: Some(settings),
        ..Changes::default()
    };
    for name in view.plain.secrets.keys() {
        changes.rewrite.insert((Kind::Secret, name.clone()));
    }
    let after = sync(|| s.alice.client.commit(&s.log, &ring, &view, &changes)).unwrap();
    assert_eq!(after.plain.settings.uri_keys, ["homepage"]);
    let lookup = sync(|| s.alice.client.lookup(&s.log, &ring, &after)).unwrap();
    assert_eq!(lookup.secrets["one"], "");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_recovery_identity_opens_every_store_and_only_admins_export_them() {
    let mut s = shared().await;
    let none = sync(|| s.alice.client.recovery_export(s.org)).unwrap_err();
    assert!(none.is(ErrorCode::NotFound), "{none}");

    let recovery = age::x25519::Identity::generate();
    let recipient = recovery.to_public().to_string();
    sync(|| {
        s.alice.client.append_fresh(
            &mut s.log,
            Body::RecoverySet {
                recipient: Some(recipient.clone()),
            },
        )
    })
    .unwrap();
    sync(|| {
        s.alice
            .client
            .run_actions(&mut s.log, &s.alice.identities(), &[0; 32])
    })
    .unwrap();

    let ring = sync(|| s.alice.client.keys(&s.log, s.store, &s.alice.identities())).unwrap();
    let view = sync(|| s.alice.client.index(&s.log, &ring, None)).unwrap();
    let mut changes = Changes::default();
    changes.secrets.insert("one".into(), Some("1 again".into()));
    changes.otp.insert("code".into(), Some(totp()));
    sync(|| s.alice.client.commit(&s.log, &ring, &view, &changes)).unwrap();
    let (_, empty) = sync(|| s.alice.client.create_store(&mut s.log, "empty")).unwrap();
    sync(|| {
        s.alice
            .client
            .run_actions(&mut s.log, &s.alice.identities(), &[0; 32])
    })
    .unwrap();

    let denied = sync(|| s.bob.client.recovery_export(s.org)).unwrap_err();
    assert!(denied.is(ErrorCode::Forbidden), "{denied}");

    let export = sync(|| s.alice.client.recovery_export(s.org)).unwrap();
    assert_eq!(export.stores.len(), 2);
    let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(recovery)];
    let recovered: Vec<_> = export
        .stores
        .iter()
        .map(|store| pasejo_client::store::recover(&s.log, store, &identities).unwrap())
        .collect();
    let team = recovered
        .iter()
        .find(|store| store.store == s.store)
        .unwrap();
    assert_eq!(team.label, "s");
    assert_eq!(
        team.values[&(Kind::Secret, "one".into())],
        secret("1 again")
    );
    assert_eq!(team.values[&(Kind::Secret, "two".into())], secret("2"));
    assert_eq!(
        team.values[&(Kind::Otp, "code".into())],
        ObjectPlain::Otp { otp: totp() }
    );
    let fresh = recovered
        .iter()
        .find(|store| store.store == empty.store)
        .unwrap();
    assert_eq!(fresh.label, "empty");
    assert!(fresh.values.is_empty());
    // Only the current version of each object travels.
    let exported = export
        .stores
        .iter()
        .find(|store| store.id == s.store)
        .unwrap();
    assert_eq!(exported.objects.len(), 3);

    let alice_identities = s.alice.identities();
    let wrong = pasejo_client::store::recover(&s.log, exported, &alice_identities);
    assert!(
        wrong.is_err(),
        "a member's own identity does not open the recovery wrapping"
    );

    let events = sync(|| s.alice.client.audit_events(s.org, 0)).unwrap();
    let outcomes: Vec<&str> = events
        .iter()
        .filter(|event| event.action == "recovery.export")
        .map(|event| event.outcome.as_str())
        .collect();
    assert!(outcomes.contains(&"ok"), "{outcomes:?}");
    assert!(outcomes.contains(&"denied"), "{outcomes:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_recovery_export_with_a_withheld_object_is_detected() {
    let mut s = shared().await;
    let recovery = age::x25519::Identity::generate();
    sync(|| {
        s.alice.client.append_fresh(
            &mut s.log,
            Body::RecoverySet {
                recipient: Some(recovery.to_public().to_string()),
            },
        )
    })
    .unwrap();
    sync(|| {
        s.alice
            .client
            .run_actions(&mut s.log, &s.alice.identities(), &[0; 32])
    })
    .unwrap();
    let mut export = sync(|| s.alice.client.recovery_export(s.org)).unwrap();
    let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(recovery)];
    let store = export
        .stores
        .iter_mut()
        .find(|store| store.id == s.store)
        .unwrap();
    assert!(pasejo_client::store::recover(&s.log, store, &identities).is_ok());
    store.objects.pop();
    let withheld = pasejo_client::store::recover(&s.log, store, &identities).unwrap_err();
    assert!(
        matches!(withheld, pasejo_client::Error::Verification(_)),
        "{withheld}"
    );
}
