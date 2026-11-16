// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Vectors shared with the browser console's JavaScript implementation.
//!
//! `vectors/console.json` holds membership-log scenarios with the verdict on
//! every entry and the resulting state, a store with two epochs, its index
//! versions and a browser wrapping, audit events, a checkpoint, a request
//! proof, a session claim and padding lengths. This test verifies the
//! committed file with this crate, and `crates/pasejo-server/web/tests`
//! verifies the same file with the console's modules, so both
//! implementations agree on every case. `PASEJO_WRITE_VECTORS=1` rewrites
//! the file before verifying it.

#![allow(clippy::unwrap_used, clippy::too_many_lines)]

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use pasejo_protocol::api::ChainLink;
use pasejo_protocol::checkpoint::{AuditEvent, Checkpoint, CheckpointPayload, verify_chain};
use pasejo_protocol::crypto::{
    EpochKeys, Key, data_commitment, derive, names_commitment, open_chain, seal_chain,
};
use pasejo_protocol::device::{DeviceKind, DeviceRecord, org_fingerprint, recovery_id};
use pasejo_protocol::encoding::{Fixed, Hash, Id, b64, unb64};
use pasejo_protocol::envelope::{Context, Envelope, Signature, TokenClaim, sign};
use pasejo_protocol::log::{
    Approval, Body, OrgLog, OrgState, Principal, Rejection, RoleChange, Settings, StoreRoleChange,
    genesis, next_entry, seal_entry,
};
use pasejo_protocol::padding::padme;
use pasejo_protocol::request::message;
use pasejo_protocol::store::{
    IndexEntry, IndexPayload, IndexPlain, Pin, StoreSettings, check_chain, open_index, seal_index,
    sign_index,
};
use pasejo_protocol::wrapping::{hpke_keypair, unwrap_hpke, wrap_hpke, wrap_hpke_with_rng};
use pasejo_protocol::{Bytes, Error};
use rand_core::{CryptoRng, RngCore};
use serde_json::{Value, json};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/vectors/console.json");
const AGE: &str = "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";
const RECOVERY: &str = "age1lggyhqrw2nlhcxprm67z43rta597azn8gknawjehu9d9dl0jq3yqqvfafg";

#[test]
fn the_committed_vectors_verify() {
    if std::env::var_os("PASEJO_WRITE_VECTORS").is_some() {
        let text = serde_json::to_string_pretty(&generate()).unwrap();
        std::fs::write(PATH, text + "\n").unwrap();
    }
    let vectors: Value = serde_json::from_str(&std::fs::read_to_string(PATH).unwrap()).unwrap();
    verify_scenarios(&vectors["scenarios"]);
    verify_geneses(&vectors["geneses"]);
    verify_store(&vectors["store"]);
    verify_audit(&vectors["audit"]);
    verify_request(&vectors["request"]);
    verify_claim(&vectors["claim"]);
    verify_padding(&vectors["padding"]);
    verify_derive(&vectors["derive"]);
    verify_hpke(&vectors["hpke"]);
    verify_fingerprints(&vectors["fingerprints"]);
}

// ---------------------------------------------------------------------------
// Shared helpers

fn rejection_code(rejection: &Rejection) -> String {
    let debug = format!("{rejection:?}");
    debug.split('(').next().unwrap_or_default().to_owned()
}

fn error_code(error: &Error) -> String {
    match error {
        Error::Invalid(message) => (*message).to_owned(),
        Error::Signature => "signature".into(),
        Error::Log(rejection) => rejection_code(rejection),
        other => format!("{other:?}"),
    }
}

fn envelope(value: &Value) -> Envelope {
    serde_json::from_value(value.clone()).unwrap()
}

fn bytes(value: &Value) -> Vec<u8> {
    unb64(value.as_str().unwrap()).unwrap()
}

fn key32(value: &Value) -> [u8; 32] {
    bytes(value).try_into().unwrap()
}

fn id(value: &Value) -> Id {
    value.as_str().unwrap().parse().unwrap()
}

fn sorted(ids: impl IntoIterator<Item = Id>) -> Vec<String> {
    let mut list: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
    list.sort();
    list
}

fn principal_key(principal: &Principal) -> String {
    match principal {
        Principal::User(id) => format!("user:{id}"),
        Principal::Team(id) => format!("team:{id}"),
    }
}

/// The state as both implementations summarize it.
fn summary(state: &OrgState) -> Value {
    let members: BTreeMap<String, Value> = state
        .members
        .iter()
        .map(|(user, role)| (user.to_string(), serde_json::to_value(role).unwrap()))
        .collect();
    let devices: BTreeMap<String, Value> = state
        .devices
        .iter()
        .map(|(id, device)| {
            (
                id.to_string(),
                json!({
                    "active": device.active,
                    "user": device.record.user.to_string(),
                    "kind": device.record.kind,
                }),
            )
        })
        .collect();
    let teams: BTreeMap<String, Value> = state
        .teams
        .iter()
        .map(|(id, team)| {
            (
                id.to_string(),
                json!({"name": team.name, "members": sorted(team.members.iter().copied())}),
            )
        })
        .collect();
    let stores: BTreeMap<String, Value> = state
        .stores
        .iter()
        .map(|(id, store)| {
            let grants: BTreeMap<String, Value> = store
                .grants
                .iter()
                .map(|(principal, grant)| {
                    (
                        principal_key(principal),
                        json!({"role": grant.role, "expires": grant.expires}),
                    )
                })
                .collect();
            let epochs: Vec<Value> = store
                .epochs
                .iter()
                .map(|epoch| {
                    json!({
                        "names_commit": epoch.names_commit,
                        "data_commit": epoch.data_commit,
                        "full": sorted(epoch.full.iter().copied()),
                        "names_only": sorted(epoch.names_only.iter().copied()),
                        "seq": epoch.seq,
                        "time": epoch.time,
                    })
                })
                .collect();
            let entitled = state.entitled(*id, state.last_time);
            (
                id.to_string(),
                json!({
                    "deleted": store.deleted,
                    "created_seq": store.created_seq,
                    "grants": grants,
                    "epochs": epochs,
                    "entitled": {
                        "full": sorted(entitled.full.iter().copied()),
                        "names_only": sorted(entitled.names_only.iter().copied()),
                    },
                }),
            )
        })
        .collect();
    json!({
        "org": state.org.to_string(),
        "name": state.name,
        "settings": state.settings,
        "members": members,
        "devices": devices,
        "recovery": state.recovery,
        "teams": teams,
        "stores": stores,
        "next_seq": state.next_seq,
        "head": state.head,
        "last_time": state.last_time,
    })
}

// ---------------------------------------------------------------------------
// Generation

#[derive(Clone)]
struct Dev {
    record: DeviceRecord,
    key: SigningKey,
}

impl Dev {
    fn new(seed: u8, user: Id, kind: DeviceKind) -> Self {
        let key = SigningKey::from_bytes(&[seed; 32]);
        let recipient = match kind {
            DeviceKind::Age => AGE.to_owned(),
            DeviceKind::Hpke => b64(&[seed; 32]),
        };
        Self {
            record: DeviceRecord {
                id: Id([seed; 16]),
                user,
                kind,
                recipient,
                signing_key: Fixed(key.verifying_key().to_bytes()),
                label: format!("device {seed}"),
            },
            key,
        }
    }

