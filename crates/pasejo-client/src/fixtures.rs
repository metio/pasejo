// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! An organization log with one store and two epochs, built locally for
//! unit tests.

use ed25519_dalek::SigningKey;
use pasejo_protocol::Id;
use pasejo_protocol::api::{ChainLink, Keys, Wrapping};
use pasejo_protocol::crypto::{EpochKeys, data_commitment, names_commitment, seal_chain};
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::Bytes;
use pasejo_protocol::log::{Body, OrgLog, Settings, genesis, next_entry, seal_entry};
use pasejo_protocol::wrapping::wrap_age;
use rand_core::OsRng;

use crate::transport::DeviceIdentity;

pub const TIME: u64 = 1_800_000_000_000;

pub struct Fixture {
    pub identity: age::x25519::Identity,
    pub device: DeviceIdentity,
    pub log: OrgLog,
    pub store: Id,
    pub epochs: Vec<EpochKeys>,
}

impl Fixture {
    /// A log whose store has `epochs` epochs, each committed in the log.
    pub fn new(epochs: usize) -> Self {
        let identity = age::x25519::Identity::generate();
        let device = DeviceIdentity {
            id: Id::random(),
            signing_key: SigningKey::generate(&mut OsRng),
            kind: DeviceKind::Age,
            recipient: identity.to_public().to_string(),
            label: "test".into(),
        };
        let envelope = genesis(
            Id::random(),
            "Acme",
            device.record(Id::random()),
            Settings::default(),
            TIME,
            &device.signing_key,
        )
        .unwrap();
        let mut fixture = Self {
            identity,
            device,
            log: OrgLog::genesis(envelope).unwrap(),
            store: Id::random(),
            epochs: Vec::new(),
        };
        fixture.append(Body::StoreCreate {
            store: fixture.store,
        });
        for _ in 0..epochs {
            fixture.add_epoch();
        }
        fixture
    }

    pub fn append(&mut self, body: Body) {
        let entry = next_entry(&self.log, self.device.id, TIME, body);
        self.log
            .append(seal_entry(&entry, &self.device.signing_key).unwrap())
            .unwrap();
    }

    pub fn add_epoch(&mut self) {
        let keys = EpochKeys::generate();
        let epoch = self.epochs.len() as u64;
        self.append(Body::StoreEpoch {
            store: self.store,
            epoch,
            names_commit: names_commitment(self.store, epoch, &keys.names),
            data_commit: data_commitment(self.store, epoch, keys.data().unwrap()),
            full: vec![self.device.id],
            names_only: Vec::new(),
        });
        self.epochs.push(keys);
    }

    pub fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }

    /// What the keys endpoint returns: a wrapping of the current epoch and
    /// the chain links back to epoch 0.
    pub fn keys(&self, full: bool) -> Keys {
        let current = self.epochs.len() as u64 - 1;
        let keys = &self.epochs[current as usize];
        let wrapped = if full {
            keys.clone()
        } else {
            keys.names_only()
        };
        Keys {
            wrappings: vec![Wrapping {
                epoch: current,
                device: self.device.id,
                full,
                ciphertext: Bytes(
                    wrap_age(self.store, current, &wrapped, &self.identity.to_public()).unwrap(),
                ),
            }],
            chain: (1..=current)
                .map(|epoch| {
                    let chain = seal_chain(
                        self.store,
                        epoch,
                        &self.epochs[epoch as usize],
                        &self.epochs[epoch as usize - 1],
                    )
                    .unwrap();
                    ChainLink {
                        epoch,
                        prev_names: Bytes(chain.prev_names),
                        prev_data: Bytes(chain.prev_data),
                    }
                })
                .collect(),
        }
    }
}
