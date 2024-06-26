use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::adapters::vcs::VersionControlSystems;

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
    pub fn load() -> Self {
        confy::load(env!("CARGO_PKG_NAME"), "config").expect("to load configuration")
    }

    pub fn add_store(
        &mut self,
        path: String,
        alias: String,
        vcs: VersionControlSystems,
    ) -> Result<()> {
        self.stores.push(Store {
            path,
            alias,
            vcs,
            identities: vec![],
        });
        confy::store(env!("CARGO_PKG_NAME"), "config", self)?;
        Ok(())
    }

    pub fn select_store(&self, alias: Option<String>) -> &Store {
        match alias {
            Some(alias) => self
                .stores
                .iter()
                .find(|&store| store.alias.eq(&alias))
                .expect("Cannot find store for given alias"),
            None => self.stores.first().expect("No store is configured"),
        }
    }

    pub fn add_identity(&mut self, file: String, alias: Option<String>) -> Result<()> {
        match alias {
            Some(alias) => {
                let store = self
                    .stores
                    .iter_mut()
                    .find(|store| store.alias.eq(&alias))
                    .expect("Cannot find store for given alias");
                store.identities.push(Identity { file });
            }
            None => self.identities.push(Identity { file }),
        }
        confy::store(env!("CARGO_PKG_NAME"), "config", self)?;
        Ok(())
    }
}
