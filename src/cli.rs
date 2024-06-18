use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

use crate::adapters::file_system::FileSystem;
use crate::adapters::file_system_rust_std::FileSystemRustStd;
use crate::adapters::git_native::GitNative;
use crate::commands::recipient::recipient_add;
use crate::configuration::{Configuration, StoreTypes};
use crate::stores::api::Store;
use crate::stores::git::GitStore;
use crate::stores::local::LocalStore;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
pub struct Cli {
    // /// Optional name of secret to search for
    // name: Option<String>,
    /// Optional name of store to use. Defaults to the first store defined in the local user configuration
    store: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage recipients
    Recipients {
        #[command(subcommand)]
        command: RecipientsCommands,
    },
    /// Manage stores
    Stores {
        #[command(subcommand)]
        command: StoreCommands,
    },
}

#[derive(Subcommand)]
pub enum RecipientsCommands {
    /// Adds a recipient
    Add {
        /// The path to a folder or secret that should be readable by the given recipient
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Remove a recipient
    Remove {
        /// The path to a folder or secret that should no longer be readable by the given recipient
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Removes the recipients of a folder or secret so that it inherits its recipients from its parent
    Inherit {
        /// The path to a folder or secret that should inherit its recipients from its parent
        #[arg(short, long)]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum StoreCommands {
    /// Initialize a new store
    Init {
        /// The path on your local system for the new store
        #[arg(short, long)]
        path: PathBuf,

        /// The alias for the new store
        #[arg(short, long)]
        alias: String,

        /// The alias for the new store
        #[arg(short, long, default_value_t, value_enum)]
        r#type: StoreTypes,
    },
}

impl Cli {
    pub fn dispatch_command(&self, configuration: Configuration) -> Result<()> {
        match &self.command {
            Some(Commands::Recipients { command }) => match command {
                RecipientsCommands::Add { path } => recipient_add(
                    select_store_implementation(&self.store, configuration),
                    path,
                ),
                RecipientsCommands::Remove { path } => Ok(()),
                RecipientsCommands::Inherit { path } => Ok(()),
            },
            Some(Commands::Stores { command }) => match command {
                StoreCommands::Init {
                    path,
                    alias,
                    r#type,
                } => Ok(()),
            },
            None => Err(anyhow!("Unknown command encountered")),
        }
    }
}

fn select_store_implementation(
    store: &Option<String>,
    configuration: Configuration,
) -> Box<dyn Store> {
    let selected_store = match store {
        Some(alias) => configuration
            .stores
            .iter()
            .find(|&store| store.alias.eq(alias))
            .expect("Cannot find store for given alias"),
        None => configuration
            .stores
            .first()
            .expect("No store is configured"),
    };

    let file_system_adapter: Box<dyn FileSystem> = Box::new(FileSystemRustStd {});

    match selected_store.r#type {
        StoreTypes::Git => Box::new(GitStore {
            git_adapter: Box::new(GitNative {}),
            file_system_adapter,
        }),
        _ => Box::new(LocalStore {
            file_system_adapter,
        }),
    }
}
