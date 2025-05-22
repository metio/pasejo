// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::commands::{config, identities, one_time_passwords, recipients, secrets, stores};
use crate::models::cli::{
    Cli, Commands, ConfigCommands, IdentityCommands, OtpCommands, RecipientCommands,
    SecretCommands, StoreCommands,
};
use crate::models::configuration::Configuration;
use crate::one_time_passwords::parser::parse_otp_args;
use crate::recipients::public_key;
use anyhow::Result;

#[allow(clippy::too_many_lines)]
pub fn dispatch_command(cli: &Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Commands::Config { command } => match command {
            ConfigCommands::Get(args) => {
                config::get(&configuration, &args.option);
                Ok(())
            }
            ConfigCommands::Set(args) => config::set(configuration, &args.option, &args.value),
        },
        Commands::Identity { command } => match command {
            IdentityCommands::Add(args) => identities::add(
                configuration,
                args.store_selection.store.as_ref(),
                args.file.as_path(),
                args.global,
            ),
            IdentityCommands::Remove(args) => identities::remove(
                configuration,
                args.store_selection.store.as_ref(),
                args.file.as_path(),
                args.global,
                args.ignore_unknown,
            ),
            IdentityCommands::List(args) => identities::list(
                &configuration,
                args.store_selection.store.as_ref(),
                args.global,
            ),
        },
        Commands::Otp { command } => match command {
            OtpCommands::Add(args) => one_time_passwords::add(
                &configuration,
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
            OtpCommands::Copy(args) => one_time_passwords::copy(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.source_path,
                &args.target_path,
                cli.offline,
            ),
            OtpCommands::List(args) => one_time_passwords::list(
                &configuration,
                args.store_selection.store.as_ref(),
                args.tree,
                cli.offline,
            ),
            OtpCommands::Move(args) => one_time_passwords::mv(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.current_path,
                &args.new_path,
                cli.offline,
            ),
            OtpCommands::Remove(args) => one_time_passwords::remove(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.password_path,
                cli.offline,
            ),
            OtpCommands::Show(args) => one_time_passwords::show(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.password_path,
                args.clip,
                cli.offline,
            ),
        },
        Commands::Recipient { command } => match command {
            RecipientCommands::Add(args) => recipients::add(
                &configuration,
                args.store_selection.store.as_ref(),
                &public_key::get(&args.keys)?,
                args.name.as_ref(),
                cli.offline,
            ),
            RecipientCommands::Remove(args) => recipients::remove(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.public_key,
                args.ignore_unknown,
                cli.offline,
            ),
            RecipientCommands::List(args) => recipients::list(
                &configuration,
                args.store_selection.store.as_ref(),
                cli.offline,
            ),
        },
        Commands::Secret { command } => match command {
            SecretCommands::Add(args) => secrets::add(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.secret_path,
                args.force,
                args.multiline,
                cli.offline,
            ),
            SecretCommands::Copy(args) => secrets::copy(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.source_path,
                &args.target_path,
                cli.offline,
            ),
            SecretCommands::List(args) => secrets::list(
                &configuration,
                args.store_selection.store.as_ref(),
                args.tree,
                cli.offline,
            ),
            SecretCommands::Show(args) => secrets::show(
                &configuration,
                args.store_selection.store.as_ref(),
                args.qrcode,
                &args.secret_path,
                args.line,
                args.clip,
                cli.offline,
            ),
            SecretCommands::Move(args) => secrets::mv(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.current_path,
                &args.new_path,
                cli.offline,
            ),
            SecretCommands::Remove(args) => secrets::remove(
                &configuration,
                args.store_selection.store.as_ref(),
                args.force,
                &args.secret_path,
                cli.offline,
            ),
            SecretCommands::Generate(args) => secrets::generate(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.secret_path,
                args.force,
                args.inplace,
                args.length,
                args.numbers,
                args.lowercase_letters,
                args.uppercase_letters,
                args.symbols,
                args.spaces,
                args.exclude_similar_characters,
                args.strict,
                cli.offline,
            ),
            SecretCommands::Edit(args) => secrets::edit(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.secret_path,
                cli.offline,
            ),
            SecretCommands::Grep(args) => secrets::grep(
                &configuration,
                args.store_selection.store.as_ref(),
                &args.search_string,
                args.regex,
                cli.offline,
            ),
        },
        Commands::Store { command } => match command {
            StoreCommands::Add(args) => stores::add(
                configuration,
                args.path.as_path(),
                &args.name,
                &args.synchronizer,
                args.default,
                cli.offline,
            ),
            StoreCommands::Decrypt(args) => stores::decrypt(
                &configuration,
                args.store_selection.store.as_ref(),
                cli.offline,
            ),
            StoreCommands::List(_) => {
                stores::list(&configuration);
                Ok(())
            }
            StoreCommands::Remove(args) => {
                stores::remove(configuration, args.store.as_ref(), args.remove_data)
            }
            StoreCommands::SetDefault(args) => stores::set_default(configuration, &args.name),
            StoreCommands::SetSynchronizer(args) => stores::set_synchronizer(
                configuration,
                args.store_selection.store.as_ref(),
                &args.synchronizer,
            ),
            StoreCommands::Sync(args) => stores::sync(
                &configuration,
                args.store_selection.store.as_ref(),
                args.pull,
                args.push,
            ),
        },
    }
}
