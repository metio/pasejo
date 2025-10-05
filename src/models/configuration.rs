// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{constants, environment_variables};
use crate::models::password_store::PasswordStore;
use crate::{identities, recipients, secrets};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env::var_os;
use std::fs;
use std::path::{absolute, Path, PathBuf};
use toml::Table;

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

    /// Time in seconds between automated execution of configured pull commands
    pub pull_interval_seconds: Option<u64>,

    /// Time in seconds between automated execution of configured push commands
    pub push_interval_seconds: Option<u64>,

    /// Global pull commands used for all stores
    pub pull_commands: Vec<String>,

    /// Global push commands used for all stores
    pub push_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StoreRegistration {
    /// The local file system path of the store
    pub path: PathBuf,

    /// The name of the store
    pub name: String,

    /// The identities used in the store
    pub identities: Vec<Identity>,

    /// The commands to execute when pulling changes into the store
    pub pull_commands: Vec<String>,

    /// The commands to execute when pushing changes from the store
    pub push_commands: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Identity {
    pub file: PathBuf,
}

impl Configuration {
    pub fn load_configuration() -> Result<Self> {
        let config_path = Self::config_path()?;
        if let Ok(config) = confy::load_path(&config_path) {
            Ok(config)
        } else {
            Self::migrate_configuration(&config_path).context("Could not migrate configuration")?;
            confy::load_path(&config_path).context("Could not load configuration")
        }
    }

    fn migrate_configuration(config_path: &Path) -> Result<()> {
        let config_content = fs::read_to_string(config_path)?;
        let mut migrated_config = config_content.parse::<Table>()?;

        if !migrated_config.contains_key("pull_commands") {
            migrated_config.insert(
                "pull_commands".to_string(),
                toml::Value::from(vec![] as Vec<String>),
            );
        }
        if !migrated_config.contains_key("push_commands") {
            migrated_config.insert(
                "push_commands".to_string(),
                toml::Value::from(vec![] as Vec<String>),
            );
        }
        if let Some(stores_value) = migrated_config.get_mut("stores")
            && let Some(stores) = stores_value.as_array_mut()
        {
            for store in stores {
                if let Some(table) = store.as_table_mut() {
                    let has_pull_commands = table.contains_key("pull_commands");
                    let has_push_commands = table.contains_key("push_commands");

                    if let Some(synchronizer) = table.remove("synchronizer") {
                        if let Some(used_synchronizer) = synchronizer.as_str() {
                            let mut pull_commands = Vec::new();
                            let mut push_commands = Vec::new();

                            match used_synchronizer {
                                "Git" => {
                                    pull_commands.push(String::from("git pull"));
                                    push_commands.push(String::from("git add %p"));
                                    push_commands
                                        .push(String::from("git commit --message 'pasejo commit'"));
                                    push_commands.push(String::from("git push"));
                                }
                                "Mercurial" => {
                                    pull_commands.push(String::from("hg pull"));
                                    push_commands.push(String::from("hg add %p"));
                                    push_commands
                                        .push(String::from("hg commit --message 'pasejo commit'"));
                                    push_commands.push(String::from("hg push"));
                                }
                                "Pijul" => {
                                    pull_commands.push(String::from("pijul pull"));
                                    push_commands.push(String::from("pijul add %p"));
                                    push_commands.push(String::from(
                                        "pijul record --message 'pasejo commit'",
                                    ));
                                    push_commands.push(String::from("pijul push"));
                                }
                                _ => {}
                            }

                            if !has_pull_commands {
                                table.insert(
                                    "pull_commands".to_string(),
                                    toml::Value::from(pull_commands),
                                );
                            }
                            if !has_push_commands {
                                table.insert(
                                    "push_commands".to_string(),
                                    toml::Value::from(push_commands),
                                );
                            }
                        }
                    } else {
                        if !has_pull_commands {
                            table.insert(
                                "pull_commands".to_string(),
                                toml::Value::from(vec![] as Vec<String>),
                            );
                        }
                        if !has_push_commands {
                            table.insert(
                                "push_commands".to_string(),
                                toml::Value::from(vec![] as Vec<String>),
                            );
                        }
                    }
                }
            }
        }

        confy::store_path(config_path, migrated_config).context("Could not store configuration")?;
        Ok(())
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

    pub fn add_store(&mut self, store_root_path: &str, store_name: &str) -> Result<()> {
        let registration = StoreRegistration {
            path: PathBuf::from(store_root_path),
            name: store_name.to_owned(),
            identities: vec![],
            pull_commands: vec![],
            push_commands: vec![],
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

    pub fn all_identity_files(&self, store: &StoreRegistration) -> Vec<PathBuf> {
        let mut identities = self.identities.clone();
        identities.extend(store.identities.clone());
        let mut files: Vec<PathBuf> = identities
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
        self.decrypt_store_from_path(registration, registration.path())
    }

    pub fn decrypt_store_from_path(
        &self,
        registration: &StoreRegistration,
        store_path: &Path,
    ) -> Result<PasswordStore> {
        let identity_files = self.all_identity_files(registration);
        let identities = identities::read(
            identity_files,
            self.ignore_missing_identities.unwrap_or(true),
        )?;
        let decrypted_store = secrets::decrypt(store_path, &identities)?;
        let store: PasswordStore = toml::from_str(&decrypted_store)?;
        Ok(store)
    }

    pub fn encrypt_store(registration: &StoreRegistration, store: &PasswordStore) -> Result<()> {
        Self::encrypt_store_to_path(store, registration.path())
    }

    pub fn encrypt_store_to_path(store: &PasswordStore, store_path: &Path) -> Result<()> {
        let recipients = recipients::read_recipients(&store.recipients)?;
        let store_toml = toml::to_string_pretty(&store)?;
        secrets::encrypt(&store_toml, store_path, &recipients)?;
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

impl StoreRegistration {
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }
}
