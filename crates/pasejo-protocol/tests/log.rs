// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Scenario tests of the membership log: every entry kind, every rule and
//! every rejection.

use ed25519_dalek::SigningKey;
use pasejo_protocol::crypto::{EpochKeys, data_commitment, names_commitment};
use pasejo_protocol::device::{DeviceKind, DeviceRecord, recovery_id};
use pasejo_protocol::encoding::{Fixed, Hash, Id, b64};
use pasejo_protocol::envelope::{Context, Envelope};
use pasejo_protocol::log::{
    Approval, Body, OrgLog, Principal, Rejection, RoleChange, Settings, StoreRole, StoreRoleChange,
    genesis, next_entry, seal_entry,
};
use pasejo_protocol::store::{IndexPayload, sign_index};
use pasejo_protocol::{Bytes, Error};
use rand_core::OsRng;

const AGE: &str = "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";

/// A device with its signing key.
#[derive(Clone)]
struct Dev {
    record: DeviceRecord,
    key: SigningKey,
}

impl Dev {
    fn new(user: Id, kind: DeviceKind) -> Self {
        let key = SigningKey::generate(&mut OsRng);
        let recipient = match kind {
            DeviceKind::Age => AGE.to_owned(),
            DeviceKind::Hpke => b64(&[9; 32]),
        };
        Self {
            record: DeviceRecord {
                id: Id::random(),
                user,
                kind,
                recipient,
                signing_key: Fixed(key.verifying_key().to_bytes()),
                label: "device".into(),
            },
            key,
        }
    }

    fn id(&self) -> Id {
        self.record.id
    }

    fn user(&self) -> Id {
        self.record.user
    }
}

/// An organization under test, with a clock that advances per entry.
struct Org {
    log: OrgLog,
    time: u64,
}

impl Org {
    fn new(founder: &Dev) -> Self {
        Self::with_settings(founder, Settings::default())
    }

    fn with_settings(founder: &Dev, settings: Settings) -> Self {
        let envelope = genesis(
            Id::random(),
            "Example",
            founder.record.clone(),
            settings,
            1_000,
            &founder.key,
        )
        .unwrap();
        Self {
            log: OrgLog::genesis(envelope).unwrap(),
            time: 1_000,
        }
    }

    fn envelope(&mut self, author: &Dev, body: Body) -> Envelope {
        self.time += 10;
        let entry = next_entry(&self.log, author.id(), self.time, body);
        seal_entry(&entry, &author.key).unwrap()
    }

    fn try_append(&mut self, author: &Dev, body: Body) -> Result<(), Rejection> {
        let envelope = self.envelope(author, body);
        self.log.append(envelope).map(|_| ())
    }

    fn append(&mut self, author: &Dev, body: Body) {
        self.try_append(author, body)
            .expect("entry should be accepted");
    }

    fn try_append_cosigned(
        &mut self,
        author: &Dev,
        cosigners: &[&Dev],
        body: Body,
    ) -> Result<(), Rejection> {
        let mut envelope = self.envelope(author, body);
        for cosigner in cosigners {
            envelope.add_signature(Context::LogEntry, cosigner.id(), &cosigner.key);
        }
        self.log.append(envelope).map(|_| ())
    }

    /// Adds a member with one age device, approved by `admin`.
    fn member(&mut self, admin: &Dev, role: RoleChange) -> Dev {
        let device = Dev::new(Id::random(), DeviceKind::Age);
        self.append(
            admin,
            Body::MemberRole {
                user: device.user(),
                role,
            },
        );
        self.append(
            admin,
            Body::DeviceAdd {
                device: device.record.clone(),
            },
        );
        device
    }

    fn store(&mut self, author: &Dev) -> Id {
        let store = Id::random();
        self.append(author, Body::StoreCreate { store });
        store
    }

    fn grant(&mut self, admin: &Dev, store: Id, principal: Principal, role: StoreRoleChange) {
        self.append(
            admin,
            Body::StoreGrant {
                store,
                principal,
                role,
                expires: None,
            },
        );
    }

    fn epoch_body(&self, store: Id, keys: &EpochKeys) -> Body {
        let state = self.log.state();
        let epoch = state.stores[&store].epochs.len() as u64;
        let entitled = state.entitled(store, self.time + 10);
        Body::StoreEpoch {
            store,
            epoch,
            names_commit: names_commitment(store, epoch, &keys.names),
            data_commit: data_commitment(store, epoch, keys.data().unwrap()),
            full: entitled.full.into_iter().collect(),
            names_only: entitled.names_only.into_iter().collect(),
        }
    }
}

fn founder() -> Dev {
    Dev::new(Id::random(), DeviceKind::Age)
}

// Genesis

#[test]
fn a_valid_genesis_makes_the_founder_admin_with_one_device() {
    let founder = founder();
    let org = Org::new(&founder);
    let state = org.log.state();
    assert_eq!(state.name, "Example");
    assert_eq!(state.members.len(), 1);
    assert_eq!(
        state.device_role(founder.id()),
        Some(pasejo_protocol::log::Role::Admin)
    );
    assert_eq!(state.next_seq, 1);
    assert_eq!(org.log.genesis_hash(), org.log.head());
}

