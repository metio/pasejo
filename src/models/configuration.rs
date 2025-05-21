// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env::var_os;
use std::path::{Path, PathBuf, absolute};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::{constants, environment_variables};
use crate::models::password_store::PasswordStore;
use crate::synchronizers::synchronizer::Synchronizers;
use crate::{identities, recipients, secrets};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All registered stores the user has configured on their system
    pub stores: Vec<StoreRegistration>,

    /// Global identities used for all stores
    pub identities: Vec<Identity>,

    /// The default store to use when no store name was specified
    pub default_store: Option<String>,

    /// Toggle whether missing identity files will be ignored
    pub ignore_missing_identities: Option<bool>,

    /// How long should secrets/OTPs be kept in the clipboard in seconds?
    pub clipboard_timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, clap::ValueEnum)]
pub enum ConfigurationOption {
    IgnoreMissingIdentities,
    ClipboardTimeout,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StoreRegistration {
    /// The local file system path of the store
    pub path: String,

    /// The name of the store
    pub name: String,

    /// The synchronizer used in the store
    pub synchronizer: Synchronizers,

    /// The identities used in the store
    pub identities: Vec<Identity>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Identity {
    pub file: String,
}

impl Configuration {
    pub fn load_configuration() -> Result<Self> {
        confy::load_path(Self::config_path()?).context("Could not load configuration")
    }

    pub fn save_configuration(&self) -> Result<()> {
        confy::store_path(Self::config_path()?, self).context("Could not store configuration")
    }

    fn config_path() -> Result<PathBuf> {
        var_os(environment_variables::PASEJO_CONFIG).map_or_else(
            || {
                confy::get_configuration_file_path(constants::APPLICATION_NAME, "config")
                    .context("Could not determine configuration path")
            },
            |path| {
                absolute(PathBuf::from(path))
                    .context("Could not resolve absolute path to configuration")
            },
        )
    }

    pub fn add_store(
        &mut self,
        store_root_path: &str,
        store_name: &str,
        synchronizer: Synchronizers,
    ) -> Result<()> {
        let registration = StoreRegistration {
            path: store_root_path.to_string(),
            name: store_name.to_owned(),
            synchronizer,
            identities: vec![],
        };
        self.stores.push(registration);
        self.save_configuration()
    }

    pub fn remove_store(&mut self, store_name: &str) -> Result<()> {
        self.default_store
            .take_if(|value| value.eq_ignore_ascii_case(store_name));
        self.stores
            .retain(|store| !store.name.eq_ignore_ascii_case(store_name));
        self.save_configuration()
    }

    fn default_store_name(&self) -> Option<String> {
        var_os(environment_variables::PASEJO_DEFAULT_STORE).map_or_else(
            || self.default_store.clone(),
            |value| value.into_string().ok(),
        )
    }

    pub fn set_default_store(&mut self, store_name: &str) -> Result<()> {
        self.default_store = Some(store_name.to_owned());
        self.save_configuration()
    }

    pub fn remove_identity(
        &mut self,
        identity: &Identity,
        store_name: Option<&String>,
        global: bool,
    ) -> Result<()> {
        if global {
            self.identities.retain(|i| i.file != identity.file);
            self.save_configuration()?;
        } else if let Some(store) = self.select_store_mut(store_name) {
            store.identities.retain(|i| i.file != identity.file);
            self.save_configuration()?;
        }
        Ok(())
    }

    pub fn has_identity(
        &mut self,
        identity: &Identity,
        store_name: Option<&String>,
        global: bool,
    ) -> bool {
        if global {
            return self.identities.iter().any(|i| i.file == identity.file);
        } else if let Some(store) = self.select_store_mut(store_name) {
            return store.identities.iter().any(|i| i.file == identity.file);
        }
        false
    }

    pub fn all_identity_files(&self, store: &StoreRegistration) -> Vec<String> {
        let mut identities = self.identities.clone();
        identities.extend(store.identities.clone());
        let mut files: Vec<String> = identities
            .iter()
            .map(|identity| identity.file.clone())
            .collect();
        files.sort();
        files.dedup();
        files
    }

    pub fn all_store_names(&self) -> Vec<String> {
        let mut names = vec![];
        for store in &self.stores {
            names.push(store.name.clone());
        }
        names
    }

    pub fn decrypt_store(&self, registration: &StoreRegistration) -> Result<PasswordStore> {
        let identity_files = self.all_identity_files(registration);
        let identities = identities::read(
            identity_files,
            self.ignore_missing_identities.unwrap_or(true),
        )?;
        let decrypted_store = secrets::decrypt(Path::new(&registration.path), &identities)?;
        let store: PasswordStore = toml::from_str(&decrypted_store)?;
        Ok(store)
    }

    pub fn encrypt_store(registration: &StoreRegistration, store: &PasswordStore) -> Result<()> {
        let recipients = recipients::read_recipients(&store.recipients)?;
        let store_toml = toml::to_string_pretty(&store)?;
        secrets::encrypt(&store_toml, Path::new(&registration.path), &recipients)?;
        Ok(())
    }

    pub fn select_store(&self, store_name: Option<&String>) -> Option<&StoreRegistration> {
        store_name
            .cloned()
            .or_else(|| self.default_store_name())
            .map_or_else(
                || self.stores.first(),
                |name| self.find_store(name.as_str()),
            )
    }

    pub fn select_store_mut(
        &mut self,
        store_name: Option<&String>,
    ) -> Option<&mut StoreRegistration> {
        if let Some(name) = store_name.cloned().or_else(|| self.default_store_name()) {
            self.find_store_mut(name.as_str())
        } else {
            self.stores.first_mut()
        }
    }

    pub fn find_store(&self, store_name: &str) -> Option<&StoreRegistration> {
        self.stores
            .iter()
            .find(|store| store.name.eq_ignore_ascii_case(store_name))
    }

    fn find_store_mut(&mut self, store_name: &str) -> Option<&mut StoreRegistration> {
        self.stores
            .iter_mut()
            .find(|store| store.name.eq_ignore_ascii_case(store_name))
    }
}
