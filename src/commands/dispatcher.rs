use crate::adapters::file_system::FileSystemDefault;
use crate::commands::{completions, identities, recipients, stores};
use crate::models::cli::*;
use crate::models::configuration::Configuration;
use anyhow::{anyhow, Result};

pub fn dispatch_command(cli: Cli, configuration: Configuration) -> Result<()> {
    match &cli.command {
        Some(Commands::Completion { shell }) => completions::print(shell),
        Some(Commands::Identity { command }) => match command {
            IdentityCommands::Add(args) => identities::add(
                FileSystemDefault::new(),
                configuration,
                &cli.store,
                &args.file,
            ),
            IdentityCommands::Remove(args) => identities::remove(
                FileSystemDefault::new(),
                configuration,
                &cli.store,
                &args.file,
            ),
        },
        Some(Commands::Recipient { command }) => match command {
            RecipientCommands::Add(args) => recipients::add(
                FileSystemDefault::new(),
                configuration.select_store(cli.store),
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
            ),
        },
        None => Err(anyhow!("Unknown command encountered")),
    }
}
