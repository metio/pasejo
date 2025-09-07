// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};
use std::{fs, path};

use crate::cli::logs;
use crate::hooks::executor::HookExecutor;
use crate::models::configuration::Configuration;
use crate::{one_time_passwords, recipients, secrets};
use anyhow::{Context, Result};

pub fn add(
    mut configuration: Configuration,
    store_path: &Path,
    store_name: &str,
    default: bool,
) -> Result<()> {
    if configuration.find_store(store_name).is_some() {
        anyhow::bail!("Store name already exists. Please use a different name.");
    }

    let absolute_path = path::absolute(store_path)?;

    if absolute_path.is_dir() {
        anyhow::bail!("Cannot use directory as store path. Please use a file path.");
    }

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
        configuration.add_store(&absolute_path.display().to_string(), store_name)?;
        logs::store_add_success(store_name, &absolute_path.display().to_string());
        if default {
            set_default(configuration, store_name)?;
        }
        Ok(())
    } else {
        anyhow::bail!("Cannot create store path. Please check the path and try again.");
    }
}

pub fn remove(
    mut configuration: Configuration,
    store_name: Option<&String>,
    remove_data: bool,
) -> Result<()> {
    let (name, path) = if let Some(registration) = configuration.select_store(store_name) {
        (&registration.name.clone(), &registration.path.clone())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    };

    configuration.remove_store(name)?;
    if remove_data {
        let path_to_store = Path::new(path);
        if path_to_store.exists() {
            fs::remove_dir(path_to_store)?;
        }
    }
    logs::store_remove_success(name);
    Ok(())
}

pub fn set_default(mut configuration: Configuration, store_name: &str) -> Result<()> {
    configuration.set_default_store(store_name)?;
    logs::store_set_default(store_name);
    Ok(())
}

pub fn decrypt(
    configuration: &Configuration,
    store_name: Option<&String>,
    offline: bool,
    store_path: Option<&PathBuf>,
) -> Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        if store_path.is_none() {
            let hooks = HookExecutor {
                configuration,
                registration,
                offline,
                force: false,
            };

            hooks.execute_pull_commands()?;
        }

        let store = if let Some(path) = store_path {
            configuration
                .decrypt_store_from_path(registration, path)
                .context("Cannot decrypt store")?
        } else {
            configuration
                .decrypt_store(registration)
                .context("Cannot decrypt store")?
        };

        let content = toml::to_string(&store)?;
        println!("{content}");

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn list(configuration: &Configuration) {
    for store in &configuration.stores {
        let text = configuration
            .default_store
            .clone()
            .filter(|default| default == &store.name)
            .map_or_else(
                || format!("{}: {}", store.name, store.path),
                |default| format!("{}: {} (default)", default, store.path),
            );

        println!("{text}");
    }
}

pub fn merge(
    configuration: &Configuration,
    store_name: Option<&String>,
    common_ancestor: &Path,
    current_version: &Path,
    other_version: &Path,
) -> Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let common_ancestor_store = configuration
            .decrypt_store_from_path(registration, common_ancestor)
            .context("Cannot decrypt common ancestor store")?;
        let mut current_version_store = configuration
            .decrypt_store_from_path(registration, current_version)
            .context("Cannot decrypt current version store")?;
        let other_version_store = configuration
            .decrypt_store_from_path(registration, other_version)
            .context("Cannot decrypt other version store")?;

        let mut errors = vec![];

        match recipients::merge_recipients(
            &common_ancestor_store.recipients,
            &current_version_store.recipients,
            &other_version_store.recipients,
        ) {
            Ok(merged_recipients) => {
                current_version_store.recipients = merged_recipients;
            }
            Err(error) => {
                errors.push(error);
            }
        }

        match secrets::merge_secrets(
            &common_ancestor_store.secrets,
            &current_version_store.secrets,
            &other_version_store.secrets,
        ) {
            Ok(merged_secrets) => {
                current_version_store.secrets = merged_secrets;
            }
            Err(error) => {
                errors.push(error);
            }
        }

        match one_time_passwords::merge_one_time_passwords(
            &common_ancestor_store.otp,
            &current_version_store.otp,
            &other_version_store.otp,
        ) {
            Ok(merged_one_time_passwords) => {
                current_version_store.otp = merged_one_time_passwords;
            }
            Err(error) => {
                errors.push(error);
            }
        }

        if errors.is_empty() {
            Configuration::encrypt_store_to_path(&current_version_store, current_version)
        } else {
            let error_messages: Vec<String> =
                errors.into_iter().map(|error| error.to_string()).collect();
            anyhow::bail!(error_messages.join("\n       "))
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn exec(
    configuration: &Configuration,
    store_name: Option<&String>,
    command: &[String],
) -> Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let store_path = registration.path();
        if let Some(parent) = store_path.parent() {
            if let Some(split) = command.split_first() {
                duct::cmd(split.0, split.1)
                    .env("PASEJO_EXEC_STORE_PATH", store_path)
                    .env("PASEJO_EXEC_STORE_PARENT", parent)
                    .env("PASEJO_EXEC_COMMAND", split.0)
                    .dir(parent)
                    .run()
                    .with_context(|| format!("Failed to run command {}", split.0))?;
            }
        } else {
            anyhow::bail!(
                "Cannot get parent directory of store path. Please check the path and try again."
            )
        }

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
