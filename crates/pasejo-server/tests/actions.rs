// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pending actions: what the server computes from the log, enrollments,
//! wrappings, grants and checkpoints, and who sees which action.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::stores::Target;
use common::{Harness, Member, Options};
use pasejo_protocol::Id;
use pasejo_protocol::api::{Action, Actions};
use pasejo_protocol::crypto::EpochKeys;
use pasejo_protocol::log::{Body, Principal, RoleChange, Settings, StoreRoleChange};
use reqwest::StatusCode;
use serde_json::Value;

async fn actions(member: &Member, org: Id) -> Vec<Action> {
    member
        .client
        .get(&format!("/v1/orgs/{org}/actions"))
        .await
        .ok::<Actions>()
        .actions
}

fn count(actions: &[Action], predicate: impl Fn(&Action) -> bool) -> usize {
    actions.iter().filter(|action| predicate(action)).count()
}

#[tokio::test]
async fn enrollments_wait_for_an_approver() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    let dave = harness.member("dave").await;
    harness.join(&alice, &bob, org, RoleChange::Approver).await;
    harness.join(&alice, &carol, org, RoleChange::Member).await;
    dave.enroll(org).await.ok::<Value>();
    let approve = |action: &Action| matches!(action, Action::ApproveDevice { enrollment } if enrollment.device.id == dave.record.id);
    assert_eq!(count(&actions(&alice, org).await, approve), 1);
    assert_eq!(count(&actions(&bob, org).await, approve), 1);
    assert_eq!(count(&actions(&carol, org).await, approve), 0);
    // A pending device cannot list anything yet.
    dave.client
        .get(&format!("/v1/orgs/{org}/actions"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
    // Approval resolves the action.
    alice
        .append(
            org,
            Body::MemberRole {
                user: dave.user(),
                role: RoleChange::Member,
            },
        )
        .await
        .ok::<Value>();
    alice
        .append(
            org,
            Body::DeviceAdd {
                device: dave.record.clone(),
            },
        )
        .await
        .ok::<Value>();
    assert_eq!(count(&actions(&alice, org).await, approve), 0);
}

#[tokio::test]
async fn a_second_device_of_a_member_is_theirs_to_approve() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let laptop = Member::new(harness.sign_in("bob").await);
    laptop.enroll(org).await.ok::<Value>();
    let approve = |action: &Action| matches!(action, Action::ApproveDevice { enrollment } if enrollment.device.id == laptop.record.id);
    assert_eq!(count(&actions(&bob, org).await, approve), 1);
    bob.append(
        org,
        Body::DeviceAdd {
            device: laptop.record.clone(),
        },
    )
    .await
    .ok::<Value>();
    assert_eq!(count(&actions(&bob, org).await, approve), 0);
}

#[tokio::test]
async fn stores_without_keys_need_their_first_epoch() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let store = alice.create_store(org).await;
    let rotate = |action: &Action| matches!(action, Action::RotateEpoch { store: s, epoch: 0 } if *s == store);
    assert_eq!(count(&actions(&alice, org).await, rotate), 1);
    alice
        .start_epoch(
            org,
            store,
            0,
            &EpochKeys::generate(),
            None,
            &[Target::Age(&alice)],
        )
        .await;
    assert_eq!(count(&actions(&alice, org).await, rotate), 0);
}

