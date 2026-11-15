// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Organizations, the membership log, enrollments and proposals.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{Harness, Member, Options};
use pasejo_protocol::Id;
use pasejo_protocol::api::{
    Append, Appended, Cosign, CreateOrg, Enrollment, EnrollmentStatus, Enrollments, LogPage, Me,
    Proposal, Proposals, ScimTokenCreated, Users,
};
use pasejo_protocol::encoding::Fixed;
use pasejo_protocol::envelope::{Context, Signature, sign};
use pasejo_protocol::log::{Body, RoleChange, Settings, genesis, next_entry, seal_entry};
use reqwest::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn a_founder_creates_an_organization_and_sees_it() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let me: Me = alice.client.get("/v1/me").await.ok();
    assert_eq!(me.orgs.len(), 1);
    assert_eq!(me.orgs[0].id, org);
    assert_eq!(me.orgs[0].name, "Acme");
    let log = alice.log(org).await;
    assert_eq!(log.entries().len(), 1);
    assert_eq!(log.state().members.len(), 1);
}

#[tokio::test]
async fn a_genesis_must_name_the_session_user_and_device() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    // Bob's record, signed by alice's key.
    let envelope = genesis(
        Id::random(),
        "Acme",
        bob.record.clone(),
        Settings::default(),
        harness.now(),
        &alice.client.key,
    )
    .unwrap();
    alice
        .client
        .post("/v1/orgs", &CreateOrg { genesis: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    // Alice's user with another signing key.
    let mut record = alice.record.clone();
    record.signing_key = bob.record.signing_key;
    let envelope = genesis(
        Id::random(),
        "Acme",
        record,
        Settings::default(),
        harness.now(),
        &bob.client.key,
    )
    .unwrap();
    alice
        .client
        .post("/v1/orgs", &CreateOrg { genesis: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn an_organization_identifier_is_taken_once() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let envelope = genesis(
        org,
        "Again",
        alice.record.clone(),
        Settings::default(),
        harness.now(),
        &alice.client.key,
    )
    .unwrap();
    alice
        .client
        .post("/v1/orgs", &CreateOrg { genesis: envelope })
        .await
        .expect(StatusCode::CONFLICT, "conflict");
}

#[tokio::test]
async fn the_creation_policy_limits_founders() {
    let harness = Harness::start_with(Options {
        extra: "[orgs]\ncreation = \"listed\"\ncreators = [\"Alice@Example.com\"]\n".into(),
        ..Options::default()
    })
    .await;
    let alice = harness.member("alice").await;
    harness.org(&alice, Settings::default()).await;
    let bob = harness.member("bob").await;
    let envelope = genesis(
        Id::random(),
        "Bobco",
        bob.record.clone(),
        Settings::default(),
        harness.now(),
        &bob.client.key,
    )
    .unwrap();
    bob.client
        .post("/v1/orgs", &CreateOrg { genesis: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");

    let closed = Harness::start_with(Options {
        extra: "[orgs]\ncreation = \"nobody\"\n".into(),
        ..Options::default()
    })
    .await;
    let carol = closed.member("carol").await;
    let envelope = genesis(
        Id::random(),
        "Carolco",
        carol.record.clone(),
        Settings::default(),
        closed.now(),
        &carol.client.key,
    )
    .unwrap();
    carol
        .client
        .post("/v1/orgs", &CreateOrg { genesis: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn members_join_through_enrollment_and_an_admin() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;

    // Strangers see nothing.
    bob.client
        .get(&format!("/v1/orgs/{org}/log"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let enrollment: Enrollment = bob.enroll(org).await.ok();
    assert_eq!(enrollment.fingerprint, bob.record.fingerprint());
    assert_eq!(enrollment.user_email, "bob@example.com");
    // Enrolling lets bob read the log to verify it before he is added.
    bob.client
        .get(&format!("/v1/orgs/{org}/log"))
        .await
        .ok::<LogPage>();
    let status: EnrollmentStatus = bob
        .client
        .get(&format!("/v1/orgs/{org}/enrollments/{}", bob.record.id))
        .await
        .ok();
    assert_eq!(
        status,
        EnrollmentStatus {
            pending: true,
            active: false
        }
    );
    // Alice sees the pending device and the fingerprint to compare.
    let pending: Enrollments = alice
        .client
        .get(&format!("/v1/orgs/{org}/enrollments"))
        .await
        .ok();
    assert_eq!(pending.enrollments.len(), 1);
    assert_eq!(pending.enrollments[0].device, bob.record);
    // Bob cannot act before he is in the log.
    bob.client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");

    alice
        .append(
            org,
            Body::MemberRole {
                user: bob.user(),
                role: RoleChange::Member,
            },
        )
        .await
        .ok::<Appended>();
    alice
        .append(
            org,
            Body::DeviceAdd {
                device: bob.record.clone(),
            },
        )
        .await
        .ok::<Appended>();
    let status: EnrollmentStatus = bob
        .client
        .get(&format!("/v1/orgs/{org}/enrollments/{}", bob.record.id))
        .await
        .ok();
    assert_eq!(
        status,
        EnrollmentStatus {
            pending: false,
            active: true
        }
    );
    let pending: Enrollments = alice
        .client
        .get(&format!("/v1/orgs/{org}/enrollments"))
        .await
        .ok();
    assert!(pending.enrollments.is_empty());
    bob.client
        .get(&format!("/v1/orgs/{org}/stores"))
        .await
        .ok::<Value>();
    let me: Me = bob.client.get("/v1/me").await.ok();
    assert_eq!(me.orgs.len(), 1);
}

#[tokio::test]
async fn enrollments_are_validated() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    // An active device does not enroll again.
    alice
        .enroll(org)
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    // A malformed recipient is refused.
    let mut broken = Member::new(bob.client.clone());
    broken.record.recipient = "not-a-recipient".into();
    broken
        .enroll(org)
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    // A device identifier is taken once.
    let mut duplicate = Member::new(bob.client.clone());
    duplicate.record.id = alice.record.id;
    duplicate
        .enroll(org)
        .await
        .expect(StatusCode::CONFLICT, "conflict");
    bob.enroll(org).await.ok::<Value>();
    bob.enroll(org)
        .await
        .expect(StatusCode::CONFLICT, "conflict");
    // Unknown organizations.
    bob.client
        .post(
            &format!("/v1/orgs/{}/enrollments", Id::random()),
            &serde_json::json!({}),
        )
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_org");
}

#[tokio::test]
async fn enrollment_status_is_private_to_its_user() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    bob.enroll(org).await.ok::<Value>();
    alice
        .client
        .get(&format!("/v1/orgs/{org}/enrollments/{}", bob.record.id))
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn only_devices_append_and_only_as_themselves() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    // Bob has no device in the log.
    let log = alice.log(org).await;
    let entry = next_entry(
        &log,
        bob.record.id,
        harness.now(),
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = seal_entry(&entry, &bob.client.key).unwrap();
    bob.client
        .post(&format!("/v1/orgs/{org}/log"), &Append { entry: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    // An entry naming alice's device as author, sent from bob's session.
    let log = alice.log(org).await;
    let entry = next_entry(
        &log,
        alice.record.id,
        harness.now(),
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = seal_entry(&entry, &alice.client.key).unwrap();
    bob.client
        .post(&format!("/v1/orgs/{org}/log"), &Append { entry: envelope })
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn the_log_refuses_what_its_rules_forbid() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    // A member cannot change roles.
    bob.append(
        org,
        Body::MemberRole {
            user: carol.user(),
            role: RoleChange::Member,
        },
    )
    .await
    .expect(StatusCode::BAD_REQUEST, "rejected");
    // The last admin cannot step down.
    alice
        .append(
            org,
            Body::MemberRole {
                user: alice.user(),
                role: RoleChange::Member,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "rejected");
    // A stale sequence number conflicts.
    let log = alice.log(org).await;
    let first = seal_entry(
        &next_entry(
            &log,
            alice.record.id,
            harness.now(),
            Body::StoreCreate {
                store: Id::random(),
            },
        ),
        &alice.client.key,
    )
    .unwrap();
    let second = seal_entry(
        &next_entry(
            &log,
            alice.record.id,
            harness.now(),
            Body::StoreCreate {
                store: Id::random(),
            },
        ),
        &alice.client.key,
    )
    .unwrap();
    alice
        .client
        .post(&format!("/v1/orgs/{org}/log"), &Append { entry: first })
        .await
        .ok::<Appended>();
    let reply = alice
        .client
        .post(&format!("/v1/orgs/{org}/log"), &Append { entry: second })
        .await;
    assert!(
        reply.status == StatusCode::CONFLICT || reply.code() == "rejected",
        "{}",
        reply.text()
    );
    // A tampered signature.
    let log = alice.log(org).await;
    let mut tampered = seal_entry(
        &next_entry(
            &log,
            alice.record.id,
            harness.now(),
            Body::StoreCreate {
                store: Id::random(),
            },
        ),
        &alice.client.key,
    )
    .unwrap();
    tampered.signatures[0].signature.0[3] ^= 1;
    alice
        .client
        .post(&format!("/v1/orgs/{org}/log"), &Append { entry: tampered })
        .await
        .expect(StatusCode::BAD_REQUEST, "rejected");
}

#[tokio::test]
async fn entries_with_a_far_off_time_are_refused() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let founded = harness.now();
    let post = |time| {
        let alice = &alice;
        async move {
            let log = alice.log(org).await;
            let mut entry = next_entry(
                &log,
                alice.record.id,
                time,
                Body::StoreCreate {
                    store: Id::random(),
                },
            );
            entry.time = time;
            let envelope = seal_entry(&entry, &alice.client.key).unwrap();
            alice
                .client
                .post(&format!("/v1/orgs/{org}/log"), &Append { entry: envelope })
                .await
        }
    };
    post(harness.now() + 301_000)
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    // Ten minutes later, an entry dated at the founding is still in order
    // for the log, but backdated for the server.
    harness.advance(600_000);
    post(founded)
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    post(harness.now()).await.ok::<Appended>();
}

#[tokio::test]
async fn revoking_a_device_ends_its_sessions() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    alice
        .append(
            org,
            Body::DeviceRevoke {
                device: bob.record.id,
            },
        )
        .await
        .ok::<Appended>();
    bob.client
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
    alice.client.get("/v1/me").await.ok::<Me>();
}

#[tokio::test]
async fn removing_a_member_ends_their_sessions() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    alice
        .append(
            org,
            Body::MemberRole {
                user: bob.user(),
                role: RoleChange::None,
            },
        )
        .await
        .ok::<Appended>();
    bob.client
        .get("/v1/me")
        .await
        .expect(StatusCode::UNAUTHORIZED, "unauthorized");
}

#[tokio::test]
async fn two_admin_changes_go_through_proposals() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let carol = harness.member("carol").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Admin).await;
    alice
        .append(
            org,
            Body::OrgSettings {
                settings: Settings {
                    two_admin: true,
                    ..Settings::default()
                },
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "rejected");
    // Turning the rule on already needs two admins.
    let entry = alice
        .sealed(
            org,
            Body::OrgSettings {
                settings: Settings {
                    two_admin: true,
                    ..Settings::default()
                },
            },
        )
        .await;
    let proposal: Proposal = alice
        .client
        .post(
            &format!("/v1/orgs/{org}/log/proposals"),
            &Append {
                entry: entry.clone(),
            },
        )
        .await
        .ok();
    let listed: Proposals = bob
        .client
        .get(&format!("/v1/orgs/{org}/log/proposals"))
        .await
        .ok();
    assert_eq!(listed.proposals, vec![proposal.clone()]);
    // Alice cannot co-sign her own proposal into acceptance.
    let own = Signature {
        device: alice.record.id,
        signature: Fixed(sign(Context::LogEntry, &entry.payload.0, &alice.client.key)),
    };
    alice
        .client
        .post(
            &format!("/v1/orgs/{org}/log/proposals/{}/cosign", proposal.id),
            &Cosign { signature: own },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "rejected");
    // A signature by another key under bob's device is refused.
    let forged = Signature {
        device: bob.record.id,
        signature: Fixed(sign(Context::LogEntry, &entry.payload.0, &carol.client.key)),
    };
    bob.client
        .post(
            &format!("/v1/orgs/{org}/log/proposals/{}/cosign", proposal.id),
            &Cosign { signature: forged },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
    let cosignature = Signature {
        device: bob.record.id,
        signature: Fixed(sign(Context::LogEntry, &entry.payload.0, &bob.client.key)),
    };
    let appended: Appended = bob
        .client
        .post(
            &format!("/v1/orgs/{org}/log/proposals/{}/cosign", proposal.id),
            &Cosign {
                signature: cosignature,
            },
        )
        .await
        .ok();
    assert_eq!(appended.seq, 3);
    assert!(alice.log(org).await.state().settings.two_admin);
    let listed: Proposals = bob
        .client
        .get(&format!("/v1/orgs/{org}/log/proposals"))
        .await
        .ok();
    assert!(listed.proposals.is_empty());

    // Now promoting carol needs both.
    carol.enroll(org).await.ok::<Value>();
    alice
        .append(
            org,
            Body::MemberRole {
                user: carol.user(),
                role: RoleChange::Admin,
            },
        )
        .await
        .expect(StatusCode::BAD_REQUEST, "rejected");
    // Entries that need one admin are not proposals.
    let entry = alice
        .sealed(
            org,
            Body::MemberRole {
                user: carol.user(),
                role: RoleChange::Member,
            },
        )
        .await;
    alice
        .client
        .post(&format!("/v1/orgs/{org}/log/proposals"), &Append { entry })
        .await
        .expect(StatusCode::BAD_REQUEST, "bad_request");
}

#[tokio::test]
async fn a_proposal_goes_stale_when_the_log_moves_on() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Admin).await;
    let entry = alice
        .sealed(
            org,
            Body::OrgSettings {
                settings: Settings {
                    two_admin: true,
                    ..Settings::default()
                },
            },
        )
        .await;
    let proposal: Proposal = alice
        .client
        .post(
            &format!("/v1/orgs/{org}/log/proposals"),
            &Append {
                entry: entry.clone(),
            },
        )
        .await
        .ok();
    alice
        .append(
            org,
            Body::StoreCreate {
                store: Id::random(),
            },
        )
        .await
        .ok::<Appended>();
    let cosignature = Signature {
        device: bob.record.id,
        signature: Fixed(sign(Context::LogEntry, &entry.payload.0, &bob.client.key)),
    };
    bob.client
        .post(
            &format!("/v1/orgs/{org}/log/proposals/{}/cosign", proposal.id),
            &Cosign {
                signature: cosignature,
            },
        )
        .await
        .expect(StatusCode::CONFLICT, "conflict");
    let listed: Proposals = alice
        .client
        .get(&format!("/v1/orgs/{org}/log/proposals"))
        .await
        .ok();
    assert!(listed.proposals.is_empty());
}

#[tokio::test]
async fn proposals_are_for_admins() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    bob.client
        .get(&format!("/v1/orgs/{org}/log/proposals"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
}

#[tokio::test]
async fn the_log_pages_after_a_sequence_number() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    for _ in 0..3 {
        alice
            .append(
                org,
                Body::StoreCreate {
                    store: Id::random(),
                },
            )
            .await
            .ok::<Appended>();
    }
    let all: LogPage = alice.client.get(&format!("/v1/orgs/{org}/log")).await.ok();
    assert_eq!(all.entries.len(), 4);
    let after: LogPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/log?since=1"))
        .await
        .ok();
    assert_eq!(after.entries, all.entries[2..].to_vec());
    let none: LogPage = alice
        .client
        .get(&format!("/v1/orgs/{org}/log?since=99"))
        .await
        .ok();
    assert!(none.entries.is_empty());
}

#[tokio::test]
async fn unknown_and_malformed_organizations() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    alice
        .client
        .get(&format!("/v1/orgs/{}/log", Id::random()))
        .await
        .expect(StatusCode::NOT_FOUND, "unknown_org");
    alice
        .client
        .get("/v1/orgs/not-hex/log")
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn organization_users_include_pending_ones() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    bob.enroll(org).await.ok::<Value>();
    let users: Users = alice
        .client
        .get(&format!("/v1/orgs/{org}/users"))
        .await
        .ok();
    let mut emails: Vec<String> = users.users.iter().map(|user| user.email.clone()).collect();
    emails.sort();
    assert_eq!(emails, ["alice@example.com", "bob@example.com"]);
    bob.client
        .get(&format!("/v1/orgs/{org}/users"))
        .await
        .expect(StatusCode::FORBIDDEN, "pending_device");
}

#[tokio::test]
async fn scim_tokens_are_for_admins_and_can_be_revoked() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let bob = harness.member("bob").await;
    let org = harness.org(&alice, Settings::default()).await;
    harness.join(&alice, &bob, org, RoleChange::Member).await;
    bob.client
        .post_empty(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    let created: ScimTokenCreated = alice
        .client
        .post_empty(&format!("/v1/orgs/{org}/scim-tokens"))
        .await
        .ok();
    assert!(created.token.starts_with("pasejo-scim-"));
    let scim = |token: &str| {
        harness
            .http
            .get(format!("{}/scim/v2/{org}/Users", harness.url))
            .header("authorization", format!("Bearer {token}"))
            .send()
    };
    assert_eq!(scim(&created.token).await.unwrap().status(), StatusCode::OK);
    bob.client
        .delete(&format!("/v1/orgs/{org}/scim-tokens/{}", created.id))
        .await
        .expect(StatusCode::FORBIDDEN, "forbidden");
    assert_eq!(
        alice
            .client
            .delete(&format!("/v1/orgs/{org}/scim-tokens/{}", created.id))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        scim(&created.token).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    alice
        .client
        .delete(&format!("/v1/orgs/{org}/scim-tokens/{}", created.id))
        .await
        .expect(StatusCode::NOT_FOUND, "not_found");
}

#[tokio::test]
async fn log_appends_are_audited_with_their_kind() {
    let harness = Harness::start().await;
    let alice = harness.member("alice").await;
    let org = harness.org(&alice, Settings::default()).await;
    let store = Id::random();
    alice
        .append(org, Body::StoreCreate { store })
        .await
        .ok::<Appended>();
    alice
        .append(
            org,
            Body::MemberRole {
                user: alice.user(),
                role: RoleChange::Member,
            },
        )
        .await;
    let events = pasejo_server::audit::events(&harness.state, &org.to_string(), 0, 100)
        .await
        .unwrap();
    let append = events
        .iter()
        .find(|event| event.action == "log.append" && event.outcome == "ok")
        .unwrap();
    assert_eq!(append.detail.as_deref(), Some("store.create"));
    assert_eq!(append.store, Some(store));
    assert_eq!(append.actor_device, Some(alice.record.id));
    let refused = events
        .iter()
        .find(|event| event.action == "log.append" && event.outcome == "denied")
        .unwrap();
    assert!(
        refused
            .detail
            .as_deref()
            .unwrap()
            .starts_with("member.role: ")
    );
    assert_eq!(
        events
            .iter()
            .find(|event| event.action == "org.create")
            .unwrap()
            .org,
        Some(org)
    );
    pasejo_protocol::checkpoint::verify_chain(events[0].prev_hash, events[0].seq, &events).unwrap();
}
