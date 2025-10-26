// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, logs, prompts};
use crate::hooks::executor::HookExecutor;
use crate::models::cli::{Cli, OtpCommands};
use crate::models::configuration::Configuration;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType};
use crate::one_time_passwords::parser::parse_otp_args;
use crate::secrets;
use anyhow::Context;
use std::time::Duration;

pub fn dispatch(
    command: &OtpCommands,
    cli: &Cli,
    configuration: &Configuration,
) -> anyhow::Result<()> {
    match command {
        OtpCommands::Add(args) => add(
            configuration,
            args.store_selection.store.as_ref(),
            &args.password_path,
            &parse_otp_args(
                args.otp_type.as_ref(),
                args.algorithm.as_ref(),
                args.secret.as_ref(),
                args.digits,
                args.period,
                args.counter,
                args.skew,
                args.url.as_ref(),
                args.qrcode.as_ref(),
            )?,
            args.force,
            cli.offline,
        ),
        OtpCommands::Copy(args) => copy(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.source_path,
            &args.target_path,
            cli.offline,
        ),
        OtpCommands::List(args) => list(
            configuration,
            args.store_selection.store.as_ref(),
            args.tree,
            cli.offline,
        ),
        OtpCommands::Move(args) => mv(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.current_path,
            &args.new_path,
            cli.offline,
        ),
        OtpCommands::Remove(args) => remove(
            configuration,
            args.store_selection.store.as_ref(),
            args.force,
            &args.password_path,
            cli.offline,
        ),
        OtpCommands::Show(args) => show(
            configuration,
            args.store_selection.store.as_ref(),
            &args.password_path,
            args.clip,
            cli.offline,
        ),
    }
}

fn add(
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

fn remove(
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

fn list(
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

fn show(
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

            if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                logs::one_time_password_copy_into_clipboard(password_path, &duration);
                clipboard::copy_text_to_clipboard(&format!("{code}"), duration)?;
            } else {
                logs::one_time_password_show(password_path);
                println!("{code}");
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

fn mv(
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

fn copy(
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
