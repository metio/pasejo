use std::path::PathBuf;

use clap::ValueHint::{AnyPath, DirPath, FilePath};
use clap::{Args, Parser, Subcommand};

use crate::adapters::vcs::VersionControlSystems;
use crate::cli::completer;
use crate::cli::parser;

/// age-backed password manager for teams
#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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

    /// Manage secrets
    Secret {
        #[command(subcommand)]
        command: SecretCommands,
    },

    /// Manage stores
    Store {
        #[command(subcommand)]
        command: StoreCommands,
    },
}

#[derive(Args)]
pub struct StoreSelectionArgs {
    /// Optional name of store to use. Defaults to the default store or the
    /// first one defined in the local user configuration
    #[arg(short, long, add = completer::store_name(), value_parser = parser::store_name)]
    pub store: Option<String>,
}

#[derive(Subcommand)]
pub enum IdentityCommands {
    /// Adds an identity either to a single store or to your global
    /// configuration file.
    Add(IdentityAddArgs),

    /// Remove an identity
    Remove(IdentityRemoveArgs),
}

#[derive(Args)]
pub struct IdentityAddArgs {
    /// The path to the identity file
    #[arg(short, long, value_hint = FilePath, value_parser = parser::existing_file)]
    pub file: PathBuf,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Add to the global configuration file when enabled, otherwise add to
    /// store
    #[arg(short, long, conflicts_with = "store")]
    pub global: bool,
}

#[derive(Args)]
pub struct IdentityRemoveArgs {
    /// The path to the identity file
    #[arg(short, long, value_hint = FilePath)]
    pub file: PathBuf,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Remove from the global configuration file when enabled, otherwise remove
    /// from store
    #[arg(short, long, conflicts_with = "store")]
    pub global: bool,
}

#[derive(Subcommand)]
pub enum RecipientCommands {
    /// Adds a recipient
    Add(RecipientAddArgs),

    /// Remove a recipient
    Remove(RecipientRemoveArgs),

    /// Removes the recipients of a folder or secret so that it inherits its
    /// recipients from its parent
    Inherit(RecipientInheritArgs),
}

#[derive(Args)]
pub struct RecipientAddArgs {
    #[command(flatten)]
    pub keys: RecipientKeysArgs,

    /// The name of the new recipient
    #[arg(short, long)]
    pub name: Option<String>,

    /// The path to a folder or secret that should be readable by the given
    /// recipient
    #[arg(short, long, value_hint = AnyPath)]
    pub path: Option<PathBuf>,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct RecipientKeysArgs {
    /// The public key of the new recipient
    #[arg(short = 'k', long)]
    pub public_key: String,
    // /// The GitHub username to add as recipient
    // #[arg(long)]
    // pub github: String,
    //
    // /// The GitLab username to add as recipient
    // #[arg(long)]
    // pub gitlab: String,
}

#[derive(Args)]
pub struct RecipientRemoveArgs {
    /// The public key of the recipient to remove
    #[arg(short = 'k', long)]
    pub public_key: String,

    /// The path to a folder or secret that should no longer be readable by the
    /// given recipient
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct RecipientInheritArgs {
    /// The path to a folder or secret that should inherit its recipients from
    /// its parent
    #[arg(short, long, value_hint = AnyPath)]
    pub path: PathBuf,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Subcommand)]
pub enum SecretCommands {
    /// Copy secret from old-path to new-path
    Copy(SecretCopyArgs),

    /// Edit an existing secret
    Edit(SecretEditArgs),

    /// Generate a secret and insert it into the store
    Generate(SecretGenerateArgs),

    /// Grep for a search-string in secrets when decrypted
    Grep(SecretGrepArgs),

    /// Insert a new secret or overwrite an existing one
    Insert(SecretInsertArgs),

    /// List all secrets, optionally limited to a subfolder of a store
    List(SecretListArgs),

    /// Move secret from old-path to new-path
    Move(SecretMoveArgs),

    /// Remove an existing secret
    Remove(SecretRemoveArgs),

    /// Show secret
    Show(SecretShowArgs),
}

#[derive(Args)]
pub struct SecretCopyArgs {
    /// Toggle prompt for overwrites of existing secrets
    #[arg(short, long)]
    pub force: bool,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretEditArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretGenerateArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretGrepArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretInsertArgs {
    /// Toggle multiline edit mode
    #[arg(short, long)]
    pub multiline: bool,

    /// Toggle prompt for overwrites of existing secrets and recipients
    #[arg(short, long)]
    pub force: bool,

    /// Ignore existing recipients of existing secrets and inherit recipients
    /// from nearest parent folder. Disabled when --recipient is used as well
    #[arg(short, long)]
    pub inherit: bool,

    /// Specify recipients for the new secret. Can be specified multiple times.
    /// If none are specified, will read from nearest .recipients file
    #[arg(short, long)]
    pub recipient: Vec<String>,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Args)]
pub struct SecretListArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretMoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretRemoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct SecretShowArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Subcommand)]
pub enum StoreCommands {
    /// Initialize a new store
    Init(StoreInitArgs),

    /// Remove an existing store
    Remove(StoreRemoveArgs),

    /// Mark a store as default
    SetDefault(StoreDefaultArgs),
}

#[derive(Args)]
pub struct StoreInitArgs {
    /// The path on your local system for the new store
    #[arg(short, long, value_hint = DirPath)]
    pub path: PathBuf,

    /// The name for the new store
    #[arg(short, long)]
    pub name: String,

    /// The version control system to use
    #[arg(short, long, default_value_t, value_enum)]
    pub vcs: VersionControlSystems,

    /// Whether the new store should be the default store
    #[arg(short, long)]
    pub default: bool,
}

#[derive(Args)]
pub struct StoreRemoveArgs {
    /// The name of the existing store
    #[arg(short, long, value_parser = parser::store_name)]
    pub name: String,

    /// Whether the store should be removed from the local file system
    #[arg(short, long)]
    pub remove_data: bool,
}

#[derive(Args)]
pub struct StoreDefaultArgs {
    /// The name of the store to use as default
    #[arg(value_parser = parser::store_name)]
    pub name: String,
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
