use crate::commands::{identities, recipients, secrets, stores};
use crate::models::cli::*;
use crate::models::configuration::Configuration;
use anyhow::Result;

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Commands::Identity { command } => match command {
            IdentityCommands::Add(args) => {
                identities::add(configuration, &args.store_selection.store, &args.file)
            }
            IdentityCommands::Remove(args) => {
                identities::remove(configuration, &args.store_selection.store, &args.file)
            }
        },
        Commands::Recipient { command } => match command {
            RecipientCommands::Add(args) => recipients::add(
                configuration.select_store(&args.store_selection.store),
                &args.public_key,
                &args.name,
                &args.path,
            ),
            RecipientCommands::Remove(_) => Ok(()),
            RecipientCommands::Inherit(_) => Ok(()),
        },
        Commands::Secret { command } => match command {
            SecretCommands::Copy(_) => Ok(()),
            SecretCommands::Edit(_) => Ok(()),
            SecretCommands::Generate(_) => Ok(()),
            SecretCommands::Grep(_) => Ok(()),
            SecretCommands::Insert(args) => secrets::insert(
                configuration.select_store(&args.store_selection.store),
                &args.multiline,
                &args.force,
                &args.inherit,
                &args.secret_path,
                &args.recipient,
            ),
            SecretCommands::List(_) => Ok(()),
            SecretCommands::Move(_) => Ok(()),
            SecretCommands::Remove(_) => Ok(()),
            SecretCommands::Show(args) => secrets::show(
                configuration.select_store(&args.store_selection.store),
                configuration.all_identities(&args.store_selection.store)?,
                &args.secret_path,
            ),
        },
        Commands::Store { command } => match command {
            StoreCommands::Init(args) => stores::init(
                configuration,
                &args.path,
                &args.alias,
                &args.vcs,
                &args.default,
            ),
            StoreCommands::Remove(args) => {
                stores::remove(configuration, &args.alias, &args.remove_data)
            }
            StoreCommands::SetDefault(args) => stores::set_default(configuration, &args.alias),
        },
    }
}
