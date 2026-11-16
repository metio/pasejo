// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! A device's files in the local data directory: its signing key, its
//! session and the organization logs it verified, all mode `0600`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use pasejo_client::Session;
use pasejo_protocol::encoding::{b64, unb64};
use pasejo_protocol::envelope::Envelope;
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::{Id, Millis};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::cli::atomic_write;
use crate::cli::data_dir::local_data_dir;
use crate::cli::i18n;

/// The directory of one device: `servers/<device>` in the data directory.
#[derive(Debug, Clone)]
pub struct DeviceFiles {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct StoredSession {
    token: String,
    expires: Millis,
    user: Option<Id>,
}

impl DeviceFiles {
    /// # Errors
    ///
    /// Fails when no data directory exists.
    #[cfg(test)]
    const fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn new(device: Id) -> Result<Self> {
        let base =
            local_data_dir().ok_or_else(|| anyhow::anyhow!(i18n::error_remote_no_data_dir()))?;
        Ok(Self {
            dir: base.join("servers").join(device.to_string()),
        })
    }

    fn signing_key_path(&self) -> PathBuf {
        self.dir.join("signing.key")
    }

    fn session_path(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    fn log_path(&self, org: Id) -> PathBuf {
        self.dir.join("orgs").join(format!("{org}.json"))
    }

    /// Generates and stores a new signing key.
    ///
    /// # Errors
    ///
    /// Fails when the key cannot be written.
    pub fn create_signing_key(&self) -> Result<SigningKey> {
        let key = SigningKey::generate(&mut OsRng);
        let text = Zeroizing::new(b64(key.as_bytes()));
        atomic_write::write(&self.signing_key_path(), text.as_bytes())?;
        Ok(key)
    }

    /// # Errors
    ///
    /// Fails when the key is missing or malformed.
    pub fn signing_key(&self) -> Result<SigningKey> {
        let path = self.signing_key_path();
        let display = path.display().to_string();
        let text = Zeroizing::new(
            fs::read_to_string(&path)
                .with_context(|| i18n::error_remote_signing_key_unreadable(&display))?,
        );
        let bytes = Zeroizing::new(unb64(text.trim()).ok().unwrap_or_default());
        let bytes: &[u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!(i18n::error_remote_signing_key_unreadable(&display)))?;
        Ok(SigningKey::from_bytes(bytes))
    }

    /// The stored session, if one exists.
    ///
    /// # Errors
    ///
    /// Fails when the session file exists but cannot be read.
    pub fn session(&self) -> Result<Option<Session>> {
        let path = self.session_path();
        if !path.exists() {
            return Ok(None);
        }
        let text = Zeroizing::new(fs::read_to_string(&path)?);
        let stored: StoredSession = serde_json::from_str(&text)?;
        Ok(Some(Session {
            token: Zeroizing::new(stored.token),
            expires: stored.expires,
            user: stored.user,
        }))
    }

    /// # Errors
    ///
    /// Fails when the session cannot be written.
    pub fn save_session(&self, session: &Session) -> Result<()> {
        let stored = StoredSession {
            token: session.token.to_string(),
            expires: session.expires,
            user: session.user,
        };
        let text = Zeroizing::new(serde_json::to_string(&stored)?);
        atomic_write::write(&self.session_path(), text.as_bytes())
    }

    /// # Errors
    ///
    /// Fails when an existing session file cannot be removed.
    pub fn clear_session(&self) -> Result<()> {
        match fs::remove_file(self.session_path()) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// The log entries cached for `org`, unverified.
    ///
    /// # Errors
    ///
    /// Fails when the cache exists but cannot be read.
    pub fn cached_log(&self, org: Id) -> Result<Option<Vec<Envelope>>> {
        let path = self.log_path(org);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
    }

    /// # Errors
    ///
    /// Fails when the cache cannot be written.
    pub fn save_log(&self, org: Id, log: &OrgLog) -> Result<()> {
        let text = serde_json::to_string(log.entries())?;
        atomic_write::write(&self.log_path(org), text.as_bytes())
    }

    /// Removes every file of this device.
    ///
    /// # Errors
    ///
    /// Fails when the directory exists but cannot be removed.
    pub fn remove(&self) -> Result<()> {
        match fs::remove_dir_all(&self.dir) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// The default cache directory of a remote store.
    #[must_use]
    pub fn store_cache(&self, store: Id) -> PathBuf {
        self.dir.join("stores").join(store.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> (assert_fs::TempDir, DeviceFiles) {
        let temp = assert_fs::TempDir::new().unwrap();
        let files = DeviceFiles::at(temp.path().join("servers").join("device"));
        (temp, files)
    }

    #[test]
    fn the_signing_key_is_written_privately_and_read_back() {
        i18n::init_for_tests();
        let (_temp, files) = files();
        assert!(files.signing_key().is_err());
        let key = files.create_signing_key().unwrap();
        assert_eq!(files.signing_key().unwrap().to_bytes(), key.to_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(files.signing_key_path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn a_malformed_signing_key_names_its_file() {
        i18n::init_for_tests();
        let (_temp, files) = files();
        fs::create_dir_all(&files.dir).unwrap();
        for text in ["", "not base64!", &b64(&[1; 31])] {
            fs::write(files.signing_key_path(), text).unwrap();
            let error = files.signing_key().unwrap_err().to_string();
            assert!(error.contains("signing.key"), "{error}");
        }
    }

    #[test]
    fn sessions_round_trip_and_clear() {
        let (_temp, files) = files();
        assert!(files.session().unwrap().is_none());
        files.clear_session().unwrap();
        let user = Some(Id([4; 16]));
        files
            .save_session(&Session {
                token: Zeroizing::new("token".into()),
                expires: 99,
                user,
            })
            .unwrap();
        let session = files.session().unwrap().unwrap();
        assert_eq!(
            (session.token.as_str(), session.expires, session.user),
            ("token", 99, user)
        );
        files.clear_session().unwrap();
        assert!(files.session().unwrap().is_none());
    }

    #[test]
    fn missing_caches_read_as_nothing_and_removal_is_idempotent() {
        let (_temp, files) = files();
        assert!(files.cached_log(Id([1; 16])).unwrap().is_none());
        files.remove().unwrap();
        files.create_signing_key().unwrap();
        files.remove().unwrap();
        assert!(!files.dir.exists());
        assert_eq!(
            files.store_cache(Id([2; 16])),
            files.dir.join("stores").join(Id([2; 16]).to_string())
        );
    }
}
