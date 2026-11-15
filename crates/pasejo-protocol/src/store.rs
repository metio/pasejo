// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The objects of a remote store: secret objects, the signed index and the
//! lookup object.

use std::collections::BTreeMap;

use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::crypto::{Key, concat, derive, open_padded, seal_padded};
use crate::encoding::{Bytes, Hash, Id};
use crate::envelope::{Context, Envelope};
use crate::{Error, Millis};

/// One-time password parameters, as the CLI stores them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct OtpParams {
    pub secret: String,
    pub otp_type: String,
    pub algorithm: String,
    pub digits: u8,
    pub period: u64,
    pub counter: u64,
    pub skew: u64,
}

/// The plaintext of a secret object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ObjectPlain {
    Secret { secret: String },
    Otp { otp: OtpParams },
}

impl Drop for ObjectPlain {
    fn drop(&mut self) {
        if let Self::Secret { secret } = self {
            secret.zeroize();
        }
    }
}

fn object_info(id: Id, version: u64) -> Vec<u8> {
    concat(&[b"pasejo object v1", id.as_bytes(), &version.to_be_bytes()])
}

fn object_aad(store: Id, epoch: u64, id: Id, version: u64) -> Vec<u8> {
    concat(&[
        b"pasejo object v1",
        store.as_bytes(),
        &epoch.to_be_bytes(),
        id.as_bytes(),
        &version.to_be_bytes(),
    ])
}

/// Encrypts one version of a secret object.
///
/// # Errors
///
/// Fails only if the plaintext cannot be serialized or is too long.
pub fn seal_object(
    store: Id,
    epoch: u64,
    id: Id,
    version: u64,
    data_key: &Key,
    plain: &ObjectPlain,
) -> Result<Vec<u8>, Error> {
    let key = derive(data_key, store, &object_info(id, version));
    let json = Zeroizing::new(serde_json::to_vec(plain)?);
    seal_padded(&key, &json, &object_aad(store, epoch, id, version))
}

/// Decrypts one version of a secret object.
///
/// # Errors
///
/// Fails when the object was not sealed for exactly this store, epoch,
/// identifier and version.
pub fn open_object(
    store: Id,
    epoch: u64,
    id: Id,
    version: u64,
    data_key: &Key,
    sealed: &[u8],
) -> Result<ObjectPlain, Error> {
    let key = derive(data_key, store, &object_info(id, version));
    let json = open_padded(&key, sealed, &object_aad(store, epoch, id, version))?;
    serde_json::from_slice(&json).map_err(|_| Error::Decryption)
}

/// Where a name points in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: Id,
    pub version: u64,
    /// SHA-256 of the object's ciphertext.
    pub hash: Hash,
}

/// Settings every member's client must agree on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreSettings {
    pub uri_keys: Vec<String>,
    pub username_keys: Vec<String>,
}

impl Default for StoreSettings {
    fn default() -> Self {
        Self {
            uri_keys: ["uri", "url", "link", "site"].map(String::from).to_vec(),
            username_keys: ["username", "user", "login", "email"]
                .map(String::from)
                .to_vec(),
        }
    }
}

/// The plaintext of the index: the store's label, settings and the names of
/// its secrets and one-time passwords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IndexPlain {
    pub label: String,
    #[serde(default)]
    pub settings: StoreSettings,
    #[serde(default)]
    pub secrets: BTreeMap<String, IndexEntry>,
    #[serde(default)]
    pub otp: BTreeMap<String, IndexEntry>,
}

impl IndexPlain {
    /// Every object identifier the index references.
    pub fn ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.secrets
            .values()
            .chain(self.otp.values())
            .map(|entry| entry.id)
    }
}

/// The signed payload of an index version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexPayload {
    pub store: Id,
    pub version: u64,
    pub prev: Option<Hash>,
    pub epoch: u64,
    pub log_head: Hash,
    pub time: Millis,
    pub index: Bytes,
    pub lookup_hash: Option<Hash>,
    pub changed: Vec<Id>,
    pub deleted: Vec<Id>,
}

fn index_aad(store: Id, epoch: u64, version: u64) -> Vec<u8> {
    concat(&[
        b"pasejo index v1",
        store.as_bytes(),
        &epoch.to_be_bytes(),
        &version.to_be_bytes(),
    ])
}

/// Encrypts an index plaintext.
///
/// # Errors
///
/// Fails only if the plaintext cannot be serialized.
pub fn seal_index(
    store: Id,
    epoch: u64,
    version: u64,
    names_key: &Key,
    plain: &IndexPlain,
) -> Result<Vec<u8>, Error> {
    let key = derive(names_key, store, b"pasejo index v1");
    let json = Zeroizing::new(serde_json::to_vec(plain)?);
    seal_padded(&key, &json, &index_aad(store, epoch, version))
}