    const fn id(&self) -> Id {
        self.record.id
    }

    const fn user(&self) -> Id {
        self.record.user
    }
}

const fn user(n: u8) -> Id {
    Id([0xa0 + n; 16])
}

struct Scenario {
    name: &'static str,
    genesis: Envelope,
    log: OrgLog,
    time: u64,
    steps: Vec<Value>,
}

impl Scenario {
    fn new(name: &'static str, founder: &Dev, settings: Settings) -> Self {
        let envelope = genesis(
            Id([0x0f; 16]),
            "Example",
            founder.record.clone(),
            settings,
            1_000,
            &founder.key,
        )
        .unwrap();
        Self {
            name,
            genesis: envelope.clone(),
            log: OrgLog::genesis(envelope).unwrap(),
            time: 1_000,
            steps: Vec::new(),
        }
    }

    fn sealed(&mut self, author: &Dev, cosigners: &[&Dev], body: Body) -> Envelope {
        self.time += 10;
        let entry = next_entry(&self.log, author.id(), self.time, body);
        let mut envelope = seal_entry(&entry, &author.key).unwrap();
        for cosigner in cosigners {
            envelope.add_signature(Context::LogEntry, cosigner.id(), &cosigner.key);
        }
        envelope
    }

    fn raw(&mut self, note: &str, envelope: &Envelope) -> bool {
        self.record(note, envelope, true)
    }

    /// Records an entry with the log's verdict on it. `serialized` is false
    /// for a payload built from a JSON value, whose keys are sorted, so the
    /// console test does not expect its own serialization to reproduce it.
    fn record(&mut self, note: &str, envelope: &Envelope, serialized: bool) -> bool {
        let result = self.log.append(envelope.clone());
        let verdict = match &result {
            Ok(_) => "ok".to_owned(),
            Err(rejection) => rejection_code(rejection),
        };
        self.steps.push(json!({
            "note": note,
            "envelope": envelope,
            "result": verdict,
            "serialized": serialized,
        }));
        result.is_ok()
    }

    fn step(&mut self, note: &str, author: &Dev, body: Body) -> bool {
        self.cosigned(note, author, &[], body)
    }

    fn ok(&mut self, note: &str, author: &Dev, body: Body) {
        assert!(self.step(note, author, body), "{}: {note}", self.name);
    }

    fn refused(&mut self, note: &str, author: &Dev, body: Body) {
        assert!(!self.step(note, author, body), "{}: {note}", self.name);
    }

    fn cosigned(&mut self, note: &str, author: &Dev, cosigners: &[&Dev], body: Body) -> bool {
        let envelope = self.sealed(author, cosigners, body);
        self.raw(note, &envelope)
    }

    /// Adds a member with one age device.
    fn member(&mut self, admin: &Dev, seed: u8, role: RoleChange) -> Dev {
        let device = Dev::new(seed, user(seed), DeviceKind::Age);
        self.ok(
            "add member",
            admin,
            Body::MemberRole {
                user: device.user(),
                role,
            },
        );
        self.ok(
            "add member device",
            admin,
            Body::DeviceAdd {
                device: device.record.clone(),
            },
        );
        device
    }

    fn finish(self) -> Value {
        json!({
            "name": self.name,
            "genesis": self.genesis,
            "steps": self.steps,
            "state": summary(self.log.state()),
        })
    }
}

fn founder() -> Dev {
    Dev::new(1, user(1), DeviceKind::Age)
}

fn epoch_body(store: Id, epoch: u64, full: Vec<Id>, names_only: Vec<Id>) -> Body {
    let keys = EpochKeys::generate();
    Body::StoreEpoch {
        store,
        epoch,
        names_commit: names_commitment(store, epoch, &keys.names),
        data_commit: data_commitment(store, epoch, keys.data().unwrap()),
        full,
        names_only,
    }
}

fn roles_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("roles", &admin, Settings::default());
    let member = s.member(&admin, 2, RoleChange::Member);
    let approver = s.member(&admin, 3, RoleChange::Approver);
    let auditor = s.member(&admin, 4, RoleChange::Auditor);
    s.refused(
        "members cannot change roles",
        &member,
        Body::MemberRole {
            user: auditor.user(),
            role: RoleChange::Admin,
        },
    );
    s.refused(
        "approvers cannot change roles",
        &approver,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Approver,
        },
    );
    s.refused(
        "the last admin cannot step down",
        &admin,
        Body::MemberRole {
            user: admin.user(),
            role: RoleChange::Member,
        },
    );
    s.refused(
        "removing a non-member",
        &admin,
        Body::MemberRole {
            user: user(9),
            role: RoleChange::None,
        },
    );
    s.ok(
        "promote a second admin",
        &admin,
        Body::MemberRole {
            user: auditor.user(),
            role: RoleChange::Admin,
        },
    );
    s.ok(
        "the founder steps down",
        &admin,
        Body::MemberRole {
            user: admin.user(),
            role: RoleChange::Member,
        },
    );
    s.refused(
        "a former admin cannot change roles",
        &admin,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Admin,
        },
    );
    let team = Id([0x71; 16]);
    s.ok(
        "create a team",
        &auditor,
        Body::TeamCreate {
            team,
            name: "Ops".into(),
        },
    );
    s.ok(
        "add the member to the team",
        &auditor,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    let store = Id([0x51; 16]);
    s.ok(
        "the member creates a store",
        &member,
        Body::StoreCreate { store },
    );
    s.ok(
        "the member grants the team",
        &member,
        Body::StoreGrant {
            store,
            principal: Principal::Team(team),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.ok(
        "the member grants the approver",
        &member,
        Body::StoreGrant {
            store,
            principal: Principal::User(approver.user()),
            role: StoreRoleChange::Admin,
            expires: None,
        },
    );
    s.ok(
        "remove the member",
        &auditor,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::None,
        },
    );
    s.refused(
        "a removed member's device cannot sign",
        &member,
        Body::StoreCreate {
            store: Id([0x52; 16]),
        },
    );
    s.ok(
        "re-add the member",
        &auditor,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Member,
        },
    );
    s.finish()
}

