// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, path};

use anyhow::Context;

use crate::cli::i18n;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::StoreCommands;
use crate::models::configuration::{Configuration, encrypt_store_to_path};
use crate::{one_time_passwords, recipients, secrets};

pub fn dispatch(
    command: &StoreCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        StoreCommands::Add(args) => {
            let mut owned = configuration.clone();
            add(&mut owned, args.path.as_path(), &args.name, args.default)
        }
        StoreCommands::Decrypt(args) => decrypt(
            configuration,
            args.store_selection.store.as_ref(),
            args.store_path.as_ref(),
            args.yes_i_know,
            offline,
        ),
        StoreCommands::List(_) => {
            list(configuration);
            Ok(())
        }
        StoreCommands::Merge(args) => merge(
            configuration,
            args.store_selection.store.as_ref(),
            &args.common_ancestor,
            &args.current_version,
            &args.other_version,
        ),
        StoreCommands::Remove(args) => {
            let mut owned = configuration.clone();
            remove(&mut owned, args.store.as_ref(), args.remove_data)
        }
        StoreCommands::SetDefault(args) => {
            let mut owned = configuration.clone();
            set_default(&mut owned, &args.name)
        }
        StoreCommands::Exec(args) => exec(
            configuration,
            args.store_selection.store.as_ref(),
            &args.command,
        ),
    }
}

fn add(
    configuration: &mut Configuration,
    store_path: &Path,
    store_name: &str,
    default: bool,
) -> anyhow::Result<()> {
    if configuration.find_store(store_name).is_some() {
        anyhow::bail!(i18n::error_store_name_already_exists());
    }

    let absolute_path = path::absolute(store_path)?;

    if absolute_path.is_dir() {
        anyhow::bail!(i18n::error_store_path_is_directory());
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
        anyhow::bail!(i18n::error_cannot_create_store_path());
    }
}

fn remove(
    configuration: &mut Configuration,
    store_name: Option<&String>,
    remove_data: bool,
) -> anyhow::Result<()> {
    let (name, path) = if let Some(registration) = configuration.select_store(store_name) {
        (registration.name.clone(), registration.path.clone())
    } else {
        anyhow::bail!(i18n::error_no_store_in_configuration())
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

fn set_default(configuration: &mut Configuration, store_name: &str) -> anyhow::Result<()> {
    configuration.set_default_store(store_name)?;
    i18n::store_set_default(store_name);
    Ok(())
}

fn decrypt(
    configuration: &Configuration,
    store_name: Option<&String>,
    store_path: Option<&PathBuf>,
    yes_i_know: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        if !yes_i_know {
            anyhow::bail!(i18n::error_decrypt_requires_yes_i_know());
        }
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
        anyhow::bail!(i18n::error_no_store_in_configuration())
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
            .context(i18n::error_cannot_decrypt_common_ancestor_store())?;
        let mut current_version_store = configuration
            .decrypt_store_from_path(registration, current_version)
            .context(i18n::error_cannot_decrypt_current_version_store())?;
        let other_version_store = configuration
            .decrypt_store_from_path(registration, other_version)
            .context(i18n::error_cannot_decrypt_other_version_store())?;

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
        anyhow::bail!(i18n::error_no_store_in_configuration())
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
                    .with_context(|| i18n::error_failed_to_run_command(binary))?;
                if !status.success() {
                    let exit = status
                        .code()
                        .map_or_else(|| String::from("signal"), |c| c.to_string());
                    anyhow::bail!(i18n::error_command_exited_with(binary, &exit));
                }
            }
        } else {
            anyhow::bail!(i18n::error_cannot_get_store_parent())
        }

        Ok(())
    } else {
        anyhow::bail!(i18n::error_no_store_in_configuration())
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn delete_store_file_if_requested_does_nothing_when_remove_data_is_false() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();

        delete_store_file_if_requested(file.path(), false).unwrap();

        assert!(
            file.path().exists(),
            "file must remain when remove_data=false"
        );
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
