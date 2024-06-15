use std::path::PathBuf;
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

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

impl Cli {
    pub fn dispatch_command(&self) -> Result<()> {
        match &self.command {
            Some(Commands::Recipients { command }) => {
                match command {
                    RecipientsCommands::Add { path } => {
                        Ok(())
                    }
                    RecipientsCommands::Remove { path } => {
                        Ok(())
                    }
                    RecipientsCommands::Inherit { path } => {
                        Ok(())
                    }
                }
            }
            None => Err(anyhow!("Unknown command encountered"))
        }
    }
}