fn devices_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("devices", &admin, Settings::default());
    let member = s.member(&admin, 2, RoleChange::Member);
    let approver = s.member(&admin, 3, RoleChange::Approver);
    let laptop = Dev::new(20, member.user(), DeviceKind::Age);
    s.ok(
        "approvers add devices of other members",
        &approver,
        Body::DeviceAdd {
            device: laptop.record.clone(),
        },
    );
    let browser = Dev::new(21, member.user(), DeviceKind::Hpke);
    s.ok(
        "members add their own devices",
        &member,
        Body::DeviceAdd {
            device: browser.record.clone(),
        },
    );
    let foreign = Dev::new(22, approver.user(), DeviceKind::Age);
    s.refused(
        "members cannot add devices of others",
        &member,
        Body::DeviceAdd {
            device: foreign.record,
        },
    );
    let stranger = Dev::new(23, user(9), DeviceKind::Age);
    s.refused(
        "devices of non-members",
        &admin,
        Body::DeviceAdd {
            device: stranger.record,
        },
    );
    let mut unlabelled = Dev::new(24, member.user(), DeviceKind::Age);
    unlabelled.record.label = " \t".into();
    s.refused(
        "a blank label",
        &admin,
        Body::DeviceAdd {
            device: unlabelled.record,
        },
    );
    let mut controlled = Dev::new(25, member.user(), DeviceKind::Age);
    controlled.record.label = "line\u{85}break".into();
    s.refused(
        "a label with a control character",
        &admin,
        Body::DeviceAdd {
            device: controlled.record,
        },
    );
    let mut long = Dev::new(26, member.user(), DeviceKind::Age);
    long.record.label = "ä".repeat(101);
    s.refused(
        "a label longer than 200 bytes",
        &admin,
        Body::DeviceAdd {
            device: long.record,
        },
    );
    let mut bom = Dev::new(27, member.user(), DeviceKind::Age);
    bom.record.label = "\u{feff}".into();
    s.ok(
        "a byte order mark is no whitespace",
        &admin,
        Body::DeviceAdd { device: bom.record },
    );
    let mut bad_hpke = Dev::new(28, member.user(), DeviceKind::Hpke);
    bad_hpke.record.recipient = b64(&[1; 31]);
    s.refused(
        "an hpke recipient of the wrong length",
        &admin,
        Body::DeviceAdd {
            device: bad_hpke.record,
        },
    );
    let mut bad_age = Dev::new(29, member.user(), DeviceKind::Age);
    bad_age.record.recipient = "rsa-key".into();
    s.refused(
        "an age recipient of the wrong shape",
        &admin,
        Body::DeviceAdd {
            device: bad_age.record,
        },
    );
    let mut ssh = Dev::new(30, member.user(), DeviceKind::Age);
    ssh.record.recipient = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGl1".into();
    s.ok(
        "an ssh recipient",
        &admin,
        Body::DeviceAdd { device: ssh.record },
    );
    let mut reused = Dev::new(31, member.user(), DeviceKind::Age);
    reused.record.id = laptop.id();
    s.refused(
        "a device identifier in use",
        &admin,
        Body::DeviceAdd {
            device: reused.record,
        },
    );
    s.ok(
        "a device revokes itself",
        &laptop,
        Body::DeviceRevoke {
            device: laptop.id(),
        },
    );
    s.refused(
        "a revoked device cannot sign",
        &laptop,
        Body::DeviceRevoke {
            device: browser.id(),
        },
    );
    s.refused(
        "revoking a revoked device",
        &admin,
        Body::DeviceRevoke {
            device: laptop.id(),
        },
    );
    s.refused(
        "members cannot revoke devices of others",
        &member,
        Body::DeviceRevoke {
            device: approver.id(),
        },
    );
    s.ok(
        "admins revoke devices of others",
        &admin,
        Body::DeviceRevoke {
            device: browser.id(),
        },
    );
    let mut again = Dev::new(32, member.user(), DeviceKind::Age);
    again.record.id = laptop.id();
    s.refused(
        "revoked identifiers are never reused",
        &admin,
        Body::DeviceAdd {
            device: again.record,
        },
    );
    s.finish()
}

fn signatures_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("signatures", &admin, Settings::default());
    let member = s.member(&admin, 2, RoleChange::Member);
    let store = Id([0x51; 16]);

    let entry = next_entry(&s.log, admin.id(), s.time + 10, Body::StoreCreate { store });
    let mut wrong_seq = entry.clone();
    wrong_seq.seq += 1;
    let envelope = seal_entry(&wrong_seq, &admin.key).unwrap();
    s.raw("a skipped sequence number", &envelope);

    let mut wrong_prev = entry.clone();
    wrong_prev.prev = Some(Hash([7; 32]));
    let envelope = seal_entry(&wrong_prev, &admin.key).unwrap();
    s.raw("a wrong predecessor", &envelope);

    let mut no_prev = entry.clone();
    no_prev.prev = None;
    let envelope = seal_entry(&no_prev, &admin.key).unwrap();
    s.raw("a missing predecessor", &envelope);

    let mut backwards = entry.clone();
    backwards.time = 5;
    let envelope = seal_entry(&backwards, &admin.key).unwrap();
    s.raw("time going backwards", &envelope);

    let mut other_org = entry.clone();
    other_org.org = Id([0x0e; 16]);
    let envelope = seal_entry(&other_org, &admin.key).unwrap();
    s.raw("another organization", &envelope);

    let mut stolen = seal_entry(&entry, &admin.key).unwrap();
    stolen.signatures[0].signature.0[3] ^= 1;
    s.raw("a bad signature", &stolen);

    let mut swapped = seal_entry(&entry, &admin.key).unwrap();
    swapped.signatures.insert(
        0,
        Signature {
            device: member.id(),
            signature: Fixed(sign(Context::LogEntry, &swapped.payload.0, &member.key)),
        },
    );
    s.raw("the author signs second", &swapped);

    let mut twice = seal_entry(&entry, &admin.key).unwrap();
    twice.add_signature(Context::LogEntry, admin.id(), &admin.key);
    s.raw("a device signing twice", &twice);

    let stranger = Dev::new(40, user(1), DeviceKind::Age);
    let mut unknown = seal_entry(&entry, &admin.key).unwrap();
    unknown.add_signature(Context::LogEntry, stranger.id(), &stranger.key);
    s.raw("an unknown cosigner", &unknown);

    let mut wrong_context = seal_entry(&entry, &admin.key).unwrap();
    wrong_context.signatures[0].signature =
        Fixed(sign(Context::Index, &wrong_context.payload.0, &admin.key));
    s.raw("a signature for another context", &wrong_context);

    let unsigned = Envelope {
        payload: seal_entry(&entry, &admin.key).unwrap().payload,
        signatures: Vec::new(),
    };
    s.raw("no signature", &unsigned);

    for (note, payload) in [
        ("payload that is not JSON", b"not json".to_vec()),
        ("an unknown kind", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["kind"] = json!("store.paint");
            serde_json::to_vec(&value).unwrap()
        }),
        ("a missing field", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["body"].as_object_mut().unwrap().remove("store");
            serde_json::to_vec(&value).unwrap()
        }),
        ("an identifier in upper case", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["body"]["store"] = json!(Id([0xab; 16]).to_string().to_uppercase());
            serde_json::to_vec(&value).unwrap()
        }),
        ("a sequence number that is a string", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["seq"] = json!(entry.seq.to_string());
            serde_json::to_vec(&value).unwrap()
        }),
        ("a negative time", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["time"] = json!(-1);
            serde_json::to_vec(&value).unwrap()
        }),
        ("unknown fields are ignored", {
            let mut value = serde_json::to_value(&entry).unwrap();
            value["extra"] = json!(true);
            value["body"]["extra"] = json!([1, 2]);
            serde_json::to_vec(&value).unwrap()
        }),
    ] {
        let mut envelope = Envelope {
            payload: Bytes(payload),
            signatures: Vec::new(),
        };
        envelope.add_signature(Context::LogEntry, admin.id(), &admin.key);
        s.record(note, &envelope, false);
    }

    let second_genesis = Body::Genesis {
        name: "Again".into(),
        founder: admin.user(),
        device: admin.record.clone(),
        settings: Settings::default(),
    };
    s.refused("a second genesis", &admin, second_genesis);
    s.ok(
        "the next valid entry",
        &member,
        Body::StoreCreate {
            store: Id([0x53; 16]),
        },
    );
    s.finish()
}

