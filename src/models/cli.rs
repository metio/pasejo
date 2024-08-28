use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::adapters::vcs::VersionControlSystems;

/// pasejo: age-backed password manager
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
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },

    /// Manage recipients
    Recipient {
        #[command(subcommand)]
        command: RecipientCommands,
    },

    /// Manage stores
    Store {
        #[command(subcommand)]
        command: StoreCommands,
    },
}

#[derive(Subcommand)]
pub enum IdentityCommands {
    /// Adds an identity
    Add(IdentityAddRemoveArgs),

    /// Remove an identity
    Remove(IdentityAddRemoveArgs),
}

#[derive(Args)]
pub struct IdentityAddRemoveArgs {
    /// The path to the identity file
    #[arg(short, long)]
    pub file: PathBuf,
}

#[derive(Args)]
pub struct IdentityRemoveArgs {
    pub name: Option<String>,
}

#[derive(Subcommand)]
pub enum RecipientCommands {
    /// Adds a recipient
    Add {
        /// The public key of the new recipient
        #[arg(short = 'k', long)]
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
        #[arg(short = 'k', long)]
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
    Init(StoreInitArgs),
}

#[derive(Args)]
pub struct StoreInitArgs {
    /// The path on your local system for the new store
    #[arg(short, long)]
    pub path: PathBuf,

    /// The alias for the new store
    #[arg(short, long)]
    pub alias: String,

    /// The version control system to use
    #[arg(short, long, default_value_t, value_enum)]
    pub vcs: VersionControlSystems,
}
