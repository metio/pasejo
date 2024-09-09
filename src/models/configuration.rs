use std::env::var_os;
use std::ops::Add;
use std::path::{absolute, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::adapters::file_system;
use crate::adapters::vcs::VersionControlSystems;
use crate::cli::constants;

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
    fn config_path() -> Result<PathBuf> {
        let app_name = env!("CARGO_PKG_NAME");
        var_os(app_name.to_owned().add("_config").to_uppercase()).map_or_else(
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

    pub fn select_store(&self, store_name: &Option<String>) -> Option<&Store> {
        store_name.as_ref().map_or_else(
            || {
                self.default_store_name().map_or_else(
                    || self.stores.first(),
                    |default| self.find_store(default.as_str()),
                )
            },
            |name| self.find_store(name.as_str()),
        )
    }

    fn default_store_name(&self) -> Option<String> {
        let app_name = env!("CARGO_PKG_NAME");
        var_os(
            app_name
                .to_owned()
                .add("_default_store_name")
                .to_uppercase(),
        )
        .map_or_else(
            || self.default_store.clone(),
            |value| value.into_string().ok(),
        )
    }

    pub fn set_default_store(&mut self, store_name: &str) -> Result<()> {
        self.default_store = Some(store_name.to_owned());
        self.store()?;
        Ok(())
    }

    pub fn add_identity(&mut self, identity: Identity, store_name: Option<String>) -> Result<()> {
        match store_name {
            Some(name) => {
                let store = self
                    .find_store_mut(name.as_str())
                    .context("Cannot find store for given name")?;
                store.identities.push(identity);
            }
            None => match self.default_store_name() {
                Some(default) => {
                    let store = self
                        .find_store_mut(default.as_str())
                        .context("Cannot find store using default name")?;
                    store.identities.push(identity);
                }
                None => self.identities.push(identity),
            },
        }
        self.store()?;
        Ok(())
    }

    pub fn remove_identity(
        &mut self,
        identity: &Identity,
        store_name: Option<String>,
    ) -> Result<()> {
        match store_name {
            Some(name) => {
                let store = self
                    .find_store_mut(name.as_str())
                    .context("Cannot find store for given name")?;
                store.identities.retain(|i| i.file != identity.file);
            }
            None => match self.default_store_name() {
                Some(default) => {
                    let store = self
                        .find_store_mut(default.as_str())
                        .context("Cannot find store using the default name")?;
                    store.identities.retain(|i| i.file != identity.file);
                }
                None => self.identities.retain(|i| i.file != identity.file),
            },
        }
        self.store()?;
        Ok(())
    }

    pub fn all_identities(&self, store_name: &Option<String>) -> Vec<Identity> {
        let mut identities = self.identities.clone();
        if let Some(name) = store_name {
            identities.extend(
                self.find_store(name.as_str())
                    .map_or_else(Vec::new, |store| store.identities.clone()),
            );
        }
        identities
    }

    pub fn all_store_names(&self) -> Vec<String> {
        let mut names = vec![];
        for store in &self.stores {
            names.push(store.name.clone());
        }
        names
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

    pub fn find_nearest_recipients(&self, secret_path: &String, inherit: bool) -> Result<PathBuf> {
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
            if path.starts_with(&self.path) {
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
