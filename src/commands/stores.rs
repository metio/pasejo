// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::Path;
use std::{fs, path};

use crate::cli::logs;
use crate::models::configuration::Configuration;
use crate::synchronizers::synchronizer::Synchronizers;
use anyhow::{Context, Result};

pub fn add(
    mut configuration: Configuration,
    store_path: &Path,
    store_name: &str,
    synchronizer: &Synchronizers,
    default: bool,
    offline: bool,
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
        configuration.add_store(
            &absolute_path.display().to_string(),
            store_name,
            synchronizer.clone(),
        )?;
        if !offline {
            let synchronizer = synchronizer.select_implementation(&absolute_path);
            logs::store_sync_push(store_name);
            synchronizer.push()?;
        }
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
) -> Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        if !offline {
            let store_path = Path::new(&registration.path);
            let synchronizer = registration.synchronizer.select_implementation(store_path);
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

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

pub fn set_synchronizer(
    mut configuration: Configuration,
    store_name: Option<&String>,
    synchronizer: &Synchronizers,
) -> Result<()> {
    if let Some(registration) = configuration.select_store_mut(store_name) {
        let name = registration.name.clone();
        registration.synchronizer = synchronizer.clone();
        configuration.save_configuration()?;
        logs::store_set_synchronizer(&name, synchronizer);

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn sync(
    configuration: &Configuration,
    store_name: Option<&String>,
    pull: Option<bool>,
    push: Option<bool>,
) -> Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);
        if pull.unwrap_or(false) {
            logs::store_sync_pull(&registration.name);
            synchronizer.pull()?;
        }
        if push.unwrap_or(false) {
            logs::store_sync_push(&registration.name);
            synchronizer.push()?;
        }
        Ok(())
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
        let store_path = Path::new(&registration.path);
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
