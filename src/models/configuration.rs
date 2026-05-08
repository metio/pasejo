// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{atomic_write, constants, environment_variables};
use crate::models::password_store::PasswordStore;
use crate::{identities, recipients, secrets};
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::env::var_os;
use std::fs;
use std::path::{Path, PathBuf, absolute};
use std::sync::OnceLock;
use toml::Table;

static CACHED_CONFIGURATION: OnceLock<Configuration> = OnceLock::new();

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Configuration {
    /// All registered stores the user has configured on their system
    pub stores: Vec<StoreRegistration>,

    /// Global identities used for all stores
    pub identities: Vec<Identity>,

    /// The default store to use when no store name was specified
    pub default_store: Option<String>,

    /// Toggle whether missing identity files will be ignored. Defaults to
    /// `true` so users with multiple hardware tokens (e.g. Yubikeys) can
    /// plug in whichever backup is available without listing every key
    /// explicitly. age still requires a matching recipient to decrypt, so
    /// a missing identity surfaces as "cannot decrypt" rather than as a
    /// silent fallback to the wrong key.
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
        Self::migrate_legacy_config_path(&config_path)?;
        if let Ok(config) = Self::read_from_path(&config_path) {
            Ok(config)
        } else {
            Self::migrate_configuration(&config_path).context("Could not migrate configuration")?;
            Self::read_from_path(&config_path).context("Could not load configuration")
        }
    }

    /// Process-wide cached load. The first caller (clap value parser, completer,
    /// or `main`) reads the file from disk; subsequent callers within the same
    /// process get the same `Configuration` reference back. Errors are not
    /// cached — if the first call fails, the next call retries.
    pub fn cached() -> Result<&'static Self> {
        if let Some(cached) = CACHED_CONFIGURATION.get() {
            return Ok(cached);
        }
        let loaded = Self::load_configuration()?;
        Ok(CACHED_CONFIGURATION.get_or_init(|| loaded))
    }

    /// One-shot relocation of the configuration file from the path used by
    /// the previous `confy`-based implementation (`ProjectDirs::from("rs",
    /// "pasejo", "pasejo")`) to the current path. Only relevant on macOS
    /// and Windows; on Linux both paths resolve to the same directory.
    /// Remove this function and its caller once users have migrated.
    fn migrate_legacy_config_path(new_path: &Path) -> Result<()> {
        if var_os(environment_variables::PASEJO_CONFIG).is_some() || new_path.exists() {
            return Ok(());
        }
        let Some(legacy_dirs) = ProjectDirs::from(
            "rs",
            constants::APPLICATION_NAME,
            constants::APPLICATION_NAME,
        ) else {
            return Ok(());
        };
        let legacy_path = legacy_dirs.config_dir().join("config.toml");
        if legacy_path == new_path || !legacy_path.exists() {
            return Ok(());
        }
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent).context("Could not create configuration directory")?;
        }
        fs::rename(&legacy_path, new_path)
            .context("Could not migrate legacy configuration file")?;
        Ok(())
    }

    fn read_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    fn migrate_configuration(config_path: &Path) -> Result<()> {
        let config_content = fs::read_to_string(config_path)?;
        let mut migrated_config = config_content.parse::<Table>()?;
        migrate_table(&mut migrated_config);
        let serialized = toml::to_string_pretty(&migrated_config)
            .context("Could not serialize migrated configuration")?;
        atomic_write::write(config_path, serialized.as_bytes())
            .context("Could not store configuration")?;
        Ok(())
    }

    pub fn save_configuration(&self) -> Result<()> {
        let path = Self::config_path()?;
        let serialized =
            toml::to_string_pretty(self).context("Could not serialize configuration")?;
        atomic_write::write(&path, serialized.as_bytes()).context("Could not store configuration")
    }

    fn config_path() -> Result<PathBuf> {
        if let Some(path) = var_os(environment_variables::PASEJO_CONFIG) {
            return absolute(PathBuf::from(path))
                .context("Could not resolve absolute path to configuration");
        }
        let project_dirs = ProjectDirs::from("wtf", "metio", constants::APPLICATION_NAME)
            .context("Could not determine configuration path")?;
        Ok(project_dirs.config_dir().join("config.toml"))
    }

    pub fn add_store(&mut self, store_root_path: &str, store_name: &str) -> Result<()> {
        let canonical_path = absolute(PathBuf::from(store_root_path))
            .context("Could not resolve absolute store path")?;
        let registration = StoreRegistration {
            path: canonical_path,
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
        if identity_files.is_empty() {
            anyhow::bail!(
                "No identity files to decrypt. Add at least one identity to complete store initialization."
            );
        }
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

fn default_hook_commands(synchronizer: &str) -> Option<(Vec<String>, Vec<String>)> {
    match synchronizer {
        "Git" => Some((
            vec![String::from("git pull")],
            vec![
                String::from("git add %p"),
                String::from("git commit --message 'pasejo commit'"),
                String::from("git push"),
            ],
        )),
        "Mercurial" => Some((
            vec![String::from("hg pull")],
            vec![
                String::from("hg add %p"),
                String::from("hg commit --message 'pasejo commit'"),
                String::from("hg push"),
            ],
        )),
        "Pijul" => Some((
            vec![String::from("pijul pull")],
            vec![
                String::from("pijul add %p"),
                String::from("pijul record --message 'pasejo commit'"),
                String::from("pijul push"),
            ],
        )),
        _ => None,
    }
}

fn migrate_table(table: &mut Table) {
    if !table.contains_key("pull_commands") {
        table.insert(
            "pull_commands".to_string(),
            toml::Value::from(Vec::<String>::new()),
        );
    }
    if !table.contains_key("push_commands") {
        table.insert(
            "push_commands".to_string(),
            toml::Value::from(Vec::<String>::new()),
        );
    }
    if let Some(stores_value) = table.get_mut("stores")
        && let Some(stores) = stores_value.as_array_mut()
    {
        for store in stores {
            if let Some(store_table) = store.as_table_mut() {
                migrate_store_table(store_table);
            }
        }
    }
}

fn migrate_store_table(table: &mut Table) {
    let has_pull_commands = table.contains_key("pull_commands");
    let has_push_commands = table.contains_key("push_commands");

    let (pull_commands, push_commands) = match table.remove("synchronizer") {
        None => (Vec::new(), Vec::new()),
        Some(synchronizer) => match synchronizer.as_str() {
            Some(name) => default_hook_commands(name).unwrap_or_default(),
            // Non-string synchronizer: drop the key but leave commands untouched.
            None => return,
        },
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    fn registration(name: &str, path: &str) -> StoreRegistration {
        StoreRegistration {
            path: PathBuf::from(path),
            name: name.to_string(),
            identities: vec![],
            pull_commands: vec![],
            push_commands: vec![],
        }
    }

    fn configuration_with_stores(stores: Vec<StoreRegistration>) -> Configuration {
        Configuration {
            stores,
            ..Configuration::default()
        }
    }

    #[test]
    fn all_store_names_returns_names_in_registration_order() {
        let cfg = configuration_with_stores(vec![
            registration("alpha", "/tmp/alpha"),
            registration("beta", "/tmp/beta"),
        ]);
        assert_eq!(cfg.all_store_names(), vec!["alpha", "beta"]);
    }

    #[test]
    fn all_store_names_is_empty_for_default_configuration() {
        assert!(Configuration::default().all_store_names().is_empty());
    }

    #[test]
    fn find_store_is_case_insensitive() {
        let cfg = configuration_with_stores(vec![registration("Alpha", "/tmp/alpha")]);
        assert!(cfg.find_store("alpha").is_some());
        assert!(cfg.find_store("ALPHA").is_some());
        assert!(cfg.find_store("Alpha").is_some());
    }

    #[test]
    fn find_store_returns_none_for_unknown_name() {
        let cfg = configuration_with_stores(vec![registration("alpha", "/tmp/alpha")]);
        assert!(cfg.find_store("missing").is_none());
    }

    #[test]
    fn all_identity_files_combines_global_and_store_and_dedups() {
        let mut cfg = configuration_with_stores(vec![StoreRegistration {
            path: PathBuf::from("/tmp/alpha"),
            name: String::from("alpha"),
            identities: vec![
                Identity {
                    file: PathBuf::from("/keys/store"),
                },
                Identity {
                    file: PathBuf::from("/keys/shared"),
                },
            ],
            pull_commands: vec![],
            push_commands: vec![],
        }]);
        cfg.identities = vec![
            Identity {
                file: PathBuf::from("/keys/global"),
            },
            Identity {
                file: PathBuf::from("/keys/shared"),
            },
        ];

        let files = cfg.all_identity_files(cfg.stores.first().unwrap());
        assert_eq!(
            files,
            vec![
                PathBuf::from("/keys/global"),
                PathBuf::from("/keys/shared"),
                PathBuf::from("/keys/store"),
            ]
        );
    }

    #[test]
    fn all_identity_files_returns_empty_when_none_configured() {
        let cfg = configuration_with_stores(vec![registration("alpha", "/tmp/alpha")]);
        let files = cfg.all_identity_files(cfg.stores.first().unwrap());
        assert!(files.is_empty());
    }

    #[test]
    fn has_identity_global_finds_match() {
        let mut cfg = Configuration {
            identities: vec![Identity {
                file: PathBuf::from("/keys/k1"),
            }],
            ..Configuration::default()
        };
        let identity = Identity {
            file: PathBuf::from("/keys/k1"),
        };
        assert!(cfg.has_identity(&identity, None, true));
    }

    #[test]
    fn has_identity_global_returns_false_when_missing() {
        let mut cfg = Configuration::default();
        let identity = Identity {
            file: PathBuf::from("/keys/k1"),
        };
        assert!(!cfg.has_identity(&identity, None, true));
    }

    #[test]
    fn has_identity_per_store_uses_store_identities() {
        let mut cfg = configuration_with_stores(vec![StoreRegistration {
            path: PathBuf::from("/tmp/alpha"),
            name: String::from("alpha"),
            identities: vec![Identity {
                file: PathBuf::from("/keys/store"),
            }],
            pull_commands: vec![],
            push_commands: vec![],
        }]);
        let identity = Identity {
            file: PathBuf::from("/keys/store"),
        };
        let store_name = String::from("alpha");
        assert!(cfg.has_identity(&identity, Some(&store_name), false));

        let other_identity = Identity {
            file: PathBuf::from("/keys/elsewhere"),
        };
        assert!(!cfg.has_identity(&other_identity, Some(&store_name), false));
    }

    #[test]
    fn select_store_with_explicit_name_returns_named_store() {
        let cfg = configuration_with_stores(vec![
            registration("alpha", "/tmp/alpha"),
            registration("beta", "/tmp/beta"),
        ]);
        let name = String::from("beta");
        let store = cfg.select_store(Some(&name)).unwrap();
        assert_eq!(store.name, "beta");
    }

    #[test]
    fn select_store_falls_back_to_first_when_no_default() {
        let cfg = configuration_with_stores(vec![
            registration("alpha", "/tmp/alpha"),
            registration("beta", "/tmp/beta"),
        ]);
        // No name given and no default; first store is the fallback.
        // Note: this test must not run with PASEJO_DEFAULT_STORE set in the
        // environment, otherwise the env var will steer the lookup. Tests in
        // this crate run in parallel so we don't rely on env state here — we
        // only assert the structural behavior on default configurations.
        if std::env::var_os(crate::cli::environment_variables::PASEJO_DEFAULT_STORE).is_none() {
            let store = cfg.select_store(None).unwrap();
            assert_eq!(store.name, "alpha");
        }
    }

    #[test]
    fn select_store_returns_none_for_unknown_name() {
        let cfg = configuration_with_stores(vec![registration("alpha", "/tmp/alpha")]);
        let name = String::from("missing");
        assert!(cfg.select_store(Some(&name)).is_none());
    }

    #[test]
    fn store_registration_path_returns_inner_path() {
        let reg = registration("alpha", "/tmp/alpha");
        assert_eq!(reg.path(), Path::new("/tmp/alpha"));
    }

    #[test]
    fn add_store_canonicalizes_relative_path() {
        // We can't safely call save_configuration here (it touches the user
        // config), so we exercise the canonicalization step directly.
        let relative = "./relative-store.age";
        let resolved = absolute(PathBuf::from(relative)).unwrap();
        assert!(
            resolved.is_absolute(),
            "expected an absolute path, got {}",
            resolved.display()
        );
    }

    fn parse_table(toml_text: &str) -> Table {
        toml_text.parse::<Table>().unwrap()
    }

    fn string_array(value: &toml::Value) -> Vec<String> {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn default_hook_commands_for_git() {
        let (pull, push) = default_hook_commands("Git").unwrap();
        assert_eq!(pull, vec!["git pull"]);
        assert_eq!(
            push,
            vec![
                "git add %p",
                "git commit --message 'pasejo commit'",
                "git push",
            ]
        );
    }

    #[test]
    fn default_hook_commands_for_mercurial() {
        let (pull, push) = default_hook_commands("Mercurial").unwrap();
        assert_eq!(pull, vec!["hg pull"]);
        assert_eq!(
            push,
            vec![
                "hg add %p",
                "hg commit --message 'pasejo commit'",
                "hg push",
            ]
        );
    }

    #[test]
    fn default_hook_commands_for_pijul() {
        let (pull, push) = default_hook_commands("Pijul").unwrap();
        assert_eq!(pull, vec!["pijul pull"]);
        assert_eq!(
            push,
            vec![
                "pijul add %p",
                "pijul record --message 'pasejo commit'",
                "pijul push",
            ]
        );
    }

    #[test]
    fn default_hook_commands_for_unknown_returns_none() {
        assert!(default_hook_commands("svn").is_none());
        assert!(default_hook_commands("").is_none());
    }

    #[test]
    fn migrate_table_inserts_top_level_command_arrays_when_missing() {
        let mut table = parse_table("");
        migrate_table(&mut table);
        assert_eq!(string_array(&table["pull_commands"]), Vec::<String>::new());
        assert_eq!(string_array(&table["push_commands"]), Vec::<String>::new());
    }

    #[test]
    fn migrate_table_preserves_existing_top_level_command_arrays() {
        let mut table = parse_table(
            r#"
            pull_commands = ["custom pull"]
            push_commands = ["custom push"]
            "#,
        );
        migrate_table(&mut table);
        assert_eq!(string_array(&table["pull_commands"]), vec!["custom pull"]);
        assert_eq!(string_array(&table["push_commands"]), vec!["custom push"]);
    }

    #[test]
    fn migrate_table_replaces_git_synchronizer_with_default_commands() {
        let mut table = parse_table(
            r#"
            [[stores]]
            path = "/tmp/store"
            name = "primary"
            synchronizer = "Git"
            "#,
        );
        migrate_table(&mut table);
        let store = table["stores"].as_array().unwrap()[0].as_table().unwrap();
        assert!(!store.contains_key("synchronizer"));
        assert_eq!(string_array(&store["pull_commands"]), vec!["git pull"]);
        assert_eq!(
            string_array(&store["push_commands"]),
            vec![
                "git add %p",
                "git commit --message 'pasejo commit'",
                "git push",
            ]
        );
    }

    #[test]
    fn migrate_table_drops_unknown_synchronizer_and_inserts_empty_commands() {
        let mut table = parse_table(
            r#"
            [[stores]]
            path = "/tmp/store"
            name = "primary"
            synchronizer = "svn"
            "#,
        );
        migrate_table(&mut table);
        let store = table["stores"].as_array().unwrap()[0].as_table().unwrap();
        assert!(!store.contains_key("synchronizer"));
        assert_eq!(string_array(&store["pull_commands"]), Vec::<String>::new());
        assert_eq!(string_array(&store["push_commands"]), Vec::<String>::new());
    }

    #[test]
    fn migrate_table_preserves_existing_per_store_commands_over_synchronizer_defaults() {
        let mut table = parse_table(
            r#"
            [[stores]]
            path = "/tmp/store"
            name = "primary"
            synchronizer = "Git"
            pull_commands = ["already configured"]
            push_commands = ["also configured"]
            "#,
        );
        migrate_table(&mut table);
        let store = table["stores"].as_array().unwrap()[0].as_table().unwrap();
        assert!(!store.contains_key("synchronizer"));
        assert_eq!(
            string_array(&store["pull_commands"]),
            vec!["already configured"]
        );
        assert_eq!(
            string_array(&store["push_commands"]),
            vec!["also configured"]
        );
    }

    #[test]
    fn migrate_table_inserts_empty_commands_when_no_synchronizer() {
        let mut table = parse_table(
            r#"
            [[stores]]
            path = "/tmp/store"
            name = "primary"
            "#,
        );
        migrate_table(&mut table);
        let store = table["stores"].as_array().unwrap()[0].as_table().unwrap();
        assert_eq!(string_array(&store["pull_commands"]), Vec::<String>::new());
        assert_eq!(string_array(&store["push_commands"]), Vec::<String>::new());
    }

    #[test]
    fn migrate_table_skips_command_insertion_when_synchronizer_is_not_a_string() {
        let mut table = parse_table(
            r#"
            [[stores]]
            path = "/tmp/store"
            name = "primary"
            synchronizer = 42
            "#,
        );
        migrate_table(&mut table);
        let store = table["stores"].as_array().unwrap()[0].as_table().unwrap();
        // The non-string synchronizer is still removed, but no command keys are inserted.
        assert!(!store.contains_key("synchronizer"));
        assert!(!store.contains_key("pull_commands"));
        assert!(!store.contains_key("push_commands"));
    }

    #[test]
    fn migrate_table_handles_missing_stores_array() {
        let mut table = parse_table(r#"default_store = "primary""#);
        migrate_table(&mut table);
        // Top-level command arrays still get filled in.
        assert!(table.contains_key("pull_commands"));
        assert!(table.contains_key("push_commands"));
        assert!(!table.contains_key("stores"));
    }
}