fn two_admin_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("two_admin", &admin, Settings::default());
    let second = s.member(&admin, 2, RoleChange::Admin);
    let member = s.member(&admin, 3, RoleChange::Member);
    let two_admin = Settings {
        device_approval: Approval::Automatic,
        two_admin: true,
        auto_join: true,
        offline_reads: false,
    };
    s.refused(
        "enabling two-admin alone",
        &admin,
        Body::OrgSettings {
            settings: two_admin,
        },
    );
    assert!(s.cosigned(
        "enabling two-admin with a cosigner",
        &admin,
        &[&second],
        Body::OrgSettings {
            settings: two_admin,
        },
    ));
    s.refused(
        "promoting an admin alone",
        &admin,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Admin,
        },
    );
    assert!(!s.cosigned(
        "a cosigning member does not count",
        &admin,
        &[&member],
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Admin,
        },
    ));
    let second_device = Dev::new(33, admin.user(), DeviceKind::Age);
    s.ok(
        "a second device of the founder",
        &admin,
        Body::DeviceAdd {
            device: second_device.record.clone(),
        },
    );
    assert!(!s.cosigned(
        "two devices of one admin are one admin",
        &admin,
        &[&second_device],
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Admin,
        },
    ));
    s.ok(
        "promoting to a non-admin role alone",
        &admin,
        Body::MemberRole {
            user: member.user(),
            role: RoleChange::Approver,
        },
    );
    s.refused(
        "setting recovery alone",
        &admin,
        Body::RecoverySet {
            recipient: Some(RECOVERY.into()),
        },
    );
    assert!(s.cosigned(
        "setting recovery with a cosigner",
        &admin,
        &[&second],
        Body::RecoverySet {
            recipient: Some(RECOVERY.into()),
        },
    ));
    let store = Id([0x51; 16]);
    s.ok("a store", &member, Body::StoreCreate { store });
    s.refused(
        "deleting a store alone",
        &member,
        Body::StoreDelete { store },
    );
    assert!(s.cosigned(
        "deleting a store with an org admin",
        &member,
        &[&second],
        Body::StoreDelete { store },
    ));
    assert!(s.cosigned(
        "disabling two-admin needs two admins too",
        &admin,
        &[&second],
        Body::OrgSettings {
            settings: Settings::default(),
        },
    ));
    s.finish()
}

fn recovery_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("recovery", &admin, Settings::default());
    let member = s.member(&admin, 2, RoleChange::Member);
    s.refused(
        "an invalid recipient",
        &admin,
        Body::RecoverySet {
            recipient: Some("nonsense".into()),
        },
    );
    s.refused(
        "members cannot set recovery",
        &member,
        Body::RecoverySet {
            recipient: Some(RECOVERY.into()),
        },
    );
    s.ok(
        "set recovery",
        &admin,
        Body::RecoverySet {
            recipient: Some(RECOVERY.into()),
        },
    );
    let store = Id([0x51; 16]);
    s.ok("a store", &member, Body::StoreCreate { store });
    s.ok(
        "an epoch with the recovery recipient",
        &member,
        epoch_body(
            store,
            0,
            vec![member.id(), recovery_id(RECOVERY)],
            vec![admin.id()],
        ),
    );
    s.ok(
        "clear recovery",
        &admin,
        Body::RecoverySet { recipient: None },
    );
    s.refused(
        "an epoch with a cleared recovery recipient",
        &member,
        epoch_body(store, 1, vec![member.id(), recovery_id(RECOVERY)], vec![]),
    );
    s.finish()
}

fn teams_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("teams", &admin, Settings::default());
    let member = s.member(&admin, 2, RoleChange::Member);
    let team = Id([0x71; 16]);
    s.refused(
        "members cannot create teams",
        &member,
        Body::TeamCreate {
            team,
            name: "Ops".into(),
        },
    );
    s.refused(
        "a blank name",
        &admin,
        Body::TeamCreate {
            team,
            name: "  ".into(),
        },
    );
    s.refused(
        "a name with a newline",
        &admin,
        Body::TeamCreate {
            team,
            name: "a\nb".into(),
        },
    );
    s.ok(
        "create a team",
        &admin,
        Body::TeamCreate {
            team,
            name: "Ops".into(),
        },
    );
    s.refused(
        "a team identifier in use",
        &admin,
        Body::TeamCreate {
            team,
            name: "Dev".into(),
        },
    );
    s.refused(
        "a store identifier taken by a team",
        &member,
        Body::StoreCreate { store: team },
    );
    s.refused(
        "adding a non-member",
        &admin,
        Body::TeamMember {
            team,
            user: user(9),
            member: true,
        },
    );
    s.refused(
        "an unknown team",
        &admin,
        Body::TeamMember {
            team: Id([0x72; 16]),
            user: member.user(),
            member: true,
        },
    );
    s.refused(
        "removing someone absent",
        &admin,
        Body::TeamMember {
            team,
            user: member.user(),
            member: false,
        },
    );
    s.ok(
        "add a member",
        &admin,
        Body::TeamMember {
            team,
            user: member.user(),
            member: true,
        },
    );
    let store = Id([0x51; 16]);
    s.ok("a store", &admin, Body::StoreCreate { store });
    s.ok(
        "grant the team",
        &admin,
        Body::StoreGrant {
            store,
            principal: Principal::Team(team),
            role: StoreRoleChange::Writer,
            expires: None,
        },
    );
    s.ok("delete the team", &admin, Body::TeamDelete { team });
    s.refused("delete it again", &admin, Body::TeamDelete { team });
    s.refused(
        "a deleted team's identifier",
        &admin,
        Body::TeamCreate {
            team,
            name: "Ops".into(),
        },
    );
    s.finish()
}

