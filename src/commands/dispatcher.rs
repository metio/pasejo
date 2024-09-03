use crate::adapters::file_system::FileSystemDefault;
use crate::commands::{identities, recipients, stores};
use crate::models::cli::*;
use crate::models::configuration::Configuration;
use anyhow::{anyhow, Result};

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Identity { command }) => match command {
            IdentityCommands::Add(args) => identities::add(
                FileSystemDefault::new(),
                configuration,
                &args.store_selection.store,
                &args.file,
            ),
            IdentityCommands::Remove(args) => identities::remove(
                FileSystemDefault::new(),
                configuration,
                &args.store_selection.store,
                &args.file,
            ),
        },
        Some(Commands::Recipient { command }) => match command {
            RecipientCommands::Add(args) => recipients::add(
                FileSystemDefault::new(),
                configuration.select_store(&args.store_selection.store),
                &args.public_key,
                &args.name,
                &args.path,
            ),
            RecipientCommands::Remove(_) => Ok(()),
            RecipientCommands::Inherit(_) => Ok(()),
        },
        Some(Commands::Store { command }) => match command {
            StoreCommands::Init(args) => stores::init(
                FileSystemDefault::new(),
                configuration,
                &args.path,
                &args.alias,
                &args.vcs,
                &args.default,
            ),
            StoreCommands::Remove(args) => stores::remove(
                FileSystemDefault::new(),
                configuration,
                &args.alias,
                &args.remove_data,
            ),
            StoreCommands::SetDefault(args) => stores::set_default(configuration, &args.alias),
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
