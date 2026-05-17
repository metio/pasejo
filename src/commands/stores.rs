// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, path};

use crate::cli::i18n;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::StoreCommands;
use crate::models::configuration::{Configuration, encrypt_store_to_path};
use crate::{one_time_passwords, recipients, secrets};
use anyhow::Context;

pub fn dispatch(
    command: &StoreCommands,
    configuration: Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        StoreCommands::Add(args) => {
            add(configuration, args.path.as_path(), &args.name, args.default)
        }
        StoreCommands::Decrypt(args) => decrypt(
            &configuration,
            args.store_selection.store.as_ref(),
            args.store_path.as_ref(),
            offline,
        ),
        StoreCommands::List(_) => {
            list(&configuration);
            Ok(())
        }
        StoreCommands::Merge(args) => merge(
            &configuration,
            args.store_selection.store.as_ref(),
            &args.common_ancestor,
            &args.current_version,
            &args.other_version,
        ),
        StoreCommands::Remove(args) => remove(configuration, args.store.as_ref(), args.remove_data),
        StoreCommands::SetDefault(args) => set_default(configuration, &args.name),
        StoreCommands::Exec(args) => exec(
            &configuration,
            args.store_selection.store.as_ref(),
            &args.command,
        ),
    }
}

fn add(
    mut configuration: Configuration,
    store_path: &Path,
    store_name: &str,
    default: bool,
) -> anyhow::Result<()> {
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
        i18n::store_add_success(store_name, &absolute_path.display().to_string());
        if default {
            set_default(configuration, store_name)?;
        }
        Ok(())
    } else {
        anyhow::bail!("Cannot create store path. Please check the path and try again.");
    }
}

fn remove(
    mut configuration: Configuration,
    store_name: Option<&String>,
    remove_data: bool,
) -> anyhow::Result<()> {
    let (name, path) = if let Some(registration) = configuration.select_store(store_name) {
        (registration.name.clone(), registration.path.clone())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    };

    // The configuration holds the only reference to the store's path, so
    // the file must be deleted before the registration is erased. A delete
    // that fails after the configuration is updated would orphan the file
    // on disk with no way to find it again.
    delete_store_file_if_requested(&path, remove_data)?;
    configuration.remove_store(&name)?;
    i18n::store_remove_success(&name);
    Ok(())
}

fn delete_store_file_if_requested(path: &Path, remove_data: bool) -> anyhow::Result<()> {
    if remove_data && path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn set_default(mut configuration: Configuration, store_name: &str) -> anyhow::Result<()> {
    configuration.set_default_store(store_name)?;
    i18n::store_set_default(store_name);
    Ok(())
}

fn decrypt(
    configuration: &Configuration,
    store_name: Option<&String>,
    store_path: Option<&PathBuf>,
    offline: bool,
) -> anyhow::Result<()> {
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
            configuration.decrypt_store_from_path(registration, path)?
        } else {
            configuration.decrypt_store(registration)?
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

fn list(configuration: &Configuration) {
    for store in &configuration.stores {
        let is_default = configuration
            .default_store
            .as_ref()
            .is_some_and(|default| default == &store.name);
        i18n::list_store(&store.name, &store.path, is_default);
    }
}

fn merge(
    configuration: &Configuration,
    store_name: Option<&String>,
    common_ancestor: &Path,
    current_version: &Path,
    other_version: &Path,
) -> anyhow::Result<()> {
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
            encrypt_store_to_path(&current_version_store, current_version)
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

fn exec(
    configuration: &Configuration,
    store_name: Option<&String>,
    command: &[String],
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let store_path = registration.path();
        if let Some(parent) = store_path.parent() {
            if let Some((binary, rest)) = command.split_first() {
                let status = Command::new(binary)
                    .args(rest)
                    .env("PASEJO_EXEC_STORE_PATH", store_path)
                    .env("PASEJO_EXEC_STORE_PARENT", parent)
                    .env("PASEJO_EXEC_COMMAND", binary)
                    .current_dir(parent)
                    .status()
                    .with_context(|| format!("Failed to run command {binary}"))?;
                if !status.success() {
                    let exit = status
                        .code()
                        .map_or_else(|| String::from("signal"), |c| c.to_string());
                    anyhow::bail!("Command {binary} exited with {exit}");
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    #[test]
    fn delete_store_file_if_requested_does_nothing_when_remove_data_is_false() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();

        delete_store_file_if_requested(file.path(), false).unwrap();

        assert!(file.path().exists(), "file must remain when remove_data=false");
    }

    #[test]
    fn delete_store_file_if_requested_deletes_existing_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();

        delete_store_file_if_requested(file.path(), true).unwrap();

        assert!(!file.path().exists());
    }

    #[test]
    fn delete_store_file_if_requested_is_noop_when_file_already_missing() {
        let temp = TempDir::new().unwrap();
        let missing = temp.child("never-existed");

        delete_store_file_if_requested(missing.path(), true).unwrap();
    }

    #[test]
    fn delete_store_file_if_requested_fails_when_path_is_a_directory() {
        // A path pointing at a directory triggers a reliable remove_file
        // failure across platforms — used here to confirm the helper
        // surfaces the error to its caller instead of swallowing it.
        let temp = TempDir::new().unwrap();
        let dir = temp.child("not-a-file");
        dir.create_dir_all().unwrap();

        let result = delete_store_file_if_requested(dir.path(), true);

        assert!(result.is_err());
        assert!(dir.path().exists(), "directory must remain on failure");
    }
}
