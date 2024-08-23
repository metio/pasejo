use anyhow::{anyhow, Result};

use crate::adapters::file_system::FileSystemDefault;
use crate::commands::{identities, recipients, stores};
use crate::models::cli::*;
use crate::models::configuration::Configuration;

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Identity { command }) => match command {
            IdentityCommands::Add(args) => {
                identities::add(configuration, &cli.store, &args.file, &args.inline)
            }
            IdentityCommands::Remove(_) => Ok(()),
        },
        Some(Commands::Recipient { command }) => match command {
            RecipientCommands::Add {
                public_key,
                name,
                path,
            } => recipients::add(
                Box::new(FileSystemDefault {}),
                configuration.select_store(cli.store),
                public_key,
                name,
                path,
            ),
            RecipientCommands::Remove {
                public_key: _,
                path: _,
            } => Ok(()),
            RecipientCommands::Inherit { path: _ } => Ok(()),
        },
        Some(Commands::Store { command }) => match command {
            StoreCommands::Init(args) => stores::init(
                Box::new(FileSystemDefault {}),
                configuration,
                &args.path,
                &args.alias,
                &args.vcs,
            ),
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
