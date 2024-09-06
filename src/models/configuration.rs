use crate::adapters::file_system;
use crate::adapters::vcs::VersionControlSystems;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::path::{absolute, Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All known stores the user has configured on their system
    pub stores: Vec<Store>,

    /// Global identities used for all stores
    pub identities: Vec<Identity>,

    /// The default store to use when no alias was specified
    pub default_store: Option<String>,
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
    fn config_path() -> Result<PathBuf> {
        let app_name = env!("CARGO_PKG_NAME");
        std::env::var_os(app_name.to_owned().add("_config").to_uppercase()).map_or_else(
            || {
                confy::get_configuration_file_path(app_name, "config")
                    .context("to load configuration")
            },
            |path| absolute(PathBuf::from(path)).context("to resolve absolute path"),
        )
    }

    pub fn load() -> Result<Self> {
        confy::load_path(Self::config_path()?).context("to load configuration")
    }

    fn store(&self) -> Result<()> {
        confy::store_path(Self::config_path()?, self)?;
        Ok(())
    }

    pub fn add_store(
        &mut self,
        path: String,
        alias: &str,
        vcs: VersionControlSystems,
    ) -> Result<()> {
        if let Some(store) = self.find_store(alias) {
            Err(anyhow!("Store with alias '{}' already exists", store.alias))
        } else {
            self.stores.push(Store {
                path,
                alias: alias.to_owned(),
                vcs,
                identities: vec![],
            });
            self.store()?;
            Ok(())
        }
    }

    pub fn remove_store(&mut self, alias: &str) -> Result<String> {
        let store = self
            .find_store(alias)
            .context("Cannot find store for given alias")?;
        let path = store.path.clone();
        self.default_store
            .take_if(|value| value.eq_ignore_ascii_case(alias));
        self.stores
            .retain(|store| !store.alias.eq_ignore_ascii_case(alias));
        self.store()?;
        Ok(path)
    }

    pub fn select_store(&self, alias: &Option<String>) -> Option<&Store> {
        alias.as_ref().map_or_else(
            || {
                self.default_store.as_ref().map_or_else(
                    || self.stores.first(),
                    |default| self.find_store(default.as_str()),
                )
            },
            |alias| self.find_store(alias.as_str()),
        )
    }

    pub fn set_default_store(&mut self, alias: &str) -> Result<()> {
        self.default_store = Some(alias.to_owned());
        self.store()?;
        Ok(())
    }

    pub fn add_identity(&mut self, identity: Identity, alias: Option<String>) -> Result<()> {
        match alias {
            Some(alias) => {
                let store = self
                    .find_store_mut(alias.as_str())
                    .context("Cannot find store for given alias")?;
                store.identities.push(identity);
            }
            None => match &self.default_store {
                Some(default) => {
                    let store = self
                        .find_store_mut(default.clone().as_str())
                        .context("Cannot find store using default alias")?;
                    store.identities.push(identity);
                }
                None => self.identities.push(identity),
            },
        }
        self.store()?;
        Ok(())
    }

    pub fn remove_identity(&mut self, identity: &Identity, alias: Option<String>) -> Result<()> {
        match alias {
            Some(alias) => {
                let store = self
                    .find_store_mut(alias.as_str())
                    .context("Cannot find store for given alias")?;
                store.identities.retain(|i| i.file != identity.file);
            }
            None => match &self.default_store {
                Some(default) => {
                    let store = self
                        .find_store_mut(default.clone().as_str())
                        .context("Cannot find store using default alias")?;
                    store.identities.retain(|i| i.file != identity.file);
                }
                None => self.identities.retain(|i| i.file != identity.file),
            },
        }
        self.store()?;
        Ok(())
    }

    pub fn all_identities(&self, alias: &Option<String>) -> Vec<Identity> {
        let mut identities = self.identities.clone();
        if let Some(alias) = alias {
            identities.extend(
                self.find_store(alias.as_str())
                    .map_or_else(Vec::new, |store| store.identities.clone()),
            );
        }
        identities
    }

    pub fn all_store_aliases(&self) -> Vec<String> {
        let mut aliases = vec![];
        for store in &self.stores {
            aliases.push(store.alias.clone());
        }
        aliases
    }

    fn find_store(&self, alias: &str) -> Option<&Store> {
        self.stores
            .iter()
            .find(|store| store.alias.eq_ignore_ascii_case(alias))
    }

    fn find_store_mut(&mut self, alias: &str) -> Option<&mut Store> {
        self.stores
            .iter_mut()
            .find(|store| store.alias.eq_ignore_ascii_case(alias))
    }
}

impl Store {
    #[must_use]
    pub fn resolve_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        PathBuf::from(&self.path).join(&path)
    }

    pub fn find_nearest_recipients(&self, secret_path: &Path, inherit: bool) -> Result<PathBuf> {
        let mut recipients = vec![];
        let recipients_extension = Path::new(".recipients");
        for path in secret_path.ancestors() {
            let recipients_file = self.resolve_path(path.join(recipients_extension));
            if file_system::file_exists(&recipients_file)? {
                recipients.push(recipients_file);
            }
        }
        let root_recipients_file = self.resolve_path(recipients_extension);
        if file_system::file_exists(&root_recipients_file)? {
            recipients.push(root_recipients_file);
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
