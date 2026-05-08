// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::{clipboard, i18n, prompts};
use crate::commands::store_op::{StoreMutation, with_store, with_store_then};
use crate::models::cli::{Cli, OtpCommands};
use crate::models::configuration::Configuration;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType};
use crate::one_time_passwords::parser::parse_otp_args;
use crate::secrets;
use std::time::Duration;
use zeroize::Zeroizing;

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
    with_store(configuration, store_name, offline, |_, store| {
        if store.otp.contains_key(password_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {password_path}. Use --force to overwrite."
            );
        }
        store.otp.insert(password_path.to_owned(), password.clone());
        i18n::one_time_password_added(password_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    password_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if store.otp.contains_key(password_path)
            && !force
            && !prompts::get_confirmation_from_user("Remove existing one-time password?")?
        {
            anyhow::bail!(
                "Not allowed to remove one-time password at {password_path}. Use --force to overwrite."
            );
        }
        if store.otp.remove(password_path).is_none() {
            anyhow::bail!("No one-time password found at '{password_path}'")
        }
        i18n::one_time_password_removed(password_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    tree: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
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
        Ok(((), StoreMutation::Unchanged))
    })
}

fn show(
    configuration: &Configuration,
    store_name: Option<&String>,
    password_path: &str,
    clip: bool,
    offline: bool,
) -> anyhow::Result<()> {
    with_store_then(
        configuration,
        store_name,
        offline,
        |_, store| {
            let Some(password) = store.otp.get_mut(password_path) else {
                anyhow::bail!("No one-time password found at '{password_path}'")
            };
            let is_hotp = password.otp_type == OneTimePasswordType::Hotp;
            let code = password.generate()?;
            let mutation = if is_hotp {
                StoreMutation::Modified
            } else {
                StoreMutation::Unchanged
            };
            Ok((code, mutation))
        },
        |code: &u32| {
            if clip {
                let duration = Duration::from_secs(configuration.clipboard_timeout.unwrap_or(45));
                i18n::one_time_password_copy_into_clipboard(password_path, &duration);
                let code_text = Zeroizing::new(format!("{code}"));
                clipboard::copy_text_to_clipboard(code_text.as_str(), duration)?;
            } else {
                i18n::one_time_password_show(password_path);
                println!("{code}");
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn mv(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    current_path: &str,
    new_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if store.otp.contains_key(new_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {new_path}. Use --force to overwrite."
            );
        }
        let Some(password) = store.otp.remove(current_path) else {
            anyhow::bail!("No one-time password found at '{current_path}'")
        };
        store.otp.insert(new_path.to_owned(), password);
        i18n::one_time_password_moved(current_path, new_path);
        Ok(((), StoreMutation::Modified))
    })
}

fn copy(
    configuration: &Configuration,
    store_name: Option<&String>,
    force: bool,
    source_path: &str,
    target_path: &str,
    offline: bool,
) -> anyhow::Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        if store.otp.contains_key(target_path)
            && !force
            && !prompts::get_confirmation_from_user("Overwrite existing one-time password?")?
        {
            anyhow::bail!(
                "One-time password already exists at {target_path}. Use --force to overwrite."
            );
        }
        let Some(password) = store.otp.get(source_path) else {
            anyhow::bail!("No one-time password found at '{source_path}'")
        };
        store
            .otp
            .insert(target_path.to_owned(), password.to_owned());
        i18n::one_time_password_copied(source_path, target_path);
        Ok(((), StoreMutation::Modified))
    })
}