#[test]
fn a_genesis_signed_by_another_key_is_rejected() {
    let founder = founder();
    let impostor = SigningKey::generate(&mut OsRng);
    let envelope = genesis(
        Id::random(),
        "Example",
        founder.record.clone(),
        Settings::default(),
        1,
        &impostor,
    )
    .unwrap();
    assert_eq!(
        OrgLog::genesis(envelope).unwrap_err(),
        Rejection::BadSignature
    );
}

#[test]
fn a_genesis_whose_device_belongs_to_someone_else_is_rejected() {
    let founder = founder();
    let mut record = founder.record.clone();
    record.user = Id::random();
    let entry = pasejo_protocol::log::Entry {
        org: Id::random(),
        seq: 0,
        prev: None,
        time: 1,
        author: record.id,
        body: Body::Genesis {
            name: "x".into(),
            founder: founder.user(),
            device: record,
            settings: Settings::default(),
        },
    };
    let envelope = seal_entry(&entry, &founder.key).unwrap();
    assert_eq!(OrgLog::genesis(envelope).unwrap_err(), Rejection::Genesis);
}

#[test]
fn a_genesis_with_a_second_signature_is_rejected() {
    let founder = founder();
    let other = founder_like();
    let mut envelope = genesis(
        Id::random(),
        "Example",
        founder.record.clone(),
        Settings::default(),
        1,
        &founder.key,
    )
    .unwrap();
    envelope.add_signature(Context::LogEntry, other.id(), &other.key);
    assert_eq!(
        OrgLog::genesis(envelope).unwrap_err(),
        Rejection::AuthorNotFirstSigner
    );
}

fn founder_like() -> Dev {
    Dev::new(Id::random(), DeviceKind::Age)
}

#[test]
fn a_genesis_with_an_empty_name_is_rejected() {
    let founder = founder();
    let envelope = genesis(
        Id::random(),
        " ",
        founder.record.clone(),
        Settings::default(),
        1,
        &founder.key,
    )
    .unwrap();
    assert_eq!(
        OrgLog::genesis(envelope).unwrap_err(),
        Rejection::InvalidName
    );
}

#[test]
fn a_log_must_start_with_a_genesis() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let envelope = org.envelope(
        &founder,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    assert_eq!(OrgLog::genesis(envelope).unwrap_err(), Rejection::Genesis);
    assert_eq!(
        OrgLog::replay(Vec::new()).unwrap_err().1,
        Rejection::Genesis
    );
}

// Chain rules

#[test]
fn entries_must_chain_in_sequence() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let mut entry = next_entry(
        &org.log,
        founder.id(),
        2_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    entry.seq = 5;
    let envelope = seal_entry(&entry, &founder.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::WrongSequence));

    let mut entry = next_entry(
        &org.log,
        founder.id(),
        2_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    entry.prev = Some(Hash::of(b"elsewhere"));
    let envelope = seal_entry(&entry, &founder.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::WrongPredecessor));

    let mut entry = next_entry(
        &org.log,
        founder.id(),
        2_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    entry.org = Id::random();
    let envelope = seal_entry(&entry, &founder.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::WrongOrganization));
    org.append(
        &founder,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
}

#[test]
fn time_never_goes_backwards() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let mut entry = next_entry(
        &org.log,
        founder.id(),
        5_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    entry.time = 10;
    let envelope = seal_entry(&entry, &founder.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::TimeWentBackwards));
}

#[test]
fn next_entry_clamps_a_lagging_clock_to_the_head() {
    let founder = founder();
    let mut org = Org::new(&founder);
    org.append(
        &founder,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let entry = next_entry(
        &org.log,
        founder.id(),
        1,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    assert_eq!(entry.time, org.log.state().last_time);
}

#[test]
fn a_second_genesis_is_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let result = org.try_append(
        &founder,
        Body::Genesis {
            name: "again".into(),
            founder: founder.user(),
            device: founder.record.clone(),
            settings: Settings::default(),
        },
    );
    assert_eq!(result, Err(Rejection::Genesis));
}

#[test]
fn the_author_must_sign_first() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let entry = next_entry(
        &org.log,
        founder.id(),
        9_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = Envelope::seal(Context::LogEntry, &entry, member.id(), &member.key).unwrap();
    assert_eq!(
        org.log.append(envelope),
        Err(Rejection::AuthorNotFirstSigner)
    );
}

#[test]
fn unknown_signers_bad_signatures_and_duplicates_are_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let stranger = founder_like();
    let entry = next_entry(
        &org.log,
        stranger.id(),
        9_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = seal_entry(&entry, &stranger.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::InactiveSigner));

    let entry = next_entry(
        &org.log,
        founder.id(),
        9_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = Envelope::seal(Context::LogEntry, &entry, founder.id(), &stranger.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::BadSignature));

    let entry = next_entry(
        &org.log,
        founder.id(),
        9_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let mut envelope = seal_entry(&entry, &founder.key).unwrap();
    envelope.add_signature(Context::LogEntry, founder.id(), &founder.key);
    assert_eq!(org.log.append(envelope), Err(Rejection::DuplicateSigner));

    let entry = next_entry(
        &org.log,
        founder.id(),
        9_000,
        Body::StoreCreate {
            store: Id::random(),
        },
    );
    let envelope = Envelope::seal(Context::Index, &entry, founder.id(), &founder.key).unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::BadSignature));
}

