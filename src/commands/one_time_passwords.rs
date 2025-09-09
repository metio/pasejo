// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, logs, prompts};
use crate::hooks::executor::HookExecutor;
use crate::models::configuration::Configuration;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType};
use crate::secrets;
use anyhow::Context;
use std::time::Duration;

pub fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    password_path: &str,
    password: &OneTimePassword,
    force: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.otp.contains_key(password_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {password_path}. Use --force to overwrite."
            );
        }

        store.otp.insert(password_path.to_owned(), password.clone());

        Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

        logs::one_time_password_added(password_path);

        hooks.execute_push_commands()?;

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    password_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.otp.contains_key(password_path)
            && !force
            && !prompts::get_confirmation_from_user("Remove existing one-time password?")?
        {
            anyhow::bail!(
                "Not allowed to remove one-time password at {password_path}. Use --force to overwrite."
            );
        }

        if store.otp.remove(password_path).is_some() {
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::one_time_password_removed(password_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No one-time password found at '{password_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if tree {
            print!(
                "{}",
                secrets::format_as_tree("", &store.otp_names_as_list())
            );
        } else {
            for secret in store.otp_names_as_list() {
                println!("{secret}");
            }
        }
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    password_path: &str,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if let Some(password) = store.otp.get_mut(password_path) {
            let is_hotp = password.otp_type == OneTimePasswordType::Hotp;
            let code = password.generate()?;
            if is_hotp {
                Configuration::encrypt_store(registration, &store)
                    .context("Cannot encrypt store")?;
            }
            logs::one_time_password_show(password_path);
            println!("{code}");
            if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                clipboard::copy_text_to_clipboard(&format!("{code}"), duration)?;
            }
            if is_hotp {
                hooks.execute_push_commands()?;
            }
            Ok(())
        } else {
            anyhow::bail!("No one-time password found at '{password_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn mv(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    current_path: &str,
    new_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.otp.contains_key(new_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {new_path}. Use --force to overwrite."
            );
        }

        if let Some(password) = store.otp.remove(current_path) {
            store.otp.insert(new_path.to_owned(), password);
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::one_time_password_moved(current_path, new_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No one-time password found at '{current_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn copy(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    source_path: &str,
    target_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let mut store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        if store.otp.contains_key(target_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {target_path}. Use --force to overwrite."
            );
        }

        if let Some(password) = store.otp.get(source_path) {
            store
                .otp
                .insert(target_path.to_owned(), password.to_owned());
            Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;

            logs::one_time_password_copied(source_path, target_path);

            hooks.execute_push_commands()?;

            Ok(())
        } else {
            anyhow::bail!("No one-time password found at '{source_path}'")
        }
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
