// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Devices: the installations that hold signing keys and receive wrapped
//! epoch keys.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Error;
use crate::encoding::{Fixed, Id, unb64};

/// How a device receives wrapped epoch keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    /// An age recipient: X25519, SSH or an age plugin. CLI and bridge
    /// devices.
    Age,
    /// An X25519 public key for HPKE. Browser devices, which only ever
    /// receive names-only wrappings.
    Hpke,
}

impl DeviceKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Age => "age",
            Self::Hpke => "hpke",
        }
    }
}

/// A device as the membership log records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: Id,
    pub user: Id,
    pub kind: DeviceKind,
    /// The age recipient string, or the base64url X25519 public key of an
    /// HPKE device.
    pub recipient: String,
    pub signing_key: Fixed<32>,
    pub label: String,
}

/// Longest device label the log accepts.
pub const MAX_LABEL_LEN: usize = 200;
/// Longest age recipient the log accepts; plugin recipients are long, but
/// not this long.
pub const MAX_RECIPIENT_LEN: usize = 2048;

impl DeviceRecord {
    /// The fingerprint approvers compare: the first 20 bytes of
    /// `SHA-256(kind ‖ 0x00 ‖ recipient ‖ 0x00 ‖ signing key)` as five groups
    /// of eight hexadecimal characters.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        fingerprint(self.kind, &self.recipient, &self.signing_key.0)
    }

    /// Checks the record's shape. Whether its author may add it is the
    /// membership log's decision.
    ///
    /// # Errors
    ///
    /// Fails for empty or overlong labels and malformed recipients.
    pub fn validate(&self) -> Result<(), Error> {
        if self.label.trim().is_empty()
            || self.label.len() > MAX_LABEL_LEN
            || self.label.chars().any(char::is_control)
        {
            return Err(Error::Invalid("device label"));
        }
        validate_recipient(self.kind, &self.recipient)
    }
}

/// Checks that `recipient` has the shape `kind` requires.
///
/// # Errors
///
/// Fails for malformed recipients.
pub fn validate_recipient(kind: DeviceKind, recipient: &str) -> Result<(), Error> {
    match kind {
        DeviceKind::Hpke => {
            let bytes = unb64(recipient).map_err(|_| Error::Invalid("hpke recipient"))?;
            if bytes.len() == 32 {
                Ok(())
            } else {
                Err(Error::Invalid("hpke recipient"))
            }
        }
        DeviceKind::Age => {
            let shaped = !recipient.is_empty()
                && recipient.len() <= MAX_RECIPIENT_LEN
                && recipient.is_ascii()
                && !recipient.chars().any(|c| c.is_ascii_control())
                && (recipient.starts_with("age1") || recipient.starts_with("ssh-"));
            if shaped {
                Ok(())
            } else {
                Err(Error::Invalid("age recipient"))
            }
        }
    }
}

/// See [`DeviceRecord::fingerprint`].
#[must_use]
pub fn fingerprint(kind: DeviceKind, recipient: &str, signing_key: &[u8; 32]) -> String {
    let mut digest = Sha256::new();
    digest.update(kind.label().as_bytes());
    digest.update([0]);
    digest.update(recipient.as_bytes());
    digest.update([0]);
    digest.update(signing_key);
    groups(&digest.finalize())
}

/// An organization's fingerprint: the first 20 bytes of its genesis entry's
/// hash, grouped like a device fingerprint. Members compare it over another
/// channel when they join.
#[must_use]
pub fn org_fingerprint(genesis: &crate::encoding::Hash) -> String {
    groups(&genesis.0)
}

