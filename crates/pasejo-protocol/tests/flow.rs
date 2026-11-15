// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The whole client-side flow over the protocol primitives: an organization
//! creates a store, wraps its keys, writes secrets, rotates the epoch and a
//! second member reads everything back.

use ed25519_dalek::SigningKey;
use pasejo_protocol::crypto::{
    EpochKeys, check_commitments, data_commitment, names_commitment, open_chain, seal_chain,
};
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::{Fixed, Hash, Id, b64};
use pasejo_protocol::log::{
    Body, OrgLog, Principal, RoleChange, Settings, StoreRoleChange, genesis, next_entry, seal_entry,
};
use pasejo_protocol::store::{
    IndexEntry, IndexPayload, IndexPlain, LookupPlain, ObjectPlain, Pin, check_chain, open_index,
    open_lookup, open_object, seal_index, seal_lookup, seal_object, sign_index,
};
use pasejo_protocol::wrapping::{hpke_keypair, unwrap_age, unwrap_hpke, wrap_age, wrap_hpke};
use pasejo_protocol::{Bytes, Error};
use rand_core::OsRng;

struct Member {
    record: DeviceRecord,
    key: SigningKey,
    identity: age::x25519::Identity,
}

impl Member {
    fn new() -> Self {
        let identity = age::x25519::Identity::generate();
        let key = SigningKey::generate(&mut OsRng);
        Self {
            record: DeviceRecord {
                id: Id::random(),
                user: Id::random(),
                kind: DeviceKind::Age,
                recipient: identity.to_public().to_string(),
                signing_key: Fixed(key.verifying_key().to_bytes()),
                label: "laptop".into(),
            },
            key,
            identity,
        }
    }

    fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }
}

fn append(log: &mut OrgLog, author: &Member, time: u64, body: Body) {
    let entry = next_entry(log, author.record.id, time, body);
    log.append(seal_entry(&entry, &author.key).unwrap())
        .unwrap();
}