#[test]
fn malformed_payloads_are_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let envelope = Envelope::seal(
        Context::LogEntry,
        &serde_json::json!({"kind": "nonsense"}),
        founder.id(),
        &founder.key,
    )
    .unwrap();
    assert_eq!(org.log.append(envelope), Err(Rejection::Malformed));
}

#[test]
fn a_refused_entry_leaves_the_log_unchanged() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let before = org.log.state().clone();
    let result = org.try_append(&founder, Body::TeamDelete { team: Id::random() });
    assert_eq!(result, Err(Rejection::UnknownTeam));
    assert_eq!(org.log.state(), &before);
    assert_eq!(org.log.entries().len(), 1);
}

// Members and roles

#[test]
fn admins_add_members_and_change_roles() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Auditor,
        },
    );
    assert_eq!(
        org.log.state().members[&user],
        pasejo_protocol::log::Role::Auditor
    );
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Approver,
        },
    );
    assert_eq!(
        org.log.state().members[&user],
        pasejo_protocol::log::Role::Approver
    );
}

#[test]
fn members_cannot_change_roles() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let result = org.try_append(
        &member,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Admin,
        },
    );
    assert!(matches!(result, Err(Rejection::Unauthorized(_))));
}

#[test]
fn the_last_admin_cannot_leave_or_be_demoted() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let demote = org.try_append(
        &founder,
        Body::MemberRole {
            user: founder.user(),
            role: RoleChange::Member,
        },
    );
    assert_eq!(demote, Err(Rejection::LastAdmin));
    let remove = org.try_append(
        &founder,
        Body::MemberRole {
            user: founder.user(),
            role: RoleChange::None,
        },
    );
    assert_eq!(remove, Err(Rejection::LastAdmin));
    let second = org.member(&founder, RoleChange::Admin);
    org.append(
        &second,
        Body::MemberRole {
            user: founder.user(),
            role: RoleChange::None,
        },
    );
    assert!(org.log.state().active_device(founder.id()).is_none());
}

#[test]
fn removing_a_member_revokes_devices_teams_and_grants() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let team = Id::random();
    org.append(
        &founder,
        Body::TeamCreate {
            team,
            name: "ops".into(),
        },
    );
    org.append(
        &founder,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(member.user()),
        StoreRoleChange::Reader,
    );
    org.append(
        &founder,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::None,
        },
    );
    let state = org.log.state();
    assert!(!state.members.contains_key(&member.user()));
    assert!(state.active_device(member.id()).is_none());
    assert!(!state.in_team(team, member.user()));
    assert!(
        !state.stores[&store]
            .grants
            .contains_key(&Principal::User(member.user()))
    );
}

#[test]
fn removing_a_non_member_is_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let result = org.try_append(
        &founder,
        Body::MemberRole {
            user: Id::random(),
            role: RoleChange::None,
        },
    );
    assert_eq!(result, Err(Rejection::UnknownUser));
}

#[test]
fn readded_members_do_not_get_their_old_devices_back() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    org.append(
        &founder,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::None,
        },
    );
    org.append(
        &founder,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Member,
        },
    );
    assert!(org.log.state().active_device(member.id()).is_none());
    let again = org.try_append(
        &founder,
        Body::DeviceAdd {
            device: member.record.clone(),
        },
    );
    assert_eq!(again, Err(Rejection::DuplicateIdentifier));
}

// Two-admin rule

fn two_admin_org() -> (Org, Dev, Dev) {
    let founder = founder();
    let mut org = Org::new(&founder);
    let second = org.member(&founder, RoleChange::Admin);
    org.append(
        &founder,
        Body::OrgSettings {
            settings: Settings {
                two_admin: false,
                ..Settings::default()
            },
        },
    );
    let result = org.try_append_cosigned(
        &founder,
        &[&second],
        Body::OrgSettings {
            settings: Settings {
                two_admin: true,
                ..Settings::default()
            },
        },
    );
    result.unwrap();
    (org, founder, second)
}

#[test]
fn enabling_two_admin_needs_two_admins() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let result = org.try_append(
        &founder,
        Body::OrgSettings {
            settings: Settings {
                two_admin: true,
                ..Settings::default()
            },
        },
    );
    assert_eq!(result, Err(Rejection::SecondAdminRequired));
}