fn stores_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("stores", &admin, Settings::default());
    let owner = s.member(&admin, 2, RoleChange::Member);
    let reader = s.member(&admin, 3, RoleChange::Member);
    let writer = s.member(&admin, 4, RoleChange::Member);
    let store = Id([0x51; 16]);
    s.ok(
        "the owner creates a store",
        &owner,
        Body::StoreCreate { store },
    );
    s.ok(
        "grant a reader",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(reader.user()),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.refused(
        "readers cannot grant",
        &reader,
        Body::StoreGrant {
            store,
            principal: Principal::User(writer.user()),
            role: StoreRoleChange::Writer,
            expires: None,
        },
    );
    s.refused(
        "org admins cannot grant into a store with an admin",
        &admin,
        Body::StoreGrant {
            store,
            principal: Principal::User(admin.user()),
            role: StoreRoleChange::Admin,
            expires: None,
        },
    );
    s.refused(
        "grants to non-members",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(user(9)),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.refused(
        "grants to unknown teams",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::Team(Id([0x79; 16])),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    let past = s.time;
    s.refused(
        "an expiry in the past",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(writer.user()),
            role: StoreRoleChange::Writer,
            expires: Some(past),
        },
    );
    let expires = s.time + 25;
    s.ok(
        "an expiring writer grant",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(writer.user()),
            role: StoreRoleChange::Writer,
            expires: Some(expires),
        },
    );
    s.refused(
        "revoking an absent grant",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(admin.user()),
            role: StoreRoleChange::None,
            expires: None,
        },
    );
    s.refused(
        "removing the last store admin",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(owner.user()),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.ok(
        "time passes",
        &admin,
        Body::TeamCreate {
            team: Id([0x71; 16]),
            name: "Clock".into(),
        },
    );
    s.ok(
        "the owner leaves",
        &admin,
        Body::MemberRole {
            user: owner.user(),
            role: RoleChange::None,
        },
    );
    s.ok(
        "an org admin rescues the orphaned store",
        &admin,
        Body::StoreGrant {
            store,
            principal: Principal::User(admin.user()),
            role: StoreRoleChange::Admin,
            expires: None,
        },
    );
    s.ok(
        "revoke the reader",
        &admin,
        Body::StoreGrant {
            store,
            principal: Principal::User(reader.user()),
            role: StoreRoleChange::None,
            expires: None,
        },
    );
    s.ok("delete the store", &admin, Body::StoreDelete { store });
    s.refused(
        "a deleted store accepts no grants",
        &admin,
        Body::StoreGrant {
            store,
            principal: Principal::User(reader.user()),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.refused(
        "a deleted store cannot be deleted again",
        &admin,
        Body::StoreDelete { store },
    );
    s.refused(
        "a deleted store's identifier",
        &admin,
        Body::StoreCreate { store },
    );
    s.finish()
}

fn epochs_scenario() -> Value {
    let admin = founder();
    let mut s = Scenario::new("epochs", &admin, Settings::default());
    let owner = s.member(&admin, 2, RoleChange::Member);
    let reader = s.member(&admin, 3, RoleChange::Member);
    let auditor = s.member(&admin, 4, RoleChange::Auditor);
    let outsider = s.member(&admin, 5, RoleChange::Member);
    let browser = Dev::new(21, owner.user(), DeviceKind::Hpke);
    s.ok(
        "the owner's browser",
        &owner,
        Body::DeviceAdd {
            device: browser.record.clone(),
        },
    );
    let store = Id([0x51; 16]);
    s.ok("a store", &owner, Body::StoreCreate { store });
    s.ok(
        "a reader",
        &owner,
        Body::StoreGrant {
            store,
            principal: Principal::User(reader.user()),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );
    s.refused(
        "an epoch that skips a number",
        &owner,
        epoch_body(store, 1, vec![owner.id()], vec![]),
    );
    s.refused(
        "an outsider in the full set",
        &owner,
        epoch_body(store, 0, vec![owner.id(), outsider.id()], vec![]),
    );
    s.refused(
        "a browser in the full set",
        &owner,
        epoch_body(store, 0, vec![owner.id(), browser.id()], vec![]),
    );
    s.refused(
        "an auditor in the full set",
        &owner,
        epoch_body(store, 0, vec![auditor.id()], vec![]),
    );
    s.refused(
        "a device in both sets",
        &owner,
        epoch_body(store, 0, vec![owner.id()], vec![owner.id()]),
    );
    s.refused(
        "a reader cannot rotate",
        &reader,
        epoch_body(store, 0, vec![owner.id()], vec![]),
    );
    s.ok(
        "epoch 0",
        &owner,
        epoch_body(
            store,
            0,
            vec![owner.id(), reader.id()],
            vec![browser.id(), auditor.id(), admin.id()],
        ),
    );
    s.ok(
        "epoch 1 with fewer recipients",
        &owner,
        epoch_body(store, 1, vec![owner.id()], vec![browser.id()]),
    );
    s.ok("delete the store", &owner, Body::StoreDelete { store });
    s.refused(
        "a deleted store takes no epochs",
        &owner,
        epoch_body(store, 2, vec![owner.id()], vec![]),
    );
    s.finish()
}

fn geneses() -> Value {
    let admin = founder();
    let org = Id([0x0f; 16]);
    let good = |name: &str, settings: Settings| {
        genesis(org, name, admin.record.clone(), settings, 1_000, &admin.key).unwrap()
    };
    let mut cases = vec![
        ("a valid genesis", good("Example", Settings::default())),
        (
            "a genesis with settings",
            good(
                "Example",
                Settings {
                    device_approval: Approval::Automatic,
                    two_admin: true,
                    auto_join: true,
                    offline_reads: false,
                },
            ),
        ),
        ("an empty name", good(" ", Settings::default())),
        (
            "a name of 201 bytes",
            good(&"x".repeat(201), Settings::default()),
        ),
    ];
    let other = Dev::new(2, user(2), DeviceKind::Age);
    let mut forged = good("Example", Settings::default());
    forged.signatures[0].signature = Fixed(sign(Context::LogEntry, &forged.payload.0, &other.key));
    cases.push(("signed by another key", forged));
    let mut cosigned = good("Example", Settings::default());
    cosigned.add_signature(Context::LogEntry, other.id(), &other.key);
    cases.push(("a second signature", cosigned));
    let mut stranger = admin.record.clone();
    stranger.user = user(2);
    cases.push((
        "a device of someone else",
        seal_entry(
            &pasejo_protocol::log::Entry {
                org,
                seq: 0,
                prev: None,
                time: 1_000,
                author: admin.id(),
                body: Body::Genesis {
                    name: "Example".into(),
                    founder: user(1),
                    device: stranger,
                    settings: Settings::default(),
                },
            },
            &admin.key,
        )
        .unwrap(),
    ));
    cases.push((
        "a first entry that is no genesis",
        seal_entry(
            &pasejo_protocol::log::Entry {
                org,
                seq: 0,
                prev: None,
                time: 1_000,
                author: admin.id(),
                body: Body::StoreCreate {
                    store: Id([0x51; 16]),
                },
            },
            &admin.key,
        )
        .unwrap(),
    ));
    cases.push((
        "a genesis with a sequence number",
        seal_entry(
            &pasejo_protocol::log::Entry {
                org,
                seq: 1,
                prev: None,
                time: 1_000,
                author: admin.id(),
                body: Body::Genesis {
                    name: "Example".into(),
                    founder: admin.user(),
                    device: admin.record.clone(),
                    settings: Settings::default(),
                },
            },
            &admin.key,
        )
        .unwrap(),
    ));
    let mut bad_device = admin.record.clone();
    bad_device.label = String::new();
    cases.push((
        "a genesis device without a label",
        genesis(
            org,
            "Example",
            bad_device,
            Settings::default(),
            1_000,
            &admin.key,
        )
        .unwrap(),
    ));
    Value::Array(
        cases
            .into_iter()
            .map(|(name, envelope)| {
                let result = match OrgLog::genesis(envelope.clone()) {
                    Ok(_) => "ok".to_owned(),
                    Err(rejection) => rejection_code(&rejection),
                };
                json!({"name": name, "genesis": envelope, "result": result})
            })
            .collect(),
    )
}

/// A store with two epochs, a chain link, a browser wrapping per epoch and
/// index versions by authorized and unauthorized authors.
fn store() -> Value {
    let admin = founder();
    let mut s = Scenario::new("store", &admin, Settings::default());
    let writer = s.member(&admin, 2, RoleChange::Member);
    let reader = s.member(&admin, 3, RoleChange::Member);
    let (hpke_private, hpke_public) = hpke_keypair();
    let browser_key = SigningKey::from_bytes(&[21; 32]);
    let browser = DeviceRecord {
        id: Id([21; 16]),
        user: admin.user(),
        kind: DeviceKind::Hpke,
        recipient: b64(&hpke_public),
        signing_key: Fixed(browser_key.verifying_key().to_bytes()),
        label: "Firefox".into(),
    };
    s.ok(
        "the admin's browser",
        &admin,
        Body::DeviceAdd {
            device: browser.clone(),
        },
    );
    let store = Id([0x51; 16]);
    s.ok("a store", &admin, Body::StoreCreate { store });
    for (who, role) in [
        (&writer, StoreRoleChange::Writer),
        (&reader, StoreRoleChange::Reader),
    ] {
        s.ok(
            "a grant",
            &admin,
            Body::StoreGrant {
                store,
                principal: Principal::User(who.user()),
                role,
                expires: None,
            },
        );
    }
    let keys0 = EpochKeys::generate();
    let keys1 = EpochKeys::generate();
    for (epoch, keys) in [(0, &keys0), (1, &keys1)] {
        s.ok(
            "an epoch",
            &admin,
            Body::StoreEpoch {
                store,
                epoch,
                names_commit: names_commitment(store, epoch, &keys.names),
                data_commit: data_commitment(store, epoch, keys.data().unwrap()),
                full: vec![admin.id(), writer.id(), reader.id()],
                names_only: vec![browser.id],
            },
        );
    }
    let chain = seal_chain(store, 1, &keys1, &keys0).unwrap();
    let wrappings: Vec<Value> = [(0, &keys0), (1, &keys1)]
        .into_iter()
        .map(|(epoch, keys)| {
            json!({
                "epoch": epoch,
                "device": browser.id,
                "full": false,
                "ciphertext": b64(&wrap_hpke(store, epoch, browser.id, &hpke_public, &keys.names_only()).unwrap()),
            })
        })
        .collect();
    let head = s.log.head();
    let time = s.time + 5;
    let plain = IndexPlain {
        label: "Production — prod".into(),
        settings: StoreSettings::default(),
        secrets: BTreeMap::from([
            (
                "db/password".to_owned(),
                IndexEntry {
                    id: Id([0x61; 16]),
                    version: 1,
                    hash: Hash([1; 32]),
                },
            ),
            (
                "mail".to_owned(),
                IndexEntry {
                    id: Id([0x62; 16]),
                    version: 3,
                    hash: Hash([2; 32]),
                },
            ),
        ]),
        otp: BTreeMap::from([(
            "github".to_owned(),
            IndexEntry {
                id: Id([0x63; 16]),
                version: 1,
                hash: Hash([3; 32]),
            },
        )]),
    };
    let index = |author: &Dev, version: u64, epoch: u64, prev: Option<Hash>, keys: &EpochKeys| {
        let payload = IndexPayload {
            store,
            version,
            prev,
            epoch,
            log_head: head,
            time,
            index: Bytes(seal_index(store, epoch, version, &keys.names, &plain).unwrap()),
            lookup_hash: None,
            changed: vec![Id([0x61; 16])],
            deleted: vec![],
        };
        sign_index(&payload, author.id(), &author.key).unwrap()
    };
    let v1 = index(&writer, 1, 0, None, &keys0);
    let v2 = index(&admin, 2, 1, Some(v1.hash()), &keys1);
    let by_reader = index(&reader, 2, 1, Some(v1.hash()), &keys1);
    let unknown_epoch = index(&writer, 2, 2, Some(v1.hash()), &keys1);
    let forked = index(&writer, 2, 1, Some(Hash([9; 32])), &keys1);
    let unknown_head = {
        let mut payload: IndexPayload = v2.open().unwrap();
        payload.log_head = Hash([8; 32]);
        sign_index(&payload, admin.id(), &admin.key).unwrap()
    };
    let unsigned = {
        let mut envelope = v2.clone();
        envelope.signatures[0].signature.0[0] ^= 1;
        envelope
    };
    let pin = Pin {
        version: 1,
        hash: v1.hash(),
    };
    let checks: Vec<Value> = [
        ("version 1 by a writer", &v1, None),
        ("version 2 by the store admin", &v2, Some(pin)),
        ("version 2 by a reader", &by_reader, Some(pin)),
        ("an unknown epoch", &unknown_epoch, Some(pin)),
        ("a fork of the pinned version", &forked, Some(pin)),
        ("an unknown log head", &unknown_head, None),
        ("a bad signature", &unsigned, None),
        (
            "version 1 against a pin of version 2",
            &v1,
            Some(Pin {
                version: 2,
                hash: v2.hash(),
            }),
        ),
    ]
    .into_iter()
    .map(|(name, envelope, pin)| {
        let result = s
            .log
            .check_index(envelope)
            .and_then(|payload| check_chain(envelope, &payload, store, pin).map(|()| payload));
        json!({
            "name": name,
            "index": envelope,
            "pin": pin,
            "result": match result {
                Ok(_) => "ok".to_owned(),
                Err(error) => error_code(&error),
            },
        })
    })
    .collect();
    json!({
        "log": s.finish(),
        "store": store,
        "browser": {
            "id": browser.id,
            "signing_seed": b64(&[21; 32]),
            "hpke_private": b64(&hpke_private),
            "hpke_public": b64(&hpke_public),
        },
        "names_keys": [b64(&keys0.names.0), b64(&keys1.names.0)],
        "keys": {
            "wrappings": wrappings,
            "chain": [ChainLink {
                epoch: 1,
                prev_names: Bytes(chain.prev_names),
                prev_data: Bytes(chain.prev_data),
            }],
        },
        "index_plain": plain,
        "checks": checks,
    })
}

fn audit_event(seq: u64, prev: Hash, detail: Option<&str>, org: Option<Id>) -> AuditEvent {
    let mut event = AuditEvent {
        seq,
        time: 1_700_000_000_000 + seq,
        org,
        actor_user: seq.is_multiple_of(2).then_some(user(1)),
        actor_device: seq.is_multiple_of(3).then_some(Id([1; 16])),
        session_kind: Some("person".into()),
        ip: Some("2001:db8::1".into()),
        user_agent: (seq % 2 == 1).then(|| "pasejo/1.0 (Linux)".into()),
        action: "object.read".into(),
        store: Some(Id([0x51; 16])),
        object: Some(Id([0x61; 16])),
        version: Some(seq),
        outcome: "ok".into(),
        detail: detail.map(str::to_owned),
        prev_hash: prev,
        hash: Hash::default(),
    };
    event.hash = event.compute_hash();
    event
}

fn audit() -> Value {
    let server = SigningKey::from_bytes(&[0x5e; 32]);
    let founder = founder();
    let org = Some(Id([0x0f; 16]));
    let start = Hash([0; 32]);
    let details = [
        None,
        Some("tenant team-a/prod"),
        Some("quote \" backslash \\ slash / tab \t newline \n"),
        Some("controls \u{1} \u{1f} \u{7f} \u{85}"),
        Some("unicode ü ß 東京 🙂 \u{2028}"),
        Some(""),
    ];
    let mut events = Vec::new();
    let mut prev = start;
    for (offset, detail) in details.iter().enumerate() {
        let event = audit_event(offset as u64 + 1, prev, *detail, org);
        prev = event.hash;
        events.push(event);
    }
    let payload = CheckpointPayload {
        org,
        seq: events.len() as u64,
        hash: prev,
        log_head: Some(Hash([4; 32])),
        time: 1_700_000_000_100,
    };
    let payload_bytes = serde_json::to_vec(&payload).unwrap();
    let checkpoint = Checkpoint {
        payload: Bytes(payload_bytes.clone()),
        server_signature: Fixed(sign(Context::Checkpoint, &payload_bytes, &server)),
        cosignatures: vec![Signature {
            device: founder.id(),
            signature: Fixed(sign(Context::Checkpoint, &payload_bytes, &founder.key)),
        }],
    };
    json!({
        "start": start,
        "first_seq": 1,
        "events": events,
        "server_key": b64(server.verifying_key().as_bytes()),
        "checkpoint": checkpoint,
        "checkpoint_payload": payload,
        "cosigner": {"device": founder.id(), "key": founder.record.signing_key},
    })
}

fn request() -> Value {
    let key = SigningKey::from_bytes(&[21; 32]);
    let body = br#"{"objects":[]}"#;
    let token = "token-value";
    let nonce = b64(&[7; 16]);
    let message = message(
        "post",
        "/v1/orgs/0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f/log?since=3",
        1_700_000_000_000,
        &nonce,
        body,
        token,
    );
    json!({
        "seed": b64(&[21; 32]),
        "method": "post",
        "path": "/v1/orgs/0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f/log?since=3",
        "timestamp": 1_700_000_000_000_u64,
        "nonce": nonce,
        "body": b64(body),
        "token": token,
        "message": b64(&message),
        "signature": b64(&sign(Context::Request, &message, &key)),
    })
}

fn claim() -> Value {
    let key = SigningKey::from_bytes(&[21; 32]);
    let claim = TokenClaim {
        purpose: "web".into(),
        value: String::new(),
        signing_key: Fixed(key.verifying_key().to_bytes()),
        time: 1_700_000_000_000,
    };
    json!({
        "seed": b64(&[21; 32]),
        "device": Id([21; 16]),
        "claim": claim,
        "envelope": Envelope::seal(Context::Token, &claim, Id([21; 16]), &key).unwrap(),
    })
}

fn padding() -> Value {
    Value::Array(
        [
            0_u64,
            1,
            2,
            3,
            4,
            5,
            7,
            8,
            9,
            15,
            16,
            17,
            31,
            32,
            33,
            63,
            64,
            65,
            100,
            255,
            256,
            257,
            1000,
            1023,
            1024,
            1025,
            4096,
            65_535,
            65_536,
            65_537,
            1_000_000,
            4_294_967_295,
        ]
        .into_iter()
        .map(|length| json!([length, padme(length)]))
        .collect(),
    )
}

fn derive_vectors() -> Value {
    let ikm = Key([0x33; 32]);
    let store = Id([0x51; 16]);
    let names = Key([0x44; 32]);
    json!({
        "ikm": b64(&ikm.0),
        "store": store,
        "derived": [
            {"info": b64(b"pasejo index v1"), "okm": b64(&derive(&ikm, store, b"pasejo index v1").0)},
            {"info": b64(b"pasejo previous names v1"), "okm": b64(&derive(&ikm, store, b"pasejo previous names v1").0)},
            {"info": b64(&[]), "okm": b64(&derive(&ikm, store, &[]).0)},
        ],
        "names_key": b64(&names.0),
        "names_commitment": names_commitment(store, 7, &names),
        "data_commitment": data_commitment(store, 7, &names),
    })
}

/// A random generator that counts up from a start byte, so the HPKE
/// ephemeral key is reproducible.
struct Counting(u8);

impl RngCore for Counting {
    fn next_u32(&mut self) -> u32 {
        rand_core::impls::next_u32_via_fill(self)
    }

    fn next_u64(&mut self) -> u64 {
        rand_core::impls::next_u64_via_fill(self)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for byte in dest {
            *byte = self.0;
            self.0 = self.0.wrapping_add(1);
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for Counting {}

fn hpke_wrapped(start: u8, public: &[u8; 32], names: &Key) -> Vec<u8> {
    let keys = EpochKeys {
        names: names.clone(),
        data: None,
    };
    wrap_hpke_with_rng(
        Id([0x51; 16]),
        3,
        Id([21; 16]),
        public,
        &keys,
        &mut Counting(start),
    )
    .unwrap()
}

fn hpke_vectors() -> Value {
    let (private, public) = hpke_keypair();
    let names = Key([0x44; 32]);
    let ikm: Vec<u8> = (0..32).map(|offset| 0x42_u8.wrapping_add(offset)).collect();
    json!({
        "private": b64(&private),
        "public": b64(&public),
        "store": Id([0x51; 16]),
        "epoch": 3,
        "device": Id([21; 16]),
        "names_key": b64(&names.0),
        "ikm": b64(&ikm),
        "wrapped": b64(&hpke_wrapped(0x42, &public, &names)),
    })
}

fn fingerprints() -> Value {
    let admin = founder();
    let hash = Hash([0xab; 32]);
    json!({
        "device": admin.record,
        "fingerprint": admin.record.fingerprint(),
        "genesis": hash,
        "org_fingerprint": org_fingerprint(&hash),
        "recovery": RECOVERY,
        "recovery_id": recovery_id(RECOVERY),
    })
}

fn generate() -> Value {
    json!({
        "scenarios": [
            roles_scenario(),
            devices_scenario(),
            signatures_scenario(),
            two_admin_scenario(),
            recovery_scenario(),
            teams_scenario(),
            stores_scenario(),
            epochs_scenario(),
        ],
        "geneses": geneses(),
        "store": store(),
        "audit": audit(),
        "request": request(),
        "claim": claim(),
        "padding": padding(),
        "derive": derive_vectors(),
        "hpke": hpke_vectors(),
        "fingerprints": fingerprints(),
    })
}

// ---------------------------------------------------------------------------
// Verification

fn replay(scenario: &Value) -> OrgLog {
    let mut log = OrgLog::genesis(envelope(&scenario["genesis"])).unwrap();
    for step in scenario["steps"].as_array().unwrap() {
        let result = match log.append(envelope(&step["envelope"])) {
            Ok(_) => "ok".to_owned(),
            Err(rejection) => rejection_code(&rejection),
        };
        assert_eq!(
            result.as_str(),
            step["result"].as_str().unwrap(),
            "{}: {}",
            scenario["name"],
            step["note"]
        );
    }
    log
}

fn verify_scenarios(scenarios: &Value) {
    let scenarios = scenarios.as_array().unwrap();
    assert_eq!(scenarios.len(), 8);
    for scenario in scenarios {
        let log = replay(scenario);
        assert_eq!(
            summary(log.state()),
            scenario["state"],
            "{}",
            scenario["name"]
        );
        let rejected = scenario["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|step| step["result"] != "ok")
            .count();
        assert!(rejected > 0, "{} rejects something", scenario["name"]);
    }
}

fn verify_geneses(cases: &Value) {
    for case in cases.as_array().unwrap() {
        let result = match OrgLog::genesis(envelope(&case["genesis"])) {
            Ok(_) => "ok".to_owned(),
            Err(rejection) => rejection_code(&rejection),
        };
        assert_eq!(
            result.as_str(),
            case["result"].as_str().unwrap(),
            "{}",
            case["name"]
        );
    }
}

fn verify_store(vectors: &Value) {
    let log = replay(&vectors["log"]);
    let store = id(&vectors["store"]);
    let browser = id(&vectors["browser"]["id"]);
    let private = key32(&vectors["browser"]["hpke_private"]);
    let mut names = Vec::new();
    for wrapping in vectors["keys"]["wrappings"].as_array().unwrap() {
        let epoch = wrapping["epoch"].as_u64().unwrap();
        let keys = unwrap_hpke(
            store,
            epoch,
            browser,
            &private,
            &bytes(&wrapping["ciphertext"]),
        )
        .unwrap();
        let record = log.epoch(store, epoch).unwrap();
        assert_eq!(
            names_commitment(store, epoch, &keys.names),
            record.names_commit
        );
        assert!(keys.data.is_none());
        names.push(keys.names);
    }
    for (index, key) in names.iter().enumerate() {
        assert_eq!(b64(&key.0), vectors["names_keys"][index].as_str().unwrap());
    }
    let link = &vectors["keys"]["chain"][0];
    let previous = open_chain(
        store,
        1,
        &EpochKeys {
            names: names[1].clone(),
            data: None,
        },
        &pasejo_protocol::crypto::Chain {
            prev_names: bytes(&link["prev_names"]),
            prev_data: bytes(&link["prev_data"]),
        },
    )
    .unwrap();
    assert_eq!(previous.names, names[0]);
    let plain: IndexPlain = serde_json::from_value(vectors["index_plain"].clone()).unwrap();
    for check in vectors["checks"].as_array().unwrap() {
        let index = envelope(&check["index"]);
        let pin: Option<Pin> = serde_json::from_value(check["pin"].clone()).unwrap();
        let result = log
            .check_index(&index)
            .and_then(|payload| check_chain(&index, &payload, store, pin).map(|()| payload));
        match (&result, check["result"].as_str().unwrap()) {
            (Ok(payload), "ok") => {
                let key = &names[usize::try_from(payload.epoch).unwrap()];
                let opened =
                    open_index(store, payload.epoch, payload.version, key, &payload.index.0)
                        .unwrap();
                assert_eq!(opened, plain);
            }
            (Err(error), expected) => assert_eq!(error_code(error), expected, "{}", check["name"]),
            (Ok(_), expected) => panic!("{}: accepted, expected {expected}", check["name"]),
        }
    }
}

fn verify_audit(vectors: &Value) {
    let events: Vec<AuditEvent> = serde_json::from_value(vectors["events"].clone()).unwrap();
    let start: Hash = serde_json::from_value(vectors["start"].clone()).unwrap();
    let end = verify_chain(start, 1, &events).unwrap();
    let checkpoint: Checkpoint = serde_json::from_value(vectors["checkpoint"].clone()).unwrap();
    let payload = checkpoint
        .verify_server(&key32(&vectors["server_key"]))
        .unwrap();
    assert_eq!(payload.hash, end);
    assert_eq!(
        serde_json::to_value(&payload).unwrap(),
        vectors["checkpoint_payload"]
    );
    checkpoint
        .verify_cosignature(
            id(&vectors["cosigner"]["device"]),
            &key32(&vectors["cosigner"]["key"]),
        )
        .unwrap();
}

fn verify_request(vectors: &Value) {
    let built = message(
        vectors["method"].as_str().unwrap(),
        vectors["path"].as_str().unwrap(),
        vectors["timestamp"].as_u64().unwrap(),
        vectors["nonce"].as_str().unwrap(),
        &bytes(&vectors["body"]),
        vectors["token"].as_str().unwrap(),
    );
    assert_eq!(b64(&built), vectors["message"].as_str().unwrap());
    let key = SigningKey::from_bytes(&key32(&vectors["seed"]));
    assert_eq!(
        b64(&sign(Context::Request, &built, &key)),
        vectors["signature"].as_str().unwrap()
    );
}

fn verify_claim(vectors: &Value) {
    let key = SigningKey::from_bytes(&key32(&vectors["seed"]));
    let claim: TokenClaim = serde_json::from_value(vectors["claim"].clone()).unwrap();
    let sealed = Envelope::seal(Context::Token, &claim, id(&vectors["device"]), &key).unwrap();
    assert_eq!(serde_json::to_value(&sealed).unwrap(), vectors["envelope"]);
}

fn verify_padding(vectors: &Value) {
    for pair in vectors.as_array().unwrap() {
        assert_eq!(padme(pair[0].as_u64().unwrap()), pair[1].as_u64().unwrap());
    }
}

fn verify_derive(vectors: &Value) {
    let ikm = Key(key32(&vectors["ikm"]));
    let store = id(&vectors["store"]);
    for case in vectors["derived"].as_array().unwrap() {
        assert_eq!(
            b64(&derive(&ikm, store, &bytes(&case["info"])).0),
            case["okm"].as_str().unwrap()
        );
    }
    let names = Key(key32(&vectors["names_key"]));
    assert_eq!(
        serde_json::to_value(names_commitment(store, 7, &names)).unwrap(),
        vectors["names_commitment"]
    );
    assert_eq!(
        serde_json::to_value(data_commitment(store, 7, &names)).unwrap(),
        vectors["data_commitment"]
    );
}

fn verify_hpke(vectors: &Value) {
    let public = key32(&vectors["public"]);
    let names = Key(key32(&vectors["names_key"]));
    assert_eq!(
        b64(&hpke_wrapped(0x42, &public, &names)),
        vectors["wrapped"].as_str().unwrap(),
        "the counting generator reproduces the wrapping"
    );
    let keys = unwrap_hpke(
        id(&vectors["store"]),
        vectors["epoch"].as_u64().unwrap(),
        id(&vectors["device"]),
        &key32(&vectors["private"]),
        &bytes(&vectors["wrapped"]),
    )
    .unwrap();
    assert_eq!(keys.names, names);
}

fn verify_fingerprints(vectors: &Value) {
    let device: DeviceRecord = serde_json::from_value(vectors["device"].clone()).unwrap();
    assert_eq!(
        device.fingerprint(),
        vectors["fingerprint"].as_str().unwrap()
    );
    let genesis: Hash = serde_json::from_value(vectors["genesis"].clone()).unwrap();
    assert_eq!(
        org_fingerprint(&genesis),
        vectors["org_fingerprint"].as_str().unwrap()
    );
    assert_eq!(
        recovery_id(vectors["recovery"].as_str().unwrap()).to_string(),
        vectors["recovery_id"].as_str().unwrap()
    );
}
