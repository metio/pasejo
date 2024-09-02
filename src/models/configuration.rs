use crate::adapters::vcs::VersionControlSystems;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::path::{absolute, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All known stores the user has configured on their system
    pub stores: Vec<Store>,

    /// Global identities used for all stores
    pub identities: Vec<Identity>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Store {
    /// The local file system path of a store
    pub path: String,

    /// The short name for a store
    pub alias: String,

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
    fn config_path() -> PathBuf {
        let app_name = env!("CARGO_PKG_NAME");
        match std::env::var_os(app_name.to_owned().add("_config").to_uppercase()) {
            Some(path) => absolute(PathBuf::from(path)).expect("to resolve absolute path"),
            None => confy::get_configuration_file_path(app_name, "config")
                .expect("to load configuration"),
        }
    }

    pub fn load() -> Self {
        confy::load_path(Configuration::config_path()).expect("to load configuration")
    }

    fn store(&self) -> Result<()> {
        confy::store_path(Configuration::config_path(), self)?;
        Ok(())
    }

    pub fn add_store(
        &mut self,
        path: String,
        alias: String,
        vcs: VersionControlSystems,
    ) -> Result<()> {
        let store = self
            .stores
            .iter()
            .find(|&store| store.alias.eq_ignore_ascii_case(&alias));

        match store {
            Some(store) => Err(anyhow!("Store with alias '{}' already exists", store.alias)),
            None => {
                self.stores.push(Store {
                    path,
                    alias,
                    vcs,
                    identities: vec![],
                });
                self.store()?;
                Ok(())
            }
        }
    }

    pub fn select_store(&self, alias: Option<String>) -> &Store {
        match alias {
            Some(alias) => self
                .stores
                .iter()
                .find(|&store| store.alias.eq_ignore_ascii_case(&alias))
                .expect("Cannot find store for given alias"),
            None => self.stores.first().expect("No store is configured"),
        }
    }

    pub fn add_identity(&mut self, identity: Identity, alias: Option<String>) -> Result<()> {
        match alias {
            Some(alias) => {
                let store = self
                    .stores
                    .iter_mut()
                    .find(|store| store.alias.eq_ignore_ascii_case(&alias))
                    .expect("Cannot find store for given alias");
                store.identities.push(identity);
            }
            None => self.identities.push(identity),
        }
        self.store()?;
        Ok(())
    }

    pub fn remove_identity(&mut self, identity: Identity, alias: Option<String>) -> Result<()> {
        match alias {
            Some(alias) => {
                let store = self
                    .stores
                    .iter_mut()
                    .find(|store| store.alias.eq_ignore_ascii_case(&alias))
                    .expect("Cannot find store for given alias");
                store.identities.retain(|i| i.file != identity.file);
            }
            None => self.identities.retain(|i| i.file != identity.file),
        }
        self.store()?;
        Ok(())
    }

    pub fn all_identities(&mut self, alias: Option<String>) -> Result<Vec<Identity>> {
        let mut identities = self.identities.clone();
        if let Some(alias) = alias {
            identities.extend(
                self.stores
                    .iter()
                    .find(|store| store.alias.eq_ignore_ascii_case(&alias))
                    .map_or_else(|| vec![], |store| store.identities.clone()),
            );
        }
        Ok(identities)
    }
}

impl Store {
    pub fn paths_for(&self, path: &Option<PathBuf>, suffix: &str) -> (PathBuf, PathBuf) {
        let suffix = PathBuf::from(suffix);
        let relative_path = path
            .as_ref()
            .map_or_else(|| suffix.clone(), |p| p.join(&suffix));
        (
            PathBuf::from(&self.path).join(&relative_path),
            relative_path,
        )
    }
}