#[test]
fn with_two_admin_granting_admin_needs_a_cosigner() {
    let (mut org, founder, second) = two_admin_org();
    let user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Member,
        },
    );
    let alone = org.try_append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Admin,
        },
    );
    assert_eq!(alone, Err(Rejection::SecondAdminRequired));
    org.try_append_cosigned(
        &founder,
        &[&second],
        Body::MemberRole {
            user,
            role: RoleChange::Admin,
        },
    )
    .unwrap();
}

#[test]
fn a_cosigner_who_is_not_an_admin_does_not_count() {
    let (mut org, founder, _) = two_admin_org();
    let member = org.member(&founder, RoleChange::Member);
    let user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Member,
        },
    );
    let result = org.try_append_cosigned(
        &founder,
        &[&member],
        Body::MemberRole {
            user,
            role: RoleChange::Admin,
        },
    );
    assert_eq!(result, Err(Rejection::SecondAdminRequired));
}

#[test]
fn two_devices_of_one_admin_are_one_admin() {
    let (mut org, founder, _) = two_admin_org();
    let second_device = Dev::new(founder.user(), DeviceKind::Age);
    org.append(
        &founder,
        Body::DeviceAdd {
            device: second_device.record.clone(),
        },
    );
    let user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Member,
        },
    );
    let result = org.try_append_cosigned(
        &founder,
        &[&second_device],
        Body::MemberRole {
            user,
            role: RoleChange::Admin,
        },
    );
    assert_eq!(result, Err(Rejection::SecondAdminRequired));
}

#[test]
fn with_two_admin_recovery_and_store_deletion_need_a_cosigner() {
    let (mut org, founder, second) = two_admin_org();
    let alone = org.try_append(
        &founder,
        Body::RecoverySet {
            recipient: Some(AGE.into()),
        },
    );
    assert_eq!(alone, Err(Rejection::SecondAdminRequired));
    org.try_append_cosigned(
        &founder,
        &[&second],
        Body::RecoverySet {
            recipient: Some(AGE.into()),
        },
    )
    .unwrap();
    let store = org.store(&founder);
    let alone = org.try_append(&founder, Body::StoreDelete { store });
    assert_eq!(alone, Err(Rejection::SecondAdminRequired));
    org.try_append_cosigned(&founder, &[&second], Body::StoreDelete { store })
        .unwrap();
}

// Devices

#[test]
fn approvers_add_devices_of_other_members() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let approver = org.member(&founder, RoleChange::Approver);
    let user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user,
            role: RoleChange::Member,
        },
    );
    let device = Dev::new(user, DeviceKind::Age);
    org.append(
        &approver,
        Body::DeviceAdd {
            device: device.record.clone(),
        },
    );
    assert!(org.log.state().active_device(device.id()).is_some());
}

#[test]
fn members_add_their_own_further_devices_only() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let own = Dev::new(member.user(), DeviceKind::Hpke);
    org.append(
        &member,
        Body::DeviceAdd {
            device: own.record.clone(),
        },
    );
    let other_user = Id::random();
    org.append(
        &founder,
        Body::MemberRole {
            user: other_user,
            role: RoleChange::Member,
        },
    );
    let foreign = Dev::new(other_user, DeviceKind::Age);
    let result = org.try_append(
        &member,
        Body::DeviceAdd {
            device: foreign.record.clone(),
        },
    );
    assert!(matches!(result, Err(Rejection::Unauthorized(_))));
}

#[test]
fn devices_of_non_members_and_invalid_devices_are_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let outsider = Dev::new(Id::random(), DeviceKind::Age);
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: outsider.record.clone()
            }
        ),
        Err(Rejection::UnknownUser)
    );
    let mut bad_recipient = Dev::new(founder.user(), DeviceKind::Age);
    bad_recipient.record.recipient = "not a recipient".into();
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: bad_recipient.record.clone()
            }
        ),
        Err(Rejection::InvalidDevice)
    );
    let mut bad_key = Dev::new(founder.user(), DeviceKind::Age);
    let bytes = (0..=255u8)
        .map(|first| {
            let mut bytes = [0u8; 32];
            bytes[0] = first;
            bytes
        })
        .find(|bytes| ed25519_dalek::VerifyingKey::from_bytes(bytes).is_err())
        .expect("some encoding is off the curve");
    bad_key.record.signing_key = Fixed(bytes);
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: bad_key.record.clone()
            }
        ),
        Err(Rejection::InvalidDevice)
    );
    let mut empty_label = Dev::new(founder.user(), DeviceKind::Age);
    empty_label.record.label = String::new();
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: empty_label.record.clone()
            }
        ),
        Err(Rejection::InvalidDevice)
    );
}

#[test]
fn device_identifiers_are_never_reused() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let mut duplicate = Dev::new(founder.user(), DeviceKind::Age);
    duplicate.record.id = founder.id();
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: duplicate.record.clone()
            }
        ),
        Err(Rejection::DuplicateIdentifier)
    );
    let store = org.store(&founder);
    let mut collides = Dev::new(founder.user(), DeviceKind::Age);
    collides.record.id = store;
    assert_eq!(
        org.try_append(
            &founder,
            Body::DeviceAdd {
                device: collides.record.clone()
            }
        ),
        Err(Rejection::DuplicateIdentifier)
    );
}

