use crate::adapters::vcs::VersionControlSystems;
use crate::models::configuration::Configuration;
use clap::ValueHint::{AnyPath, DirPath, FilePath};
use clap::{Args, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use std::path::PathBuf;

/// age-backed password manager for teams
#[derive(Parser)]
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
    /// Optional name of store to use. Defaults to the default store or the first one defined in the
    /// local user configuration
    #[arg(short, long, add = ArgValueCompleter::new(store_alias_completer), value_parser = store_alias_is_known
    )]
    pub store: Option<String>,
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

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
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

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct RecipientRemoveArgs {
    /// The public key of the recipient to remove
    #[arg(short = 'k', long)]
    pub public_key: String,

    /// The path to a folder or secret that should no longer be readable by the given recipient
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct RecipientInheritArgs {
    /// The path to a folder or secret that should inherit its recipients from its parent
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

    /// Ignore existing recipients of existing secrets and inherit recipients from nearest parent folder. Disabled when --recipient is used as well
    #[arg(short, long)]
    pub inherit: bool,

    /// Specify recipients for the new secret. Can be specified multiple times. If none are specified, will read from nearest .recipients file
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
pub struct StoreRemoveArgs {
    /// The alias of the existing store
    #[arg(short, long, value_parser = store_alias_is_known)]
    pub alias: String,

    /// Whether the store should be removed from the local file system
    #[arg(short, long)]
    pub remove_data: bool,
}

#[derive(Args)]
pub struct StoreDefaultArgs {
    /// The alias of the store to use as default
    #[arg(value_parser = store_alias_is_known)]
    pub alias: String,
}

fn store_alias_is_known(input: &str) -> Result<String, String> {
    let configuration = Configuration::load();
    let aliases = configuration.all_store_aliases();

    if aliases.contains(&input.to_owned()) {
        Ok(input.to_owned())
    } else {
        Err(format!("alias {} does not exist in configuration", input))
    }
}

fn store_alias_completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let configuration = Configuration::load();
    let aliases = configuration.all_store_aliases();

    match current.to_str() {
        Some(value) => aliases
            .iter()
            .filter(|&alias| alias.starts_with(value))
            .map(|alias| CompletionCandidate::new(alias))
            .collect(),
        None => aliases
            .iter()
            .map(|alias| CompletionCandidate::new(alias))
            .collect(),
    }
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
