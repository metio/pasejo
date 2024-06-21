use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::adapters::vcs::VersionControlSystems;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    pub stores: Vec<Store>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Store {
    pub path: String,
    pub alias: String,
    pub vcs: VersionControlSystems,
}

impl Configuration {
    pub fn load() -> Self {
        confy::load(env!("CARGO_PKG_NAME"), "config").expect("to load configuration")
    }

    pub fn add_store(&mut self, path: String, alias: String, vcs: VersionControlSystems) -> Result<()> {
        self.stores.push(Store { path, alias, vcs });
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
}