#[test]
fn devices_revoke_themselves_and_admins_revoke_others() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let second = Dev::new(member.user(), DeviceKind::Age);
    org.append(
        &member,
        Body::DeviceAdd {
            device: second.record.clone(),
        },
    );
    org.append(
        &second,
        Body::DeviceRevoke {
            device: second.id(),
        },
    );
    assert!(org.log.state().active_device(second.id()).is_none());
    let other = org.member(&founder, RoleChange::Member);
    assert!(matches!(
        org.try_append(&member, Body::DeviceRevoke { device: other.id() }),
        Err(Rejection::Unauthorized(_))
    ));
    org.append(&founder, Body::DeviceRevoke { device: other.id() });
    assert_eq!(
        org.try_append(&founder, Body::DeviceRevoke { device: other.id() }),
        Err(Rejection::UnknownDevice)
    );
}

#[test]
fn revoked_devices_cannot_sign() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    org.append(
        &founder,
        Body::DeviceRevoke {
            device: member.id(),
        },
    );
    assert_eq!(
        org.try_append(
            &member,
            Body::StoreCreate {
                store: Id::random()
            }
        ),
        Err(Rejection::InactiveSigner)
    );
}

// Recovery

#[test]
fn the_recovery_recipient_is_validated_and_receives_full_keys() {
    let founder = founder();
    let mut org = Org::new(&founder);
    assert_eq!(
        org.try_append(
            &founder,
            Body::RecoverySet {
                recipient: Some("bogus".into())
            }
        ),
        Err(Rejection::InvalidRecovery)
    );
    org.append(
        &founder,
        Body::RecoverySet {
            recipient: Some(AGE.into()),
        },
    );
    let store = org.store(&founder);
    let entitled = org.log.state().entitled(store, org.time);
    assert!(entitled.full.contains(&recovery_id(AGE)));
    assert!(org.log.state().is_recovery(recovery_id(AGE)));
    org.append(&founder, Body::RecoverySet { recipient: None });
    assert!(
        !org.log
            .state()
            .entitled(store, org.time)
            .full
            .contains(&recovery_id(AGE))
    );
}

#[test]
fn members_cannot_set_the_recovery_recipient() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    assert!(matches!(
        org.try_append(
            &member,
            Body::RecoverySet {
                recipient: Some(AGE.into())
            }
        ),
        Err(Rejection::Unauthorized(_))
    ));
}

// Teams

#[test]
fn teams_are_managed_by_admins() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let team = Id::random();
    assert!(matches!(
        org.try_append(
            &member,
            Body::TeamCreate {
                team,
                name: "ops".into()
            }
        ),
        Err(Rejection::Unauthorized(_))
    ));
    assert_eq!(
        org.try_append(
            &founder,
            Body::TeamCreate {
                team,
                name: "\n".into()
            }
        ),
        Err(Rejection::InvalidName)
    );
    org.append(
        &founder,
        Body::TeamCreate {
            team,
            name: "ops".into(),
        },
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::TeamCreate {
                team,
                name: "ops".into()
            }
        ),
        Err(Rejection::DuplicateIdentifier)
    );
    org.append(
        &founder,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    assert!(org.log.state().in_team(team, member.user()));
    assert_eq!(
        org.try_append(
            &founder,
            Body::TeamMember {
                team,
                user: Id::random(),
                member: true
            }
        ),
        Err(Rejection::UnknownUser)
    );
    org.append(
        &founder,
        Body::TeamMember {
            team,
            user: member.user(),
            member: false,
        },
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::TeamMember {
                team,
                user: member.user(),
                member: false
            }
        ),
        Err(Rejection::UnknownUser)
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::TeamMember {
                team: Id::random(),
                user: member.user(),
                member: true
            }
        ),
        Err(Rejection::UnknownTeam)
    );
}

#[test]
fn deleting_a_team_removes_its_grants() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let team = Id::random();
    org.append(
        &founder,
        Body::TeamCreate {
            team,
            name: "ops".into(),
        },
    );
    org.append(
        &founder,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::Team(team),
        StoreRoleChange::Writer,
    );
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        Some(StoreRole::Writer)
    );
    org.append(&founder, Body::TeamDelete { team });
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        None
    );
    assert_eq!(
        org.try_append(&founder, Body::TeamDelete { team }),
        Err(Rejection::UnknownTeam)
    );
}

// Stores and grants

#[test]
fn the_creator_of_a_store_becomes_its_admin() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&member);
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        Some(StoreRole::Admin)
    );
    assert_eq!(
        org.log.state().store_role(store, founder.user(), org.time),
        None
    );
    assert_eq!(
        org.try_append(&member, Body::StoreCreate { store }),
        Err(Rejection::DuplicateIdentifier)
    );
}

