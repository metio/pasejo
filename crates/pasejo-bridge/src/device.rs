// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The bridge's device keys.
//!
//! An identifier, the Ed25519 key that signs its requests and claims, and
//! the age identity store keys are wrapped to. `pasejo-bridge keygen` writes
//! them as one JSON file, which the operator stores in a Kubernetes Secret and
//! mounts read-only.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use age::secrecy::ExposeSecret;
use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use pasejo_client::transport::DeviceIdentity;
use pasejo_protocol::Id;
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::{b64, unb64};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

#[derive(Serialize, Deserialize)]
pub struct DeviceFile {
    pub id: String,
    pub signing_key: String,
    pub identity: String,
}

impl Drop for DeviceFile {
    fn drop(&mut self) {
        self.signing_key.zeroize();
        self.identity.zeroize();
    }
}

/// Loaded device keys.
pub struct Device {
    pub id: Id,
    pub signing_key: SigningKey,
    pub identity: age::x25519::Identity,
}

impl Device {
    /// New random keys.
    #[must_use]
    pub fn generate() -> Self {
        Self {
            id: Id::random(),
            signing_key: SigningKey::generate(&mut OsRng),
            identity: age::x25519::Identity::generate(),
        }
    }

    /// The recipient store keys are wrapped to.
    #[must_use]
    pub fn recipient(&self) -> String {
        self.identity.to_public().to_string()
    }

    /// The device as the client presents it, labelled `label`.
    #[must_use]
    pub fn identity(&self, label: &str) -> DeviceIdentity {
        DeviceIdentity {
            id: self.id,
            signing_key: self.signing_key.clone(),
            kind: DeviceKind::Age,
            recipient: self.recipient(),
            label: label.to_owned(),
        }
    }

    /// The identities that open this device's wrappings.
    #[must_use]
    pub fn identities(&self) -> Vec<Box<dyn age::Identity>> {
        vec![Box::new(self.identity.clone())]
    }

    /// The JSON form of the keys.
    ///
    /// # Errors
    ///
    /// Fails when serialization does.
    pub fn to_json(&self) -> Result<Zeroizing<String>> {
        let file = DeviceFile {
            id: self.id.to_string(),
            signing_key: b64(self.signing_key.as_bytes()),
            identity: self.identity.to_string().expose_secret().to_owned(),
        };
        Ok(Zeroizing::new(serde_json::to_string_pretty(&file)?))
    }

    /// Parses keys written by [`Self::to_json`].
    ///
    /// # Errors
    ///
    /// Fails for malformed JSON, identifiers or keys.
    pub fn from_json(text: &str) -> Result<Self> {
        let file: DeviceFile = serde_json::from_str(text).context("malformed device file")?;
        let id = Id::from_str(&file.id).context("malformed device identifier")?;
        let key = Zeroizing::new(unb64(&file.signing_key).context("malformed signing key")?);
        let key: [u8; 32] = key
            .as_slice()
            .try_into()
            .context("the signing key is not 32 bytes long")?;
        let identity = age::x25519::Identity::from_str(file.identity.trim())
            .map_err(|error| anyhow::anyhow!("malformed age identity: {error}"))?;
        Ok(Self {
            id,
            signing_key: SigningKey::from_bytes(&key),
            identity,
        })
    }

    /// Reads keys from `path`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let text = Zeroizing::new(
            fs::read_to_string(path)
                .with_context(|| format!("cannot read device file {}", path.display()))?,
        );
        Self::from_json(&text)
    }

    /// Writes the keys to a new file at `path`, readable by its owner only.
    ///
    /// # Errors
    ///
    /// Fails when the file exists or cannot be written.
    pub fn create(&self, path: &Path) -> Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(path)
            .with_context(|| format!("cannot create device file {}", path.display()))?;
        file.write_all(self.to_json()?.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn keys_round_trip_through_their_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device.json");
        let device = Device::generate();
        device.create(&path).unwrap();
        let loaded = Device::load(&path).unwrap();
        assert_eq!(loaded.id, device.id);
        assert_eq!(loaded.signing_key.as_bytes(), device.signing_key.as_bytes());
        assert_eq!(loaded.recipient(), device.recipient());
        assert!(loaded.recipient().starts_with("age1"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        assert!(device.create(&path).is_err(), "an existing file stays");
    }

    #[test]
    fn malformed_files_are_refused() {
        let device = Device::generate();
        let good: serde_json::Value = serde_json::from_str(&device.to_json().unwrap()).unwrap();
        for (field, value) in [
            ("id", "not-an-id"),
            ("signing_key", "AAAA"),
            ("signing_key", "!"),
            ("identity", "AGE-SECRET-KEY-1NOPE"),
        ] {
            let mut bad = good.clone();
            bad[field] = value.into();
            assert!(Device::from_json(&bad.to_string()).is_err(), "{field}");
        }
        assert!(Device::from_json("{}").is_err());
        assert!(Device::load(Path::new("/nonexistent/device.json")).is_err());
    }

    #[test]
    fn the_client_identity_carries_the_age_recipient() {
        let device = Device::generate();
        let identity = device.identity("bridge");
        assert_eq!(identity.kind, DeviceKind::Age);
        assert_eq!(identity.recipient, device.recipient());
        assert_eq!(identity.label, "bridge");
        assert_eq!(device.identities().len(), 1);
    }
}
