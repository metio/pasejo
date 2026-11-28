// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! A store's epoch keys as this device holds them: unwrapped from its
//! wrapping, checked against the commitments in the membership log, and
//! extended backwards through the epoch chain.

use std::collections::BTreeMap;

use pasejo_protocol::Id;
use pasejo_protocol::api::Keys;
use pasejo_protocol::crypto::{Chain, EpochKeys, check_commitments, open_chain};
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::wrapping::unwrap_age;

use crate::error::{Error, Result};
use crate::transport::Client;

/// Every epoch key of one store that this device can reach.
#[derive(Debug, Clone)]
pub struct KeyRing {
    pub store: Id,
    pub current: u64,
    epochs: BTreeMap<u64, EpochKeys>,
}

impl KeyRing {
    /// The keys of `epoch`.
    ///
    /// # Errors
    ///
    /// Fails for an epoch outside the ring.
    pub fn epoch(&self, epoch: u64) -> Result<&EpochKeys> {
        self.epochs
            .get(&epoch)
            .ok_or_else(|| Error::Unavailable(format!("no keys for epoch {epoch}")))
    }

    /// The current epoch's keys.
    #[must_use]
    pub fn current(&self) -> &EpochKeys {
        &self.epochs[&self.current]
    }

    /// Whether this device holds the data keys.
    #[must_use]
    pub fn full(&self) -> bool {
        self.current().data.is_some()
    }

    /// A ring holding exactly `keys` for the current epoch, as the device
    /// that generated them knows it.
    #[must_use]
    pub fn generated(store: Id, epoch: u64, keys: EpochKeys, previous: Option<&Self>) -> Self {
        let mut epochs = previous.map_or_else(BTreeMap::new, |ring| ring.epochs.clone());
        epochs.insert(epoch, keys);
        Self {
            store,
            current: epoch,
            epochs,
        }
    }

    /// Builds a ring from what the keys endpoint returned, checking each
    /// key against the commitments of the signed epoch entries in `log`.
    ///
    /// # Errors
    ///
    /// Fails when no wrapping opens with `identities`, or any key differs
    /// from its commitment.
    pub fn open(
        log: &OrgLog,
        store: Id,
        device: Id,
        keys: &Keys,
        identities: &[Box<dyn age::Identity>],
    ) -> Result<Self> {
        let current = log
            .state()
            .stores
            .get(&store)
            .and_then(pasejo_protocol::log::StoreState::current_epoch)
            .ok_or_else(|| Error::Unavailable("the store has no keys yet".into()))?;
        let wrapping = keys
            .wrappings
            .iter()
            .find(|wrapping| wrapping.epoch == current && wrapping.device == device)
            .ok_or_else(|| {
                Error::Unavailable("this device holds no wrapping of the store's keys".into())
            })?;
        let opened = unwrap_age(store, current, &wrapping.ciphertext.0, identities)?;
        check(log, store, current, &opened)?;
        let mut epochs = BTreeMap::new();
        let mut keys_at = opened;
        let mut epoch = current;
        epochs.insert(epoch, keys_at.clone());
        while epoch > 0 {
            let Some(link) = keys.chain.iter().find(|link| link.epoch == epoch) else {
                break;
            };
            let previous = open_chain(
                store,
                epoch,
                &keys_at,
                &Chain {
                    prev_names: link.prev_names.0.clone(),
                    prev_data: link.prev_data.0.clone(),
                },
            )?;
            epoch -= 1;
            check(log, store, epoch, &previous)?;
            epochs.insert(epoch, previous.clone());
            keys_at = previous;
        }
        Ok(Self {
            store,
            current,
            epochs,
        })
    }
}

fn check(log: &OrgLog, store: Id, epoch: u64, keys: &EpochKeys) -> Result<()> {
    let record = log
        .epoch(store, epoch)
        .ok_or_else(|| Error::verification(format!("epoch {epoch} is not in the log")))?;
    check_commitments(
        store,
        epoch,
        keys,
        &record.names_commit,
        &record.data_commit,
    )
    .map_err(Error::from)
}

impl Client {
    /// Fetches and opens this device's keys of `store`.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, or the keys do not open or verify.
    pub fn keys(
        &self,
        log: &OrgLog,
        store: Id,
        identities: &[Box<dyn age::Identity>],
    ) -> Result<KeyRing> {
        let keys = self.wrapped_keys(log, store)?;
        KeyRing::open(log, store, self.device.id, &keys, identities)
    }