#[tokio::test]
async fn new_readers_need_wrappings_and_former_readers_need_a_rotation() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    harness.join(&alice, &carol, org, RoleChange::Member).await;
    let store = alice.create_store(org).await;
    let keys = EpochKeys::generate();
    alice
        .start_epoch(org, store, 0, &keys, None, &[Target::Age(&alice)])
        .await;
    alice
        .append(
            org,
            Body::StoreGrant {
                store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::Reader,
                expires: None,
            },
        )
        .await
        .ok::<Value>();
    let wrap = |action: &Action| matches!(action, Action::WrapKeys { store: s, epoch: 0, devices } if *s == store && devices.contains(&bob.record.id));
    assert_eq!(count(&actions(&alice, org).await, wrap), 1);
    // Bob is a reader: he sees the wrap action, carol does not.
    assert_eq!(count(&actions(&bob, org).await, wrap), 1);
    assert_eq!(count(&actions(&carol, org).await, wrap), 0);

    alice
        .upload_epoch(
            org,
            store,
            0,
            &keys,
            None,
            &[Target::Age(&alice), Target::Age(&bob)],
        )
        .await;
    assert_eq!(count(&actions(&alice, org).await, wrap), 0);
    // Taking bob's access away calls for epoch 1.
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
    let rotate = |action: &Action| matches!(action, Action::RotateEpoch { store: s, epoch: 1 } if *s == store);
    assert_eq!(count(&actions(&alice, org).await, rotate), 1);
    assert_eq!(count(&actions(&bob, org).await, rotate), 0);
    alice
        .start_epoch(
            org,
            store,
            1,
            &EpochKeys::generate(),
            Some(&keys),
            &[Target::Age(&alice)],
        )
        .await;
    assert_eq!(count(&actions(&alice, org).await, rotate), 0);
}

#[tokio::test]
async fn expired_grants_are_listed_for_revocation() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let store = alice.create_store(org).await;
    let keys = EpochKeys::generate();
    alice
        .start_epoch(org, store, 0, &keys, None, &[Target::Age(&alice)])
        .await;
    let expires = harness.now() + 1_000;
    alice
        .append(
            org,
            Body::StoreGrant {
                store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::Reader,
                expires: Some(expires),
            },
        )
        .await
        .ok::<Value>();
    alice
        .upload_epoch(
            org,
            store,
            0,
            &keys,
            None,
            &[Target::Age(&alice), Target::Age(&bob)],
        )
        .await;
    let revoke = |action: &Action| matches!(action, Action::RevokeExpired { store: s, principal: Principal::User(u) } if *s == store && *u == bob.user());
    assert_eq!(count(&actions(&alice, org).await, revoke), 0);
    harness.advance(2_000);
    let pending = actions(&alice, org).await;
    assert_eq!(count(&pending, revoke), 1);
    // The expired reader's wrapping makes the epoch stale too.
    assert!(
        pending.iter().any(
            |action| matches!(action, Action::RotateEpoch { store: s, epoch: 1 } if *s == store)
        ),
        "{pending:?}"
    );
}

#[tokio::test]
async fn unsigned_checkpoints_ask_auditors_to_co_sign() {
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 2\n".into(),
        ..Options::default()
    })
    .await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let carol = harness.member("carol").await;
    harness.join(&alice, &carol, org, RoleChange::Auditor).await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    let cosign = |action: &Action| matches!(action, Action::CosignCheckpoint { .. });
    assert!(count(&actions(&carol, org).await, cosign) > 0);
    assert!(count(&actions(&alice, org).await, cosign) > 0);
    assert_eq!(count(&actions(&bob, org).await, cosign), 0);
}

#[tokio::test]
async fn membership_actions_are_for_admins_only() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    harness.join(&alice, &bob, org, RoleChange::Approver).await;
    let token: pasejo_protocol::api::ScimTokenCreated = alice
        .client
        .post_empty(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .ok();
    let reply = harness
        .http
        .post(format!("{}/scim/v2/{org}/Users", harness.url))
        .bearer_auth(&token.token)
        .body(r#"{"userName":"carol@example.com"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(reply.status(), StatusCode::CREATED);
    let join = |action: &Action| matches!(action, Action::JoinMember { email, .. } if email == "carol@example.com");
    assert_eq!(count(&actions(&alice, org).await, join), 1);
    assert_eq!(count(&actions(&bob, org).await, join), 0);
}

#[tokio::test]
async fn unknown_organizations_have_no_actions() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    alice
        .client
        .get(&format!("/v1/orgs/{}/actions", Id::random()))
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_org");
}