#[test]
fn store_admins_grant_and_revoke() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(member.user()),
        StoreRoleChange::Writer,
    );
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        Some(StoreRole::Writer)
    );
    assert!(matches!(
        org.try_append(
            &member,
            Body::StoreGrant {
                store,
                principal: Principal::User(member.user()),
                role: StoreRoleChange::Admin,
                expires: None,
            }
        ),
        Err(Rejection::Unauthorized(_))
    ));
    org.grant(
        &founder,
        store,
        Principal::User(member.user()),
        StoreRoleChange::None,
    );
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        None
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::User(member.user()),
                role: StoreRoleChange::None,
                expires: None,
            }
        ),
        Err(Rejection::UnknownGrant)
    );
}

#[test]
fn org_admins_cannot_grant_themselves_into_a_store_with_an_admin() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&member);
    assert!(matches!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::User(founder.user()),
                role: StoreRoleChange::Reader,
                expires: None,
            }
        ),
        Err(Rejection::Unauthorized(_))
    ));
}

#[test]
fn org_admins_rescue_a_store_whose_admins_left() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&member);
    org.append(
        &founder,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::None,
        },
    );
    assert!(!org.log.state().store_has_admin(store, org.time));
    org.grant(
        &founder,
        store,
        Principal::User(founder.user()),
        StoreRoleChange::Admin,
    );
    assert_eq!(
        org.log.state().store_role(store, founder.user(), org.time),
        Some(StoreRole::Admin)
    );
}

#[test]
fn the_last_admin_grant_of_a_store_cannot_be_removed() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let store = org.store(&founder);
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::User(founder.user()),
                role: StoreRoleChange::Reader,
                expires: None,
            }
        ),
        Err(Rejection::LastStoreAdmin)
    );
}

#[test]
fn grants_to_non_members_or_unknown_teams_are_rejected() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let store = org.store(&founder);
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::User(Id::random()),
                role: StoreRoleChange::Reader,
                expires: None,
            }
        ),
        Err(Rejection::UnknownUser)
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::Team(Id::random()),
                role: StoreRoleChange::Reader,
                expires: None,
            }
        ),
        Err(Rejection::UnknownTeam)
    );
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store: Id::random(),
                principal: Principal::User(founder.user()),
                role: StoreRoleChange::Reader,
                expires: None,
            }
        ),
        Err(Rejection::UnknownStore)
    );
}

#[test]
fn grants_expire() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    let expires = org.time + 1_000;
    org.append(
        &founder,
        Body::StoreGrant {
            store,
            principal: Principal::User(member.user()),
            role: StoreRoleChange::Reader,
            expires: Some(expires),
        },
    );
    let state = org.log.state();
    assert_eq!(
        state.store_role(store, member.user(), expires - 1),
        Some(StoreRole::Reader)
    );
    assert_eq!(state.store_role(store, member.user(), expires), None);
    assert!(!state.entitled(store, expires).full.contains(&member.id()));
    assert_eq!(
        org.try_append(
            &founder,
            Body::StoreGrant {
                store,
                principal: Principal::User(member.user()),
                role: StoreRoleChange::Reader,
                expires: Some(1),
            }
        ),
        Err(Rejection::ExpiryInPast)
    );
}

#[test]
fn team_grants_and_direct_grants_combine_to_the_strongest() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let team = Id::random();
    org.append(
        &founder,
        Body::TeamCreate {
            team,
            name: "ops".into(),
        },
    );
    org.append(
        &founder,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(member.user()),
        StoreRoleChange::Reader,
    );
    org.grant(
        &founder,
        store,
        Principal::Team(team),
        StoreRoleChange::Writer,
    );
    assert_eq!(
        org.log.state().store_role(store, member.user(), org.time),
        Some(StoreRole::Writer)
    );
}

#[test]
fn deleted_stores_accept_nothing_and_grant_nothing() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let store = org.store(&founder);
    org.append(&founder, Body::StoreDelete { store });
    assert_eq!(
        org.log.state().store_role(store, founder.user(), org.time),
        None
    );
    assert_eq!(
        org.try_append(&founder, Body::StoreDelete { store }),
        Err(Rejection::UnknownStore)
    );
    assert!(org.log.state().entitled(store, org.time).full.is_empty());
}

// Epochs

#[test]
fn epochs_commit_keys_to_the_entitled_devices() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let reader = org.member(&founder, RoleChange::Member);
    let auditor = org.member(&founder, RoleChange::Auditor);
    let browser = Dev::new(reader.user(), DeviceKind::Hpke);
    org.append(
        &reader,
        Body::DeviceAdd {
            device: browser.record.clone(),
        },
    );
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(reader.user()),
        StoreRoleChange::Reader,
    );
    let keys = EpochKeys::generate();
    let body = org.epoch_body(store, &keys);
    org.append(&founder, body);
    let record = org.log.epoch(store, 0).unwrap();
    assert!(record.full.contains(&founder.id()));
    assert!(record.full.contains(&reader.id()));
    assert!(record.names_only.contains(&browser.id()));
    assert!(record.names_only.contains(&auditor.id()));
    assert!(!record.full.contains(&browser.id()));
    assert_eq!(record.names_commit, names_commitment(store, 0, &keys.names));
}

