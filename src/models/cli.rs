use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::adapters::vcs::VersionControlSystems;

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
pub struct Cli {
    /// Optional name of store to use. Defaults to the first store defined in the local user configuration
    #[arg(short, long)]
    pub store: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage identities
    Identities {
        #[command(subcommand)]
        command: IdentityCommands,
    },

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
pub enum IdentityCommands {
    /// Adds an identity
    Add {
        /// The path to the identity file to add to the configuration
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Remove an identity
    Remove {
        /// The path to the identity file to remove from the configuration
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum RecipientsCommands {
    /// Adds a recipient
    Add {
        /// The public key of the new recipient
        #[arg(short, long)]
        public_key: String,

        /// The name of the new recipient
        #[arg(short, long)]
        name: Option<String>,

        /// The path to a folder or secret that should be readable by the given recipient
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Remove a recipient
    Remove {
        /// The public key of the recipient to remove
        #[arg(short, long)]
        public_key: String,

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

        /// The version control system to use
        #[arg(short, long, default_value_t, value_enum)]
        vcs: VersionControlSystems,
    },
}
