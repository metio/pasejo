// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env::var_os;
use std::path::{Path, PathBuf, absolute};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::adapters::file_system;
use crate::adapters::vcs::{VersionControlSystem, VersionControlSystems};
use crate::cli::{constants, environment_variables};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All known stores the user has configured on their system
    pub stores: Vec<Store>,

    /// Global identities used for all stores
    pub identities: Vec<Identity>,

    /// The default store to use when no store name was specified
    pub default_store: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Store {
    /// The local file system path of a store
    pub path: String,

    /// The name for a store
    pub name: String,

    /// The VCS used in a single store
    pub vcs: VersionControlSystems,

    /// The identities used for a single store
    pub identities: Vec<Identity>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Identity {
    pub file: String,
}

impl Configuration {
    pub fn load() -> Result<Self> {
        confy::load_path(Self::config_path()?).context("Could not load configuration")
    }

    fn store(&self) -> Result<()> {
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
        store_root_path: String,
        store_name: &str,
        vcs: VersionControlSystems,
    ) -> Result<()> {
        if let Some(store) = self.find_store(store_name) {
            Err(anyhow!("Store with name '{}' already exists", store.name))
        } else {
            self.stores.push(Store {
                path: store_root_path,
                name: store_name.to_owned(),
                vcs,
                identities: vec![],
            });
            self.store()?;
            Ok(())
        }
    }

    pub fn remove_store(&mut self, store_name: &str) -> Result<String> {
        let store = self
            .find_store(store_name)
            .context("Cannot find store for given name")?;
        let path = store.path.clone();
        self.default_store
            .take_if(|value| value.eq_ignore_ascii_case(store_name));
        self.stores
            .retain(|store| !store.name.eq_ignore_ascii_case(store_name));
        self.store()?;
        Ok(path)
    }

    fn default_store_name(&self) -> Option<String> {
        var_os(environment_variables::PASEJO_DEFAULT_STORE).map_or_else(
            || self.default_store.clone(),
            |value| value.into_string().ok(),
        )
    }

    pub fn set_default_store(&mut self, store_name: &str) -> Result<()> {
        self.default_store = Some(store_name.to_owned());
        self.store()?;
        Ok(())
    }

    pub fn add_identity(
        &mut self,
        identity: Identity,
        store_name: Option<&String>,
        global: bool,
    ) -> Result<()> {
        if global {
            self.identities.push(identity);
            self.store()?;
        } else if let Some(store) = self.select_store_mut(store_name) {
            store.identities.push(identity);
            self.store()?;
        }
        Ok(())
    }

    pub fn remove_identity(
        &mut self,
        identity: &Identity,
        store_name: Option<&String>,
        global: bool,
    ) -> Result<()> {
        if global {
            self.identities.retain(|i| i.file != identity.file);
            self.store()?;
        } else if let Some(store) = self.select_store_mut(store_name) {
            store.identities.retain(|i| i.file != identity.file);
            self.store()?;
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

    pub fn all_identity_files(&self, store: &Store) -> Vec<String> {
        let mut identities = self.identities.clone();
        identities.extend(store.identities.clone());
        identities
            .iter()
            .map(|identity| identity.file.clone())
            .collect()
    }

    pub fn all_store_names(&self) -> Vec<String> {
        let mut names = vec![];
        for store in &self.stores {
            names.push(store.name.clone());
        }
        names
    }

    pub fn select_store(&self, store_name: Option<&String>) -> Option<&Store> {
        store_name
            .cloned()
            .or_else(|| self.default_store_name())
            .map_or_else(
                || self.stores.first(),
                |name| self.find_store(name.as_str()),
            )
    }

    pub fn select_store_mut(&mut self, store_name: Option<&String>) -> Option<&mut Store> {
        if let Some(name) = store_name.cloned().or_else(|| self.default_store_name()) {
            self.find_store_mut(name.as_str())
        } else {
            self.stores.first_mut()
        }
    }

    fn find_store(&self, store_name: &str) -> Option<&Store> {
        self.stores
            .iter()
            .find(|store| store.name.eq_ignore_ascii_case(store_name))
    }

    fn find_store_mut(&mut self, store_name: &str) -> Option<&mut Store> {
        self.stores
            .iter_mut()
            .find(|store| store.name.eq_ignore_ascii_case(store_name))
    }
}

impl Store {
    pub fn vcs(&self) -> Box<dyn VersionControlSystem> {
        self.vcs.select_implementation(PathBuf::from(&self.path))
    }

    #[must_use]
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        PathBuf::from(&self.path).join(&path)
    }

    pub fn resolve_secret_path(&self, secret_path: &String) -> PathBuf {
        self.resolve_path(file_system::append_file_extension(
            PathBuf::from(secret_path),
            constants::SECRET_FILE_EXTENSION,
        ))
    }

    pub fn resolve_recipients_path(&self, secret_path: &String) -> PathBuf {
        self.resolve_path(file_system::append_file_extension(
            PathBuf::from(secret_path),
            constants::RECIPIENTS_FILE_EXTENSION,
        ))
    }

    pub fn secret_at_path_exists(&self, path: &Path) -> bool {
        let absolute_path_to_secret_file = self.resolve_path(file_system::append_file_extension(
            path.to_path_buf(),
            constants::SECRET_FILE_EXTENSION,
        ));
        let absolute_path_to_secret_directory = self.resolve_path(path);
        let file_exists = absolute_path_to_secret_file.is_file();
        let directory_exists = absolute_path_to_secret_directory.is_dir();

        file_exists || directory_exists
    }

    pub fn find_nearest_existing_recipients(
        &self,
        secret_path: &String,
        inherit: bool,
    ) -> Result<PathBuf> {
        let mut recipients = vec![];

        // the secret itself might have a .recipients file
        let secret_recipients_file = self.resolve_path(file_system::append_file_extension(
            PathBuf::from(secret_path),
            constants::RECIPIENTS_FILE_EXTENSION,
        ));
        if secret_recipients_file.is_file() {
            recipients.push(secret_recipients_file);
        }

        // ... or any of its parent folders within its associated store
        for path in self.resolve_path(secret_path).ancestors() {
            if path.starts_with(&self.path) && recipients.len() < 2 {
                let recipients_file =
                    self.resolve_path(path.join(constants::RECIPIENTS_DOT_EXTENSION));
                if recipients_file.is_file() {
                    recipients.push(recipients_file);
                }
            } else {
                break;
            }
        }

        if inherit && !recipients.is_empty() {
            // remove nearest match to select the first parent afterward
            recipients.remove(0);
        }

        recipients
            .first()
            .cloned()
            .context("No recipients file found for the given secret. Make sure to call 'pasejo recipients add ...' or specify recipients directly with '--recipient'")
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::fixture::ChildPath;
    use assert_fs::prelude::*;

    use crate::adapters::vcs::VersionControlSystems;
    use crate::cli::constants;
    use crate::models::configuration::Store;

    #[test]
    fn nearest_recipient_for_secret() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, false, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "{secret_path}{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn nearest_recipient_for_parent() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, false, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "some/nested/folder/structure/{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn nearest_recipient_for_parent_of_parent() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, false, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "some/nested/folder/{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn nearest_recipient_in_root() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, false, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(constants::RECIPIENTS_DOT_EXTENSION);
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn inherit_recipient_for_secret() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        let error: &str = test_find_nearest_recipients(&temp, true, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "{secret_path}{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
        .unwrap_err()
        .downcast()?;
        assert_eq!(
            error,
            "No recipients file found for the given secret. Make sure to call 'pasejo recipients add ...' or specify recipients directly with '--recipient'"
        );
        Ok(())
    }

    #[test]
    fn inherit_recipient_for_parent() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, true, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "{secret_path}{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;
            let secret_recipients = temp.child(format!(
                "some/nested/folder/structure/{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn inherit_recipient_for_parent_of_parent() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, true, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "{secret_path}{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;
            let secret_recipients = temp.child(format!(
                "some/nested/folder/{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    #[test]
    fn inherit_recipient_in_root() -> anyhow::Result<()> {
        let temp = TempDir::new()?;
        test_find_nearest_recipients(&temp, true, || {
            let secret_path = String::from("some/nested/folder/structure/secret");
            temp.child(&secret_path).create_dir_all()?;
            let secret_recipients = temp.child(format!(
                "{secret_path}{}",
                constants::RECIPIENTS_DOT_EXTENSION
            ));
            secret_recipients.touch()?;
            let secret_recipients = temp.child(constants::RECIPIENTS_DOT_EXTENSION);
            secret_recipients.touch()?;

            Ok((secret_path, secret_recipients))
        })
    }

    fn test_find_nearest_recipients<T>(
        temp: &TempDir,
        inherit: bool,
        testcase: T,
    ) -> anyhow::Result<()>
    where
        T: FnOnce() -> anyhow::Result<(String, ChildPath)>,
    {
        let store = Store {
            path: temp.path().display().to_string(),
            name: String::from("test"),
            identities: vec![],
            vcs: VersionControlSystems::None,
        };

        let (secret_path, secret_recipients) = testcase()?;
        let recipient_path = store.find_nearest_existing_recipients(&secret_path, inherit)?;
        assert_eq!(secret_recipients.path(), recipient_path);
        Ok(())
    }
}