    /// Fetches this device's wrapped keys of `store` without opening them,
    /// for callers that open several stores' keys back to back with
    /// [`KeyRing::open`].
    ///
    /// # Errors
    ///
    /// Fails for devices other than age devices, and when the server
    /// refuses.
    pub fn wrapped_keys(&self, log: &OrgLog, store: Id) -> Result<Keys> {
        if self.device.kind != DeviceKind::Age {
            return Err(Error::Unavailable("only age devices open keys here".into()));
        }
        let org = log.state().org;
        self.get(&format!("/v1/orgs/{org}/stores/{store}/keys"))
    }
}

#[cfg(test)]
mod tests {
    use pasejo_protocol::api::ChainLink;
    use pasejo_protocol::crypto::seal_chain;
    use pasejo_protocol::encoding::Bytes;

    use super::*;
    use crate::fixtures::Fixture;

    #[test]
    fn a_ring_reaches_every_epoch_through_the_chain() {
        let fixture = Fixture::new(3);
        let ring = KeyRing::open(
            &fixture.log,
            fixture.store,
            fixture.device.id,
            &fixture.keys(true),
            &fixture.identities(),
        )
        .unwrap();
        assert_eq!(ring.current, 2);
        assert!(ring.full());
        for (epoch, keys) in fixture.epochs.iter().enumerate() {
            assert_eq!(ring.epoch(epoch as u64).unwrap(), keys);
        }
        assert!(ring.epoch(3).is_err());
    }

    #[test]
    fn a_names_only_wrapping_gives_a_names_only_ring() {
        let fixture = Fixture::new(2);
        let ring = KeyRing::open(
            &fixture.log,
            fixture.store,
            fixture.device.id,
            &fixture.keys(false),
            &fixture.identities(),
        )
        .unwrap();
        assert!(!ring.full());
        assert!(ring.current().data().is_err());
        // Names-only keys cannot open the chain's data half.
        assert!(ring.epoch(0).is_err() || ring.epoch(0).unwrap().data().is_err());
    }

    #[test]
    fn a_missing_chain_link_stops_the_ring_at_the_gap() {
        let fixture = Fixture::new(3);
        let mut keys = fixture.keys(true);
        keys.chain.retain(|link| link.epoch != 1);
        let ring = KeyRing::open(
            &fixture.log,
            fixture.store,
            fixture.device.id,
            &keys,
            &fixture.identities(),
        )
        .unwrap();
        assert!(ring.epoch(1).is_ok());
        assert!(ring.epoch(0).is_err());
    }

    #[test]
    fn a_chain_link_to_uncommitted_keys_is_refused() {
        let fixture = Fixture::new(2);
        let mut keys = fixture.keys(true);
        let forged =
            seal_chain(fixture.store, 1, &fixture.epochs[1], &EpochKeys::generate()).unwrap();
        keys.chain = vec![ChainLink {
            epoch: 1,
            prev_names: Bytes(forged.prev_names),
            prev_data: Bytes(forged.prev_data),
        }];
        assert!(
            KeyRing::open(
                &fixture.log,
                fixture.store,
                fixture.device.id,
                &keys,
                &fixture.identities()
            )
            .is_err()
        );
    }

    #[test]
    fn wrappings_of_other_devices_or_epochs_do_not_count() {
        let fixture = Fixture::new(2);
        let identities = fixture.identities();
        assert!(
            KeyRing::open(
                &fixture.log,
                fixture.store,
                Id::random(),
                &fixture.keys(true),
                &identities
            )
            .is_err()
        );
        let mut stale = fixture.keys(true);
        stale.wrappings[0].epoch = 0;
        assert!(
            KeyRing::open(
                &fixture.log,
                fixture.store,
                fixture.device.id,
                &stale,
                &identities
            )
            .is_err()
        );
        let stranger: Vec<Box<dyn age::Identity>> =
            vec![Box::new(age::x25519::Identity::generate())];
        assert!(
            KeyRing::open(
                &fixture.log,
                fixture.store,
                fixture.device.id,
                &fixture.keys(true),
                &stranger
            )
            .is_err()
        );
    }

    #[test]
    fn a_store_without_epochs_has_no_keys() {
        let fixture = Fixture::new(0);
        let keys = pasejo_protocol::api::Keys {
            wrappings: Vec::new(),
            chain: Vec::new(),
        };
        assert!(
            KeyRing::open(
                &fixture.log,
                fixture.store,
                fixture.device.id,
                &keys,
                &fixture.identities()
            )
            .is_err()
        );
    }

    #[test]
    fn a_generated_ring_extends_the_previous_one() {
        let fixture = Fixture::new(1);
        let first = KeyRing::generated(fixture.store, 0, fixture.epochs[0].clone(), None);
        let next = EpochKeys::generate();
        let second = KeyRing::generated(fixture.store, 1, next.clone(), Some(&first));
        assert_eq!(second.current, 1);
        assert_eq!(second.current(), &next);
        assert_eq!(second.epoch(0).unwrap(), &fixture.epochs[0]);
    }
}
