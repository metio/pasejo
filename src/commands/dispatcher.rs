use anyhow::{anyhow, Result};

use crate::adapters::file_system;
use crate::cli::arguments::*;
use crate::cli::configuration;
use crate::commands::{recipients, stores};

pub fn dispatch_command(cli: Cli, configuration: configuration::Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Recipients { command }) => match command {
            RecipientsCommands::Add {
                public_key,
                name,
                path,
            } => recipients::add(
                Box::new(file_system::FileSystemDefault {}),
                configuration.select_store(cli.store),
                public_key,
                name,
                path,
            ),
            RecipientsCommands::Remove { path } => Ok(()),
            RecipientsCommands::Inherit { path } => Ok(()),
        },
        Some(Commands::Stores { command }) => match command {
            StoreCommands::Init { path, alias, vcs } => stores::init(
                Box::new(file_system::FileSystemDefault {}),
                configuration,
                path,
                alias,
                vcs,
            ),
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
