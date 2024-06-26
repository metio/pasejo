use anyhow::{anyhow, Result};

use crate::adapters::file_system::FileSystemDefault;
use crate::models::cli::*;
use crate::models::configuration::Configuration;
use crate::commands::{recipients, stores};

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Identities { command}) => match command {
            IdentityCommands::Add { file: _file } => Ok(()),
            IdentityCommands::Remove { file: _file } => Ok(()),
        },
        Some(Commands::Recipients { command }) => match command {
            RecipientsCommands::Add {
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
            RecipientsCommands::Remove { public_key: _, path: _ } => Ok(()),
            RecipientsCommands::Inherit { path: _ } => Ok(()),
        },
        Some(Commands::Stores { command }) => match command {
            StoreCommands::Init { path, alias, vcs } => stores::init(
                Box::new(FileSystemDefault {}),
                configuration,
                path,
                alias,
                vcs,
            ),
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
