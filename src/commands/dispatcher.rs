use anyhow::Result;

use crate::commands::{identities, recipients, secrets, stores};
use crate::models::cli::{
    Cli, Commands, IdentityCommands, RecipientCommands, SecretCommands, StoreCommands,
};
use crate::models::configuration::{Configuration, Store};

pub fn dispatch_command(cli: &Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Commands::Identity { command } => match command {
            IdentityCommands::Add(args) => identities::add(
                configuration,
                &args.store_selection.store,
                &args.file,
                args.global,
            ),
            IdentityCommands::Remove(args) => identities::remove(
                configuration,
                &args.store_selection.store,
                &args.file,
                args.global,
                args.ignore_missing,
            ),
        },
        Commands::Recipient { command } => match command {
            RecipientCommands::Add(args) => do_with_store(
                configuration.select_store(&args.store_selection.store),
                |store| recipients::add(store, &args.keys.public_key, &args.name, &args.path),
            ),
            RecipientCommands::Remove(_) => Ok(()),
            RecipientCommands::Inherit(_) => Ok(()),
        },
        Commands::Secret { command } => match command {
            SecretCommands::Copy(_) => Ok(()),
            SecretCommands::Edit(_) => Ok(()),
            SecretCommands::Generate(_) => Ok(()),
            SecretCommands::Grep(_) => Ok(()),
            SecretCommands::Insert(args) => do_with_store(
                configuration.select_store(&args.store_selection.store),
                |store| {
                    secrets::insert(
                        store,
                        args.multiline,
                        args.force,
                        args.inherit,
                        &args.secret_path,
                        &args.recipient,
                    )
                },
            ),
            SecretCommands::List(args) => do_with_store(
                configuration.select_store(&args.store_selection.store),
                secrets::list,
            ),
            SecretCommands::Move(_) => Ok(()),
            SecretCommands::Remove(_) => Ok(()),
            SecretCommands::Show(args) => do_with_store(
                configuration.select_store(&args.store_selection.store),
                |store| {
                    secrets::show(
                        store,
                        &configuration.all_identities(store),
                        &args.secret_path,
                    )
                },
            ),
        },
        Commands::Store { command } => match command {
            StoreCommands::Init(args) => stores::init(
                configuration,
                &args.path,
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
