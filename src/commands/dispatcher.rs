// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use anyhow::Result;

use crate::commands::{identities, recipients, secrets, stores};
use crate::models::cli::{
    Cli, Commands, IdentityCommands, RecipientCommands, SecretCommands, StoreCommands,
};
use crate::models::configuration::{Configuration, Store};
use crate::recipients::public_key;

pub fn dispatch_command(cli: &Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
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
                args.ignore_missing,
            ),
        },
        Commands::Recipient { command } => match command {
            RecipientCommands::Add(args) => do_with_store(
                configuration.select_store(args.store_selection.store.as_ref()),
                |store| {
                    recipients::add(
                        store,
                        &public_key::get(&args.keys)?,
                        args.name.as_ref(),
                        args.path.as_ref(),
                    )
                },
            ),
            RecipientCommands::Remove(_) | RecipientCommands::Inherit(_) => Ok(()),
        },
        Commands::Secret { command } => match command {
            SecretCommands::Generate(_) | SecretCommands::Edit(_) | SecretCommands::Copy(_) => {
                Ok(())
            }
            SecretCommands::Grep(_) | SecretCommands::Remove(_) => Ok(()),
            SecretCommands::Insert(args) => do_with_store(
                configuration.select_store(args.store_selection.store.as_ref()),
                |store| {
                    secrets::insert(
                        store,
                        args.force,
                        args.inherit,
                        args.multiline,
                        &args.secret_path,
                        &args.recipient,
                    )
                },
            ),
            SecretCommands::List(args) => do_with_store(
                configuration.select_store(args.store_selection.store.as_ref()),
                |store| secrets::list(store, args.tree),
            ),
            SecretCommands::Move(args) => do_with_store(
                configuration.select_store(args.store_selection.store.as_ref()),
                |store| secrets::mv(store, &args.current_path, &args.new_path),
            ),
            SecretCommands::Show(args) => do_with_store(
                configuration.select_store(args.store_selection.store.as_ref()),
                |store| {
                    secrets::show(
                        store,
                        configuration.all_identity_files(store),
                        args.qrcode,
                        &args.secret_path,
                    )
                },
            ),
        },
        Commands::Store { command } => match command {
            StoreCommands::Init(args) => stores::init(
                configuration,
                args.path.as_path(),
                &args.name,
                &args.vcs,
                args.default,
            ),
            StoreCommands::Remove(args) => {
                stores::remove(configuration, &args.name, args.remove_data)
            }
            StoreCommands::SetDefault(args) => stores::set_default(configuration, &args.name),
        },
    }
}

fn do_with_store<F: FnOnce(&Store) -> Result<()>>(
    store: Option<&Store>,
    function: F,
) -> Result<()> {
    if let Some(store) = store {
        function(store)
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store init ...' first to create one"
        )
    }
}
