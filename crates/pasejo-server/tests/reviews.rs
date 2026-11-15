// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Access reviews: who opens them, who decides, and how revocations turn
//! into pending actions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Harness, Member};
use pasejo_protocol::Id;
use pasejo_protocol::api::{
    Action, Actions, Decision, Review, ReviewDecision, ReviewRequest, Reviews,
};
use pasejo_protocol::log::{Body, Principal, RoleChange, Settings, StoreRole, StoreRoleChange};
use reqwest::StatusCode;
use serde_json::Value;

struct Setup {
    harness: Harness,
    alice: Member,
    bob: Member,
    carol: Member,
    dave: Member,
    org: Id,
    store: Id,
    team: Id,
}

/// Alice administers; bob reads the store; carol reviews; dave is in a
/// team with write access that carol also belongs to.
async fn setup() -> Setup {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    let dave = harness.member("dave").await;
    for member in [&bob, &carol, &dave] {
        harness.join(&alice, member, org, RoleChange::Member).await;
    }
    let store = alice.create_store(org).await;
    let team = Id::random();
    for body in [
        Body::StoreGrant {
            store,
            principal: Principal::User(bob.user()),
            role: StoreRoleChange::Reader,
            expires: None,
        },
        Body::TeamCreate {
            team,
            name: "ops".into(),
        },
        Body::TeamMember {
            team,
            user: dave.user(),
            member: true,
        },
        Body::TeamMember {
            team,
            user: carol.user(),
            member: true,
        },
        Body::StoreGrant {
            store,
            principal: Principal::Team(team),
            role: StoreRoleChange::Writer,
            expires: None,
        },
    ] {
        alice.append(org, body).await.ok::<Value>();
    }
    Setup {
        harness,
        alice,
        bob,
        carol,
        dave,
        org,
        store,
        team,
    }
}

fn path(org: Id) -> String {
    format!("/v1/orgs/{org}/reviews")
}

