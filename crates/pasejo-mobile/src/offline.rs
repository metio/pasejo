// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The offline copy of values, kept while the organization allows offline
//! reads. Each value the device fetched from the server is sealed to the
//! device's own age recipient, so opening the copy needs the same
//! authentication as opening the stores, and the copy holds only values
//! whose first read the server recorded.
//!
//! Opening the copy for an unlock period reseals every value to a key that
//! exists only in memory for that period, as the online key ring holds
//! store keys: memory holds keys and sealed values, and a value is in the
//! clear only while one read uses it. The device's own identity cannot do
//! that job, because a hardware identity agrees only within seconds of an
//! authentication.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::str::FromStr;

use pasejo_protocol::Id;
use pasejo_protocol::encoding::b64;
use pasejo_protocol::store::{ObjectPlain, StoreSettings};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::MobileError;

/// One sealed value, stored in the device state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub store: Id,
    pub id: Id,
    pub version: u64,
    /// The age-encrypted [`Plain`], base64-encoded.
    pub sealed: String,
}

/// What a record seals: the value and what the matcher needs around it.
#[derive(Serialize, Deserialize)]
pub struct Plain {
    pub label: String,
    pub settings: StoreSettings,
    pub name: String,
    pub object: ObjectPlain,
}

/// A value of the opened copy, still sealed, with the identifier and
/// version a read report names.
pub struct Entry {
    pub id: Id,
    pub version: u64,
    sealed: Record,
}

/// One store of the opened copy: names and settings, values sealed.
pub struct Store {
    pub label: String,
    pub settings: StoreSettings,
    pub secrets: BTreeMap<String, Entry>,
    pub otp: BTreeMap<String, Entry>,
}

/// The copy opened for one unlock period.
pub struct Opened {
    key: age::x25519::Identity,
    pub stores: BTreeMap<Id, Store>,
}

impl Opened {
    /// Opens every record with `identity` and reseals it to a fresh key; a
    /// later record of the same name replaces an earlier one.
    ///
    /// # Errors
    ///
    /// Fails when any record does not open.
    pub fn open(identity: &dyn age::Identity, records: &[Record]) -> Result<Self, MobileError> {
        let key = age::x25519::Identity::generate();
        let recipient = key.to_public().to_string();
        let mut stores: BTreeMap<Id, Store> = BTreeMap::new();
        for record in records {
            let plain = open(identity, record)?;
            let sealed = seal(&recipient, record.store, record.id, record.version, &plain)?;
            let store = stores.entry(record.store).or_insert_with(|| Store {
                label: plain.label.clone(),
                settings: plain.settings.clone(),
                secrets: BTreeMap::new(),
                otp: BTreeMap::new(),
            });
            let entry = Entry {
                id: record.id,
                version: record.version,
                sealed,
            };
            match &plain.object {
                ObjectPlain::Secret { .. } => store.secrets.insert(plain.name.clone(), entry),
                ObjectPlain::Otp { .. } => store.otp.insert(plain.name.clone(), entry),
            };
        }
        Ok(Self { key, stores })
    }

    /// The value of one entry, in the clear for the caller's use only.
    ///
    /// # Errors
    ///
    /// Fails when the entry does not open with this period's key.
    pub fn value(&self, entry: &Entry) -> Result<ObjectPlain, MobileError> {
        open(&self.key, &entry.sealed).map(|plain| plain.object)
    }
}

/// Seals `plain` to the age `recipient`.
///
/// # Errors
///
/// Fails for a malformed recipient.
pub fn seal(
    recipient: &str,
    store: Id,
    id: Id,
    version: u64,
    plain: &Plain,
) -> Result<Record, MobileError> {
    let recipient = age::x25519::Recipient::from_str(recipient)
        .map_err(|error| MobileError::key(error.to_string()))?;
    let json = Zeroizing::new(
        serde_json::to_vec(plain).map_err(|error| MobileError::invalid(error.to_string()))?,
    );
    let encryptor =
        age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
            .map_err(|error| MobileError::key(error.to_string()))?;
    let mut sealed = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut sealed)
        .map_err(|error| MobileError::key(error.to_string()))?;
    writer
        .write_all(&json)
        .and_then(|()| writer.finish().map(|_| ()))
        .map_err(|error| MobileError::key(error.to_string()))?;
    Ok(Record {
        store,
        id,
        version,
        sealed: b64(&sealed),
    })
}

/// Opens one record with `identity`.
///
/// # Errors
///
/// Fails when the identity cannot open the record or its content is
/// malformed.
pub fn open(identity: &dyn age::Identity, record: &Record) -> Result<Plain, MobileError> {
    let sealed = pasejo_protocol::encoding::unb64(&record.sealed)
        .map_err(|_| MobileError::verification("an offline record is not base64"))?;
    let decryptor = age::Decryptor::new(sealed.as_slice())
        .map_err(|_| MobileError::verification("an offline record is malformed"))?;
    let mut reader = decryptor
        .decrypt(std::iter::once(identity))
        .map_err(|_| MobileError::verification("an offline record does not open"))?;
    let mut json = Zeroizing::new(Vec::new());
    reader
        .read_to_end(&mut json)
        .map_err(|_| MobileError::verification("an offline record does not open"))?;
    serde_json::from_slice(&json)
        .map_err(|_| MobileError::verification("an offline record is malformed"))
}