/// The first 20 bytes of `hash` as five groups of eight hexadecimal
/// characters.
fn groups(hash: &[u8]) -> String {
    let mut text = String::with_capacity(44);
    for (index, byte) in hash[..20].iter().enumerate() {
        if index > 0 && index % 4 == 0 {
            text.push(' ');
        }
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// The identifier under which wrappings for the organization's recovery
/// recipient are stored: the first 16 bytes of
/// `SHA-256("pasejo recovery v1" ‖ 0x00 ‖ recipient)`.
#[must_use]
pub fn recovery_id(recipient: &str) -> Id {
    let mut digest = Sha256::new();
    digest.update(b"pasejo recovery v1");
    digest.update([0]);
    digest.update(recipient.as_bytes());
    let hash = digest.finalize();
    let mut id = [0u8; 16];
    id.copy_from_slice(&hash[..16]);
    Id(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::b64;

    #[test]
    fn org_fingerprints_group_the_genesis_hash() {
        let hash = crate::encoding::Hash(core::array::from_fn(|index| {
            u8::try_from(index).unwrap_or(0)
        }));
        assert_eq!(
            org_fingerprint(&hash),
            "00010203 04050607 08090a0b 0c0d0e0f 10111213"
        );
    }

    fn record() -> DeviceRecord {
        DeviceRecord {
            id: Id::random(),
            user: Id::random(),
            kind: DeviceKind::Age,
            recipient: "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p".into(),
            signing_key: Fixed([1; 32]),
            label: "laptop".into(),
        }
    }

    #[test]
    fn fingerprints_have_five_groups_and_depend_on_every_key() {
        let device = record();
        let print = device.fingerprint();
        assert_eq!(print.len(), 44);
        assert_eq!(print.split(' ').count(), 5);
        assert!(print.split(' ').all(|group| group.len() == 8));
        let mut other = device.clone();
        other.signing_key = Fixed([2; 32]);
        assert_ne!(other.fingerprint(), print);
        let mut other = device.clone();
        other.recipient.push('x');
        assert_ne!(other.fingerprint(), print);
        let mut other = device;
        other.kind = DeviceKind::Hpke;
        assert_ne!(other.fingerprint(), print);
    }

    #[test]
    fn the_fingerprint_ignores_label_and_identifiers() {
        let device = record();
        let mut other = device.clone();
        other.label = "renamed".into();
        other.id = Id::random();
        assert_eq!(other.fingerprint(), device.fingerprint());
    }

    #[test]
    fn age_recipients_are_checked_for_shape() {
        assert!(validate_recipient(DeviceKind::Age, &record().recipient).is_ok());
        assert!(validate_recipient(DeviceKind::Age, "ssh-ed25519 AAAAC3Nza").is_ok());
        assert!(validate_recipient(DeviceKind::Age, "age1yubikey1qabc").is_ok());
        assert!(validate_recipient(DeviceKind::Age, "").is_err());
        assert!(validate_recipient(DeviceKind::Age, "pgp key").is_err());
        assert!(validate_recipient(DeviceKind::Age, "age1\nage1").is_err());
        assert!(validate_recipient(DeviceKind::Age, &"age1".repeat(600)).is_err());
    }

    #[test]
    fn hpke_recipients_are_32_byte_keys() {
        assert!(validate_recipient(DeviceKind::Hpke, &b64(&[5; 32])).is_ok());
        assert!(validate_recipient(DeviceKind::Hpke, &b64(&[5; 31])).is_err());
        assert!(validate_recipient(DeviceKind::Hpke, "age1abc").is_err());
    }

    #[test]
    fn labels_must_be_present_short_and_printable() {
        let mut device = record();
        device.validate().unwrap();
        device.label = " ".into();
        assert!(device.validate().is_err());
        device.label = "a\u{7}b".into();
        assert!(device.validate().is_err());
        device.label = "x".repeat(MAX_LABEL_LEN + 1);
        assert!(device.validate().is_err());
    }

    #[test]
    fn recovery_identifiers_are_stable_per_recipient() {
        assert_eq!(recovery_id("age1abc"), recovery_id("age1abc"));
        assert_ne!(recovery_id("age1abc"), recovery_id("age1abd"));
    }

    #[test]
    fn kinds_serialize_lowercase() {
        assert_eq!(
            serde_json::to_string(&DeviceKind::Hpke).unwrap(),
            "\"hpke\""
        );
        assert_eq!(
            serde_json::from_str::<DeviceKind>("\"age\"").unwrap(),
            DeviceKind::Age
        );
    }
}