fn epoch_body(log: &OrgLog, store: Id, epoch: u64, keys: &EpochKeys, time: u64) -> Body {
    let entitled = log.state().entitled(store, time);
    Body::StoreEpoch {
        store,
        epoch,
        names_commit: names_commitment(store, epoch, &keys.names),
        data_commit: data_commitment(store, epoch, keys.data().unwrap()),
        full: entitled.full.into_iter().collect(),
        names_only: entitled.names_only.into_iter().collect(),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn a_store_round_trips_through_writes_rotation_and_a_second_reader() {
    let alice = Member::new();
    let bob = Member::new();
    let mut log = OrgLog::genesis(
        genesis(
            Id::random(),
            "Org",
            alice.record.clone(),
            Settings::default(),
            1,
            &alice.key,
        )
        .unwrap(),
    )
    .unwrap();
    append(
        &mut log,
        &alice,
        2,
        Body::MemberRole {
            user: bob.record.user,
            role: RoleChange::Member,
        },
    );
    append(
        &mut log,
        &alice,
        3,
        Body::DeviceAdd {
            device: bob.record.clone(),
        },
    );
    let store = Id::random();
    append(&mut log, &alice, 4, Body::StoreCreate { store });
    append(
        &mut log,
        &alice,
        5,
        Body::StoreGrant {
            store,
            principal: Principal::User(bob.record.user),
            role: StoreRoleChange::Reader,
            expires: None,
        },
    );

    // Epoch 0: alice wraps keys to both devices.
    let epoch0 = EpochKeys::generate();
    let body = epoch_body(&log, store, 0, &epoch0, 6);
    append(&mut log, &alice, 6, body);
    let alice_recipient: age::x25519::Recipient = alice.record.recipient.parse().unwrap();
    let bob_recipient: age::x25519::Recipient = bob.record.recipient.parse().unwrap();
    let wrapped_bob = wrap_age(store, 0, &epoch0, &bob_recipient).unwrap();
    let _wrapped_alice = wrap_age(store, 0, &epoch0, &alice_recipient).unwrap();

    // Alice writes two secrets.
    let id = Id::random();
    let object = seal_object(
        store,
        0,
        id,
        1,
        epoch0.data().unwrap(),
        &ObjectPlain::Secret {
            secret: "hunter2\nurl: https://example.com".into(),
        },
    )
    .unwrap();
    let mut index = IndexPlain {
        label: "team".into(),
        ..IndexPlain::default()
    };
    index.secrets.insert(
        "web/example".into(),
        IndexEntry {
            id,
            version: 1,
            hash: Hash::of(&object),
        },
    );
    let mut lookup = LookupPlain::default();
    lookup
        .secrets
        .insert("web/example".into(), "url: https://example.com".into());
    let sealed_index = seal_index(store, 0, 1, &epoch0.names, &index).unwrap();
    let sealed_lookup = seal_lookup(store, 0, 1, epoch0.data().unwrap(), &lookup).unwrap();
    let payload = IndexPayload {
        store,
        version: 1,
        prev: None,
        epoch: 0,
        log_head: log.head(),
        time: 7,
        index: Bytes(sealed_index),
        lookup_hash: Some(Hash::of(&sealed_lookup)),
        changed: vec![id],
        deleted: vec![],
    };
    let first = sign_index(&payload, alice.record.id, &alice.key).unwrap();

    // Bob reads: the log vouches for the index author and the key commitments.
    let checked = log.check_index(&first).unwrap();
    check_chain(&first, &checked, store, None).unwrap();
    let bob_keys = unwrap_age(store, 0, &wrapped_bob, &bob.identities()).unwrap();
    let record = log.epoch(store, 0).unwrap();
    check_commitments(
        store,
        0,
        &bob_keys,
        &record.names_commit,
        &record.data_commit,
    )
    .unwrap();
    let read_index = open_index(store, 0, 1, &bob_keys.names, &checked.index.0).unwrap();
    assert_eq!(read_index.label, "team");
    let entry = &read_index.secrets["web/example"];
    assert_eq!(Hash::of(&object), entry.hash);
    let plain = open_object(
        store,
        0,
        entry.id,
        entry.version,
        bob_keys.data().unwrap(),
        &object,
    )
    .unwrap();
    let ObjectPlain::Secret { secret } = &plain else {
        panic!("expected a secret");
    };
    assert!(secret.starts_with("hunter2"));
    assert_eq!(
        open_lookup(store, 0, 1, bob_keys.data().unwrap(), &sealed_lookup).unwrap(),
        lookup
    );

    // A server that forges a wrapping with keys of its own is caught.
    let forged = wrap_age(store, 0, &EpochKeys::generate(), &bob_recipient).unwrap();
    let forged_keys = unwrap_age(store, 0, &forged, &bob.identities()).unwrap();
    assert_eq!(
        check_commitments(
            store,
            0,
            &forged_keys,
            &record.names_commit,
            &record.data_commit
        ),
        Err(Error::Commitment)
    );

    // Bob loses access; alice rotates to epoch 1 and chains epoch 0.
    append(
        &mut log,
        &alice,
        8,
        Body::StoreGrant {
            store,
            principal: Principal::User(bob.record.user),
            role: StoreRoleChange::None,
            expires: None,
        },
    );
    let epoch1 = EpochKeys::generate();
    let body = epoch_body(&log, store, 1, &epoch1, 9);
    let Body::StoreEpoch { full, .. } = &body else {
        unreachable!()
    };
    assert!(!full.contains(&bob.record.id));
    append(&mut log, &alice, 9, body);
    let chain = seal_chain(store, 1, &epoch1, &epoch0).unwrap();
    assert_eq!(open_chain(store, 1, &epoch1, &chain).unwrap(), epoch0);

    // The next index is in epoch 1 and chains to the first one.
    let sealed_index = seal_index(store, 1, 2, &epoch1.names, &index).unwrap();
    let second_payload = IndexPayload {
        version: 2,
        prev: Some(first.hash()),
        epoch: 1,
        log_head: log.head(),
        time: 10,
        index: Bytes(sealed_index),
        lookup_hash: None,
        changed: vec![],
        ..payload
    };
    let second = sign_index(&second_payload, alice.record.id, &alice.key).unwrap();
    let checked = log.check_index(&second).unwrap();
    check_chain(
        &second,
        &checked,
        store,
        Some(Pin {
            version: 1,
            hash: first.hash(),
        }),
    )
    .unwrap();
    // Objects written in epoch 0 stay readable through the chain.
    let previous = open_chain(store, 1, &epoch1, &chain).unwrap();
    assert!(open_object(store, 0, id, 1, previous.data().unwrap(), &object).is_ok());
    // Bob, no longer entitled, cannot write an index.
    let bob_index = sign_index(&second_payload, bob.record.id, &bob.key).unwrap();
    assert!(log.check_index(&bob_index).is_err());
}

#[test]
fn browser_devices_read_names_but_never_secrets() {
    let alice = Member::new();
    let mut log = OrgLog::genesis(
        genesis(
            Id::random(),
            "Org",
            alice.record.clone(),
            Settings::default(),
            1,
            &alice.key,
        )
        .unwrap(),
    )
    .unwrap();
    let (private, public) = hpke_keypair();
    let browser_key = SigningKey::generate(&mut OsRng);
    let browser = DeviceRecord {
        id: Id::random(),
        user: alice.record.user,
        kind: DeviceKind::Hpke,
        recipient: b64(&public),
        signing_key: Fixed(browser_key.verifying_key().to_bytes()),
        label: "browser".into(),
    };
    append(
        &mut log,
        &alice,
        2,
        Body::DeviceAdd {
            device: browser.clone(),
        },
    );
    let store = Id::random();
    append(&mut log, &alice, 3, Body::StoreCreate { store });
    let keys = EpochKeys::generate();
    let body = epoch_body(&log, store, 0, &keys, 4);
    append(&mut log, &alice, 4, body);
    let record = log.epoch(store, 0).unwrap();
    assert!(record.names_only.contains(&browser.id));

    let wrapped = wrap_hpke(store, 0, browser.id, &public, &keys.names_only()).unwrap();
    let unwrapped = unwrap_hpke(store, 0, browser.id, &private, &wrapped).unwrap();
    assert!(unwrapped.data.is_none());
    check_commitments(
        store,
        0,
        &unwrapped,
        &record.names_commit,
        &record.data_commit,
    )
    .unwrap();
    let index = IndexPlain {
        label: "shared".into(),
        ..IndexPlain::default()
    };
    let sealed = seal_index(store, 0, 1, &keys.names, &index).unwrap();
    assert_eq!(
        open_index(store, 0, 1, &unwrapped.names, &sealed)
            .unwrap()
            .label,
        "shared"
    );
    assert!(unwrapped.data().is_err());
}
