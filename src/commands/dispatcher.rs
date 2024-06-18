use anyhow::{anyhow, Result};

use crate::adapters::file_system_std::FileSystemStd;
use crate::cli::arguments::{Cli, Commands, RecipientsCommands, StoreCommands};
use crate::cli::configuration::Configuration;
use crate::commands::recipients::recipient_add;
use crate::commands::stores::store_init;

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Recipients { command }) => match command {
            RecipientsCommands::Add {
                public_key,
                name,
                path,
            } => recipient_add(
                configuration.select_store(cli.store),
                public_key,
                name,
                path,
                Box::new(FileSystemStd {}),
            ),
            RecipientsCommands::Remove { path } => Ok(()),
            RecipientsCommands::Inherit { path } => Ok(()),
        },
        Some(Commands::Stores { command }) => match command {
            StoreCommands::Init { path, alias, vcs } => {
                store_init(path, alias, vcs, Box::new(FileSystemStd {}), configuration)
            }
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