async fn open(setup: &Setup, reviewers: Vec<Id>) -> Review {
    let reply = setup
        .alice
        .client
        .post(
            &path(setup.org),
            &ReviewRequest {
                store: setup.store,
                reviewers,
                due: setup.harness.now() + 86_400_000,
            },
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    reply.json()
}

async fn decide(
    member: &Member,
    org: Id,
    review: Id,
    principal: Principal,
    decision: Decision,
) -> common::Reply {
    member
        .client
        .post(
            &format!("{}/{review}/decisions", path(org)),
            &ReviewDecision {
                principal,
                decision,
            },
        )
        .await
}

#[tokio::test]
async fn a_review_lists_every_grant_of_the_store() {
    let setup = setup().await;
    let review = open(&setup, vec![setup.carol.user()]).await;
    assert!(!review.complete);
    assert_eq!(review.created_by, setup.alice.user());
    let mut principals: Vec<(Principal, StoreRole)> = review
        .items
        .iter()
        .map(|item| (item.principal, item.role))
        .collect();
    principals.sort_by_key(|(principal, _)| format!("{principal:?}"));
    // StoreCreate made alice an admin.
    assert_eq!(principals.len(), 3);
    assert!(principals.contains(&(Principal::User(setup.alice.user()), StoreRole::Admin)));
    assert!(principals.contains(&(Principal::User(setup.bob.user()), StoreRole::Reader)));
    assert!(principals.contains(&(Principal::Team(setup.team), StoreRole::Writer)));
    assert!(review.items.iter().all(|item| item.decision.is_none()));
}

#[tokio::test]
async fn reviews_are_opened_by_admins_with_member_reviewers_and_a_future_due_date() {
    let setup = setup().await;
    let Setup {
        harness,
        alice,
        bob,
        carol,
        org,
        store,
        ..
    } = &setup;
    let due = harness.now() + 60_000;
    bob.client
        .post(
            &path(*org),
            &ReviewRequest {
                store: *store,
                reviewers: vec![carol.user()],
                due,
            },
        )
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    alice
        .client
        .post(
            &path(*org),
            &ReviewRequest {
                store: *store,
                reviewers: vec![],
                due,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    alice
        .client
        .post(
            &path(*org),
            &ReviewRequest {
                store: *store,
                reviewers: vec![Id::random()],
                due,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    alice
        .client
        .post(
            &path(*org),
            &ReviewRequest {
                store: *store,
                reviewers: vec![carol.user()],
                due: harness.now() - 1,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    alice
        .client
        .post(
            &path(*org),
            &ReviewRequest {
                store: Id::random(),
                reviewers: vec![carol.user()],
                due,
            },
        )
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_store");
    // A store admin who is no organization admin may open one.
    alice
        .append(
            *org,
            Body::StoreGrant {
                store: *store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::Admin,
                expires: None,
            },
        )
        .await
        .ok::<Value>();
    let reply = bob
        .client
        .post(
            &path(*org),
            &ReviewRequest {
                store: *store,
                reviewers: vec![carol.user()],
                due,
            },
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
}

#[tokio::test]
async fn reviewers_decide_and_revocations_become_actions() {
    let setup = setup().await;
    let review = open(&setup, vec![setup.carol.user()]).await;
    let Setup {
        alice,
        bob,
        carol,
        dave,
        org,
        store,
        team,
        ..
    } = &setup;
    // Only reviewers decide.
    decide(
        dave,
        *org,
        review.id,
        Principal::User(bob.user()),
        Decision::Revoke,
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    decide(
        alice,
        *org,
        review.id,
        Principal::User(bob.user()),
        Decision::Revoke,
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    decide(
        carol,
        *org,
        review.id,
        Principal::User(dave.user()),
        Decision::Revoke,
    )
    .await
    .expect(StatusCode::NOT_FOUND, "not_found");
    decide(
        carol,
        *org,
        Id::random(),
        Principal::User(bob.user()),
        Decision::Revoke,
    )
    .await
    .expect(StatusCode::NOT_FOUND, "not_found");

    let decided: Review = decide(
        carol,
        *org,
        review.id,
        Principal::User(bob.user()),
        Decision::Revoke,
    )
    .await
    .ok();
    let item = decided
        .items
        .iter()
        .find(|item| item.principal == Principal::User(bob.user()))
        .unwrap();
    assert_eq!(
        (item.decision, item.decided_by),
        (Some(Decision::Revoke), Some(carol.user()))
    );
    let revoke = |action: &Action| matches!(action, Action::RevokeGrant { principal: Principal::User(u), review: r, .. } if *u == bob.user() && *r == review.id);
    let pending: Actions = alice
        .client
        .get(&format!("/v1/orgs/{org}/actions"))
        .await
        .ok();
    assert_eq!(
        pending
            .actions
            .iter()
            .filter(|action| revoke(action))
            .count(),
        1
    );
    // Revoking the grant resolves the action.
    alice
        .append(
            *org,
            Body::StoreGrant {
                store: *store,
                principal: Principal::User(bob.user()),
                role: StoreRoleChange::None,
                expires: None,
            },
        )
        .await
        .ok::<Value>();
    let pending: Actions = alice
        .client
        .get(&format!("/v1/orgs/{org}/actions"))
        .await
        .ok();
    assert_eq!(
        pending
            .actions
            .iter()
            .filter(|action| revoke(action))
            .count(),
        0
    );

    let kept: Review = decide(
        carol,
        *org,
        review.id,
        Principal::User(alice.user()),
        Decision::Keep,
    )
    .await
    .ok();
    assert!(!kept.complete);
    let _ = team;
}

#[tokio::test]
async fn reviewers_do_not_decide_on_their_own_access() {
    let setup = setup().await;
    let Setup {
        alice,
        carol,
        dave,
        org,
        team,
        ..
    } = &setup;
    let review = open(&setup, vec![carol.user(), dave.user(), alice.user()]).await;
    // Carol and dave are in the team; alice holds a direct grant.
    decide(
        carol,
        *org,
        review.id,
        Principal::Team(*team),
        Decision::Keep,
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    decide(
        dave,
        *org,
        review.id,
        Principal::Team(*team),
        Decision::Keep,
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    decide(
        alice,
        *org,
        review.id,
        Principal::User(alice.user()),
        Decision::Keep,
    )
    .await
    .expect(StatusCode::FORBIDDEN, "forbidden");
    // Someone outside the team decides on it.
    decide(
        alice,
        *org,
        review.id,
        Principal::Team(*team),
        Decision::Keep,
    )
    .await
    .ok::<Review>();
    decide(
        carol,
        *org,
        review.id,
        Principal::User(alice.user()),
        Decision::Keep,
    )
    .await
    .ok::<Review>();
    let complete: Review = decide(
        dave,
        *org,
        review.id,
        Principal::User(setup.bob.user()),
        Decision::Keep,
    )
    .await
    .ok();
    assert!(complete.complete);
}

#[tokio::test]
async fn reviews_are_visible_to_those_involved() {
    let setup = setup().await;
    let review = open(&setup, vec![setup.carol.user()]).await;
    let Setup {
        harness,
        alice,
        bob,
        carol,
        org,
        ..
    } = &setup;
    let auditor = harness.member("erin").await;
    harness
        .join(alice, &auditor, *org, RoleChange::Auditor)
        .await;
    for member in [alice, carol, &auditor] {
        let listed: Reviews = member.client.get(&path(*org)).await.ok();
        assert_eq!(
            listed.reviews.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![review.id]
        );
    }
    let listed: Reviews = bob.client.get(&path(*org)).await.ok();
    assert!(listed.reviews.is_empty());
    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 1000)
        .await
        .unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.action == "review.create" && event.store == Some(setup.store))
    );
}