#[test]
fn epochs_refuse_unentitled_recipients() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let outsider = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    let keys = EpochKeys::generate();
    let Body::StoreEpoch {
        names_commit,
        data_commit,
        ..
    } = org.epoch_body(store, &keys)
    else {
        unreachable!()
    };
    let result = org.try_append(
        &founder,
        Body::StoreEpoch {
            store,
            epoch: 0,
            names_commit,
            data_commit,
            full: vec![founder.id(), outsider.id()],
            names_only: vec![],
        },
    );
    assert_eq!(result, Err(Rejection::UnentitledRecipient));
}

#[test]
fn browser_devices_never_appear_in_the_full_set() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let browser = Dev::new(founder.user(), DeviceKind::Hpke);
    org.append(
        &founder,
        Body::DeviceAdd {
            device: browser.record.clone(),
        },
    );
    let store = org.store(&founder);
    let keys = EpochKeys::generate();
    let result = org.try_append(
        &founder,
        Body::StoreEpoch {
            store,
            epoch: 0,
            names_commit: names_commitment(store, 0, &keys.names),
            data_commit: data_commitment(store, 0, keys.data().unwrap()),
            full: vec![founder.id(), browser.id()],
            names_only: vec![],
        },
    );
    assert_eq!(result, Err(Rejection::UnentitledRecipient));
}

#[test]
fn a_device_cannot_be_in_both_sets() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let store = org.store(&founder);
    let keys = EpochKeys::generate();
    let result = org.try_append(
        &founder,
        Body::StoreEpoch {
            store,
            epoch: 0,
            names_commit: names_commitment(store, 0, &keys.names),
            data_commit: data_commitment(store, 0, keys.data().unwrap()),
            full: vec![founder.id()],
            names_only: vec![founder.id()],
        },
    );
    assert_eq!(result, Err(Rejection::UnentitledRecipient));
}

#[test]
fn epochs_are_numbered_consecutively_and_rotated_by_store_admins() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let writer = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(writer.user()),
        StoreRoleChange::Writer,
    );
    let keys = EpochKeys::generate();
    let body = org.epoch_body(store, &keys);
    org.append(&founder, body);
    let body = org.epoch_body(store, &keys);
    assert!(matches!(
        org.try_append(&writer, body),
        Err(Rejection::Unauthorized(_))
    ));
    let Body::StoreEpoch {
        names_commit,
        data_commit,
        full,
        names_only,
        ..
    } = org.epoch_body(store, &keys)
    else {
        unreachable!()
    };
    let skipped = org.try_append(
        &founder,
        Body::StoreEpoch {
            store,
            epoch: 5,
            names_commit,
            data_commit,
            full,
            names_only,
        },
    );
    assert_eq!(skipped, Err(Rejection::WrongEpoch));
    let body = org.epoch_body(store, &keys);
    org.append(&founder, body);
    assert_eq!(org.log.state().stores[&store].current_epoch(), Some(1));
}

// Index authorship

fn index_payload(org: &Org, store: Id, version: u64, time: u64) -> IndexPayload {
    IndexPayload {
        store,
        version,
        prev: None,
        epoch: 0,
        log_head: org.log.head(),
        time,
        index: Bytes(vec![1, 2, 3]),
        lookup_hash: None,
        changed: vec![],
        deleted: vec![],
    }
}

#[test]
fn indexes_by_writers_are_accepted_and_by_readers_refused() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let reader = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(reader.user()),
        StoreRoleChange::Reader,
    );
    let body = org.epoch_body(store, &EpochKeys::generate());
    org.append(&founder, body);
    let payload = index_payload(&org, store, 1, org.time);
    let by_admin = sign_index(&payload, founder.id(), &founder.key).unwrap();
    assert_eq!(org.log.check_index(&by_admin).unwrap(), payload);
    let by_reader = sign_index(&payload, reader.id(), &reader.key).unwrap();
    assert!(org.log.check_index(&by_reader).is_err());
    let forged = sign_index(&payload, founder.id(), &reader.key).unwrap();
    assert_eq!(org.log.check_index(&forged), Err(Error::Signature));
}

#[test]
fn an_index_stays_valid_after_its_author_is_revoked() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let writer = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(writer.user()),
        StoreRoleChange::Writer,
    );
    let body = org.epoch_body(store, &EpochKeys::generate());
    org.append(&founder, body);
    let payload = index_payload(&org, store, 1, org.time);
    let envelope = sign_index(&payload, writer.id(), &writer.key).unwrap();
    org.append(
        &founder,
        Body::DeviceRevoke {
            device: writer.id(),
        },
    );
    assert!(org.log.check_index(&envelope).is_ok());
    let later = index_payload(&org, store, 2, org.time);
    let after_revocation = sign_index(&later, writer.id(), &writer.key).unwrap();
    assert!(org.log.check_index(&after_revocation).is_err());
}