/// Decrypts an index plaintext.
///
/// # Errors
///
/// Fails when the index was not sealed for this store, epoch and version.
pub fn open_index(
    store: Id,
    epoch: u64,
    version: u64,
    names_key: &Key,
    sealed: &[u8],
) -> Result<IndexPlain, Error> {
    let key = derive(names_key, store, b"pasejo index v1");
    let json = open_padded(&key, sealed, &index_aad(store, epoch, version))?;
    serde_json::from_slice(&json).map_err(|_| Error::Decryption)
}

/// The plaintext of the lookup object: per secret name, the lines of the
/// secret whose keys are lookup fields or browser policies. The password
/// line is never part of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LookupPlain {
    pub secrets: BTreeMap<String, String>,
}

impl Drop for LookupPlain {
    fn drop(&mut self) {
        for value in self.secrets.values_mut() {
            value.zeroize();
        }
    }
}

fn lookup_aad(store: Id, epoch: u64, version: u64) -> Vec<u8> {
    concat(&[
        b"pasejo lookup v1",
        store.as_bytes(),
        &epoch.to_be_bytes(),
        &version.to_be_bytes(),
    ])
}

/// Encrypts a lookup object for index version `version`.
///
/// # Errors
///
/// Fails only if the plaintext cannot be serialized.
pub fn seal_lookup(
    store: Id,
    epoch: u64,
    version: u64,
    data_key: &Key,
    plain: &LookupPlain,
) -> Result<Vec<u8>, Error> {
    let key = derive(data_key, store, b"pasejo lookup v1");
    let json = Zeroizing::new(serde_json::to_vec(plain)?);
    seal_padded(&key, &json, &lookup_aad(store, epoch, version))
}

/// Decrypts a lookup object.
///
/// # Errors
///
/// Fails when the lookup object was not sealed for this store, epoch and
/// index version.
pub fn open_lookup(
    store: Id,
    epoch: u64,
    version: u64,
    data_key: &Key,
    sealed: &[u8],
) -> Result<LookupPlain, Error> {
    let key = derive(data_key, store, b"pasejo lookup v1");
    let json = open_padded(&key, sealed, &lookup_aad(store, epoch, version))?;
    serde_json::from_slice(&json).map_err(|_| Error::Decryption)
}

/// A version of a secret object as it travels between client and server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedObject {
    pub id: Id,
    pub version: u64,
    pub epoch: u64,
    pub ciphertext: Bytes,
}

/// Signs an index payload as `device`.
///
/// # Errors
///
/// Fails only if the payload cannot be serialized.
pub fn sign_index(payload: &IndexPayload, device: Id, key: &SigningKey) -> Result<Envelope, Error> {
    Envelope::seal(Context::Index, payload, device, key)
}

/// What a client remembers about the last index version it accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pin {
    pub version: u64,
    pub hash: Hash,
}

