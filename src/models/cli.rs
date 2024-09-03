use crate::adapters::vcs::VersionControlSystems;
use clap::ValueHint::{AnyPath, DirPath, FilePath};
use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pasejo")]
#[command(about = "age-backed password manager for teams", long_about = None)]
pub struct Cli {
    /// Optional name of store to use. Defaults to the first store defined in the local user configuration
    #[arg(short, long)]
    pub store: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate shell completions
    Completion { shell: Shell },

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
    #[arg(short, long, value_hint = FilePath)]
    pub file: PathBuf,
}

#[derive(Args)]
pub struct IdentityRemoveArgs {
    pub name: Option<String>,
}

#[derive(Subcommand)]
pub enum RecipientCommands {
    /// Adds a recipient
    Add(RecipientAddArgs),

    /// Remove a recipient
    Remove(RecipientRemoveArgs),

    /// Removes the recipients of a folder or secret so that it inherits its recipients from its parent
    Inherit(RecipientInheritArgs),
}

#[derive(Args)]
pub struct RecipientAddArgs {
    /// The public key of the new recipient
    #[arg(short = 'k', long)]
    pub public_key: String,

    /// The name of the new recipient
    #[arg(short, long)]
    pub name: Option<String>,

    /// The path to a folder or secret that should be readable by the given recipient
    #[arg(short, long, value_hint = AnyPath)]
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct RecipientRemoveArgs {
    /// The public key of the recipient to remove
    #[arg(short = 'k', long)]
    pub public_key: String,

    /// The path to a folder or secret that should no longer be readable by the given recipient
    #[arg(short, long)]
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct RecipientInheritArgs {
    /// The path to a folder or secret that should inherit its recipients from its parent
    #[arg(short, long, value_hint = AnyPath)]
    pub path: PathBuf,
}

#[derive(Subcommand)]
pub enum StoreCommands {
    /// Initialize a new store
    Init(StoreInitArgs),

    /// Mark a store as default
    SetDefault(StoreDefaultArgs),
}

#[derive(Args)]
pub struct StoreInitArgs {
    /// The path on your local system for the new store
    #[arg(short, long, value_hint = DirPath)]
    pub path: PathBuf,

    /// The alias for the new store
    #[arg(short, long)]
    pub alias: String,

    /// The version control system to use
    #[arg(short, long, default_value_t, value_enum)]
    pub vcs: VersionControlSystems,

    /// Whether the new store should be the default store
    #[arg(short, long)]
    pub default: bool,
}

#[derive(Args)]
pub struct StoreDefaultArgs {
    /// The alias of the store to use as default
    pub alias: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