#[test]
fn indexes_naming_unknown_heads_or_epochs_are_refused() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let store = org.store(&founder);
    let mut payload = index_payload(&org, store, 1, org.time);
    let no_epoch = sign_index(&payload, founder.id(), &founder.key).unwrap();
    assert!(org.log.check_index(&no_epoch).is_err());
    let body = org.epoch_body(store, &EpochKeys::generate());
    org.append(&founder, body);
    payload.log_head = Hash::of(b"split view");
    let unknown_head = sign_index(&payload, founder.id(), &founder.key).unwrap();
    assert!(org.log.check_index(&unknown_head).is_err());
}

// Replay

#[test]
fn replay_reaches_the_same_state_as_appending() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let member = org.member(&founder, RoleChange::Member);
    let store = org.store(&founder);
    org.grant(
        &founder,
        store,
        Principal::User(member.user()),
        StoreRoleChange::Writer,
    );
    let body = org.epoch_body(store, &EpochKeys::generate());
    org.append(&founder, body);
    let replayed = OrgLog::replay(org.log.entries().to_vec()).unwrap();
    assert_eq!(replayed.state(), org.log.state());
    assert_eq!(replayed.head(), org.log.head());
}

#[test]
fn replay_reports_the_first_bad_entry() {
    let founder = founder();
    let mut org = Org::new(&founder);
    org.store(&founder);
    org.store(&founder);
    let mut entries = org.log.entries().to_vec();
    entries[2].payload.0[3] ^= 1;
    let (seq, _) = OrgLog::replay(entries).unwrap_err();
    assert_eq!(seq, 2);
}

#[test]
fn earlier_states_are_reconstructed() {
    let founder = founder();
    let mut org = Org::new(&founder);
    let first_head = org.log.head();
    let store = org.store(&founder);
    assert!(org.log.state_at(0).unwrap().stores.is_empty());
    assert!(org.log.state_at(1).unwrap().stores.contains_key(&store));
    assert!(org.log.state_at(2).is_err());
    assert_eq!(org.log.seq_of(&first_head), Some(0));
    assert_eq!(org.log.seq_of(&Hash::of(b"nope")), None);
}

#[test]
fn settings_survive_replay() {
    let founder = founder();
    let mut org = Org::with_settings(
        &founder,
        Settings {
            device_approval: Approval::Automatic,
            two_admin: false,
            auto_join: true,
            offline_reads: false,
        },
    );
    org.store(&founder);
    let replayed = OrgLog::replay(org.log.entries().to_vec()).unwrap();
    assert_eq!(
        replayed.state().settings.device_approval,
        Approval::Automatic
    );
    assert!(replayed.state().settings.auto_join);
}

#[test]
fn entries_round_trip_through_json_with_their_kind() {
    let founder = founder();
    let org = Org::new(&founder);
    let entry: pasejo_protocol::log::Entry = org.log.entries()[0].open().unwrap();
    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["kind"], "org.genesis");
    assert_eq!(json["body"]["name"], "Example");
    assert_eq!(entry.body.kind(), "org.genesis");
}

#[test]
fn offline_read_periods_follow_the_settings_entries() {
    let founder = founder();
    let allowed = Settings {
        offline_reads: true,
        ..Settings::default()
    };
    let mut org = Org::with_settings(&founder, allowed);
    assert_eq!(org.log.offline_read_periods(), [(1_000, None)]);
    org.append(
        &founder,
        Body::OrgSettings {
            settings: Settings {
                auto_join: true,
                ..allowed
            },
        },
    );
    assert_eq!(org.log.offline_read_periods(), [(1_000, None)]);
    org.append(
        &founder,
        Body::OrgSettings {
            settings: Settings::default(),
        },
    );
    org.store(&founder);
    org.append(&founder, Body::OrgSettings { settings: allowed });
    assert_eq!(
        org.log.offline_read_periods(),
        [(1_000, Some(1_020)), (1_040, None)]
    );
    assert!(org.log.offline_reads_allowed_at(1_000));
    assert!(org.log.offline_reads_allowed_at(1_019));
    assert!(!org.log.offline_reads_allowed_at(1_020));
    assert!(!org.log.offline_reads_allowed_at(999));
    assert!(org.log.offline_reads_allowed_at(u64::MAX));
    let replayed = OrgLog::replay(org.log.entries().to_vec()).unwrap();
    assert!(replayed.state().settings.offline_reads);
}

#[test]
fn offline_reads_are_omitted_from_settings_until_allowed() {
    let json = serde_json::to_value(Settings::default()).unwrap();
    assert_eq!(json.get("offline_reads"), None);
    let parsed: Settings = serde_json::from_value(json).unwrap();
    assert!(!parsed.offline_reads);
    let allowed = Settings {
        offline_reads: true,
        ..Settings::default()
    };
    assert_eq!(
        serde_json::to_value(allowed).unwrap()["offline_reads"],
        true
    );
}

#[test]
fn a_log_that_never_allowed_offline_reads_has_no_periods() {
    let founder = founder();
    let mut org = Org::new(&founder);
    org.store(&founder);
    assert_eq!(org.log.offline_read_periods(), []);
    assert!(!org.log.offline_reads_allowed_at(1_010));
}