/// Adds `record`, replacing an older version of the same object.
pub fn upsert(records: &mut Vec<Record>, record: Record) {
    records.retain(|existing| !(existing.store == record.store && existing.id == record.id));
    records.push(record);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use pasejo_protocol::store::OtpParams;

    use super::*;

    fn identity() -> (age::x25519::Identity, String) {
        let identity = age::x25519::Identity::generate();
        let recipient = identity.to_public().to_string();
        (identity, recipient)
    }

    fn secret(name: &str, value: &str) -> Plain {
        Plain {
            label: "Work".into(),
            settings: StoreSettings::default(),
            name: name.into(),
            object: ObjectPlain::Secret {
                secret: value.into(),
            },
        }
    }

    #[test]
    fn a_record_opens_with_its_identity_only() {
        let (identity, recipient) = identity();
        let record = seal(
            &recipient,
            Id::random(),
            Id::random(),
            3,
            &secret("mail", "hunter2"),
        )
        .unwrap();
        assert!(!record.sealed.contains("hunter2"));
        let plain = open(&identity, &record).unwrap();
        assert_eq!(plain.name, "mail");
        assert_eq!(
            plain.object,
            ObjectPlain::Secret {
                secret: "hunter2".into()
            }
        );
        let (other, _) = self::identity();
        assert!(matches!(
            open(&other, &record),
            Err(MobileError::Verification { .. })
        ));
    }

    #[test]
    fn a_tampered_record_does_not_open() {
        let (identity, recipient) = identity();
        let mut record = seal(
            &recipient,
            Id::random(),
            Id::random(),
            1,
            &secret("mail", "x"),
        )
        .unwrap();
        let mut bytes = pasejo_protocol::encoding::unb64(&record.sealed).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        record.sealed = b64(&bytes);
        assert!(open(&identity, &record).is_err());
        record.sealed = "not base64!".into();
        assert!(open(&identity, &record).is_err());
    }

    #[test]
    fn the_view_groups_records_by_store_and_kind() {
        let (identity, recipient) = identity();
        let (work, home) = (Id::random(), Id::random());
        let otp = Plain {
            label: "Work".into(),
            settings: StoreSettings::default(),
            name: "mail".into(),
            object: ObjectPlain::Otp {
                otp: OtpParams {
                    secret: "JBSWY3DPEHPK3PXP".into(),
                    otp_type: "totp".into(),
                    algorithm: "sha1".into(),
                    digits: 6,
                    period: 30,
                    counter: 0,
                    skew: 0,
                },
            },
        };
        let mail = Id::random();
        let records = vec![
            seal(&recipient, work, mail, 1, &secret("mail", "a")).unwrap(),
            seal(&recipient, work, Id::random(), 1, &otp).unwrap(),
            seal(
                &recipient,
                home,
                Id::random(),
                2,
                &Plain {
                    label: "Home".into(),
                    ..secret("bank", "b")
                },
            )
            .unwrap(),
        ];
        let opened = Opened::open(&identity, &records).unwrap();
        let stores = &opened.stores;
        assert_eq!(stores.len(), 2);
        assert_eq!(stores[&work].label, "Work");
        assert!(matches!(
            opened.value(&stores[&work].secrets["mail"]).unwrap(),
            ObjectPlain::Secret { ref secret } if secret == "a"
        ));
        assert_eq!(stores[&work].secrets["mail"].id, mail);
        assert!(matches!(
            opened.value(&stores[&work].otp["mail"]).unwrap(),
            ObjectPlain::Otp { ref otp } if otp.digits == 6
        ));
        assert_eq!(stores[&home].label, "Home");
        assert_eq!(stores[&home].secrets["bank"].version, 2);
    }

    #[test]
    fn the_opened_copy_holds_values_sealed_to_a_key_of_its_own() {
        let (identity, recipient) = identity();
        let store = Id::random();
        let records = vec![
            seal(
                &recipient,
                store,
                Id::random(),
                1,
                &secret("mail", "hunter2-offline"),
            )
            .unwrap(),
        ];
        let opened = Opened::open(&identity, &records).unwrap();
        let entry = &opened.stores[&store].secrets["mail"];
        assert_ne!(entry.sealed.sealed, records[0].sealed);
        assert!(open(&identity, &entry.sealed).is_err());
        let bytes = pasejo_protocol::encoding::unb64(&entry.sealed.sealed).unwrap();
        assert!(!bytes.windows(15).any(|window| window == b"hunter2-offline"));
        assert!(matches!(
            opened.value(entry).unwrap(),
            ObjectPlain::Secret { ref secret } if secret == "hunter2-offline"
        ));
    }

    #[test]
    fn each_opening_uses_another_key() {
        let (identity, recipient) = identity();
        let store = Id::random();
        let records = vec![seal(&recipient, store, Id::random(), 1, &secret("mail", "a")).unwrap()];
        let first = Opened::open(&identity, &records).unwrap();
        let second = Opened::open(&identity, &records).unwrap();
        assert!(first.value(&second.stores[&store].secrets["mail"]).is_err());
    }

    #[test]
    fn a_copy_another_identity_sealed_does_not_open() {
        let (_, recipient) = identity();
        let (other, _) = identity();
        let records = vec![
            seal(
                &recipient,
                Id::random(),
                Id::random(),
                1,
                &secret("mail", "a"),
            )
            .unwrap(),
        ];
        assert!(Opened::open(&other, &records).is_err());
    }

    #[test]
    fn a_new_version_replaces_the_old_one() {
        let (_, recipient) = identity();
        let (store, id) = (Id::random(), Id::random());
        let mut records = vec![];
        upsert(
            &mut records,
            seal(&recipient, store, id, 1, &secret("mail", "old")).unwrap(),
        );
        upsert(
            &mut records,
            seal(&recipient, store, Id::random(), 1, &secret("bank", "b")).unwrap(),
        );
        upsert(
            &mut records,
            seal(&recipient, store, id, 2, &secret("mail", "new")).unwrap(),
        );
        assert_eq!(records.len(), 2);
        assert_eq!(
            records
                .iter()
                .find(|record| record.id == id)
                .unwrap()
                .version,
            2
        );
    }
}
