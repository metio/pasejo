use anyhow::{anyhow, Result};

use crate::adapters::file_system::FileSystemDefault;
use crate::models::cli::*;
use crate::models::configuration::Configuration;
use crate::commands::{recipients, stores};

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Identity { command}) => match command {
            IdentityCommands::Add { file: _file } => Ok(()),
            IdentityCommands::Remove { file: _file } => Ok(()),
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
            RecipientCommands::Remove { public_key: _, path: _ } => Ok(()),
            RecipientCommands::Inherit { path: _ } => Ok(()),
        },
        Some(Commands::Store { command }) => match command {
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
