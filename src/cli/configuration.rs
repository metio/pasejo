use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::vcs::api::VCSTypes;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    pub(crate) stores: Vec<Store>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Store {
    pub(crate) path: String,
    pub(crate) alias: String,
    pub(crate) vcs: VCSTypes,
}

impl Configuration {
    pub fn load() -> Self {
        confy::load(env!("CARGO_PKG_NAME"), "config").expect("to load configuration")
    }

    pub(crate) fn add_store(&mut self, path: String, alias: String, vcs: VCSTypes) -> Result<()> {
        self.stores.push(Store { path, alias, vcs });
        confy::store(env!("CARGO_PKG_NAME"), "config", self)?;
        Ok(())
    }

    pub(crate) fn select_store(&self, alias: Option<String>) -> &Store {
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