/// Checks an index payload's shape and its place in the chain the client
/// pinned. Signature and authorship are checked against the membership log
/// by [`crate::log::OrgLog::check_index`].
///
/// # Errors
///
/// Fails for another store, a version that goes backwards, or a version
/// that forks from the pinned one.
pub fn check_chain(
    envelope: &Envelope,
    payload: &IndexPayload,
    store: Id,
    pinned: Option<Pin>,
) -> Result<(), Error> {
    if payload.store != store {
        return Err(Error::Invalid("index of another store"));
    }
    if payload.version == 0 || (payload.version == 1) != payload.prev.is_none() {
        return Err(Error::Invalid("index version"));
    }
    if let Some(pin) = pinned {
        if payload.version < pin.version {
            return Err(Error::Invalid("index went backwards"));
        }
        if payload.version == pin.version && envelope.hash() != pin.hash {
            return Err(Error::Invalid("index forked"));
        }
        if payload.version == pin.version + 1 && payload.prev != Some(pin.hash) {
            return Err(Error::Invalid("index forked"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;

    fn otp() -> OtpParams {
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

    #[test]
    fn objects_round_trip_and_bind_store_epoch_id_and_version() {
        let (store, id, key) = (Id::random(), Id::random(), Key::random());
        let plain = ObjectPlain::Secret {
            secret: "hunter2\nuser: alice".into(),
        };
        let sealed = seal_object(store, 1, id, 4, &key, &plain).unwrap();
        assert_eq!(open_object(store, 1, id, 4, &key, &sealed).unwrap(), plain);
        assert!(open_object(store, 1, id, 5, &key, &sealed).is_err());
        assert!(open_object(store, 2, id, 4, &key, &sealed).is_err());
        assert!(open_object(store, 1, Id::random(), 4, &key, &sealed).is_err());
        assert!(open_object(Id::random(), 1, id, 4, &key, &sealed).is_err());
        assert!(open_object(store, 1, id, 4, &Key::random(), &sealed).is_err());
    }

    #[test]
    fn otp_objects_round_trip() {
        let (store, id, key) = (Id::random(), Id::random(), Key::random());
        let plain = ObjectPlain::Otp { otp: otp() };
        let sealed = seal_object(store, 0, id, 1, &key, &plain).unwrap();
        assert_eq!(open_object(store, 0, id, 1, &key, &sealed).unwrap(), plain);
    }

    #[test]
    fn object_plaintext_names_its_kind() {
        let json = serde_json::to_value(ObjectPlain::Secret { secret: "x".into() }).unwrap();
        assert_eq!(json, serde_json::json!({"kind": "secret", "secret": "x"}));
    }

    #[test]
    fn indexes_round_trip_with_label_settings_and_entries() {
        let (store, key) = (Id::random(), Key::random());
        let mut plain = IndexPlain {
            label: "production".into(),
            ..IndexPlain::default()
        };
        plain.secrets.insert(
            "db/postgres".into(),
            IndexEntry {
                id: Id::random(),
                version: 3,
                hash: Hash::of(b"c"),
            },
        );
        plain.otp.insert(
            "github".into(),
            IndexEntry {
                id: Id::random(),
                version: 1,
                hash: Hash::of(b"d"),
            },
        );
        let sealed = seal_index(store, 2, 9, &key, &plain).unwrap();
        assert_eq!(open_index(store, 2, 9, &key, &sealed).unwrap(), plain);
        assert!(open_index(store, 2, 8, &key, &sealed).is_err());
        assert_eq!(plain.ids().count(), 2);
    }

    #[test]
    fn the_store_label_is_not_visible_in_the_ciphertext() {
        let (store, key) = (Id::random(), Key::random());
        let plain = IndexPlain {
            label: "very-recognizable-label".into(),
            ..IndexPlain::default()
        };
        let sealed = seal_index(store, 0, 1, &key, &plain).unwrap();
        assert!(
            !sealed
                .windows(b"recognizable".len())
                .any(|window| window == b"recognizable")
        );
    }

    #[test]
    fn indexes_without_optional_maps_decode_with_defaults() {
        let plain: IndexPlain = serde_json::from_str(r#"{"label":"x"}"#).unwrap();
        assert_eq!(plain.settings, StoreSettings::default());
        assert!(plain.secrets.is_empty());
    }

    #[test]
    fn lookup_objects_round_trip() {
        let (store, key) = (Id::random(), Key::random());
        let mut plain = LookupPlain::default();
        plain
            .secrets
            .insert("example".into(), "\nurl: https://example.com".into());
        let sealed = seal_lookup(store, 0, 3, &key, &plain).unwrap();
        assert_eq!(open_lookup(store, 0, 3, &key, &sealed).unwrap(), plain);
        assert!(open_lookup(store, 0, 4, &key, &sealed).is_err());
    }

    fn payload(store: Id, version: u64, prev: Option<Hash>) -> IndexPayload {
        IndexPayload {
            store,
            version,
            prev,
            epoch: 0,
            log_head: Hash::default(),
            time: 0,
            index: Bytes(vec![]),
            lookup_hash: None,
            changed: vec![],
            deleted: vec![],
        }
    }

    #[test]
    fn index_chains_reject_rollbacks_forks_and_other_stores() {
        let key = SigningKey::generate(&mut OsRng);
        let device = Id::random();
        let store = Id::random();
        let first = payload(store, 1, None);
        let first_envelope = sign_index(&first, device, &key).unwrap();
        check_chain(&first_envelope, &first, store, None).unwrap();
        let pin = Pin {
            version: 1,
            hash: first_envelope.hash(),
        };

        let second = payload(store, 2, Some(first_envelope.hash()));
        let second_envelope = sign_index(&second, device, &key).unwrap();
        check_chain(&second_envelope, &second, store, Some(pin)).unwrap();

        let fork = payload(store, 2, Some(Hash::of(b"elsewhere")));
        let fork_envelope = sign_index(&fork, device, &key).unwrap();
        assert!(check_chain(&fork_envelope, &fork, store, Some(pin)).is_err());

        let pin_two = Pin {
            version: 2,
            hash: second_envelope.hash(),
        };
        assert!(check_chain(&first_envelope, &first, store, Some(pin_two)).is_err());
        assert!(check_chain(&fork_envelope, &fork, store, Some(pin_two)).is_err());
        check_chain(&second_envelope, &second, store, Some(pin_two)).unwrap();
        assert!(check_chain(&first_envelope, &first, Id::random(), None).is_err());

        let jump = payload(store, 7, Some(Hash::of(b"x")));
        let jump_envelope = sign_index(&jump, device, &key).unwrap();
        check_chain(&jump_envelope, &jump, store, Some(pin)).unwrap();
    }

    #[test]
    fn version_one_has_no_predecessor_and_later_versions_do() {
        let key = SigningKey::generate(&mut OsRng);
        let store = Id::random();
        for (payload, ok) in [
            (payload(store, 1, Some(Hash::default())), false),
            (payload(store, 2, None), false),
            (payload(store, 0, None), false),
            (payload(store, 1, None), true),
        ] {
            let envelope = sign_index(&payload, Id::random(), &key).unwrap();
            assert_eq!(check_chain(&envelope, &payload, store, None).is_ok(), ok);
        }
    }
}
