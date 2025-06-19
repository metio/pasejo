// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::PathBuf;

use crate::cli::completer;
use crate::cli::parser;
use crate::models::configuration::ConfigurationOption;
use crate::models::password_store::{OneTimePasswordAlgorithm, OneTimePasswordType};
use crate::synchronizers::synchronizer::Synchronizers;
use clap::ValueHint::{DirPath, FilePath};
use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::InfoLevel;

/// age-backed password manager for teams
#[derive(Parser)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<InfoLevel>,

    /// Work offline, do not synchronize with remote stores
    #[arg(short = 'O', long)]
    pub offline: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage pasejo configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    /// Manage identities
    Identity {
        #[command(subcommand)]
        command: IdentityCommands,
    },

    /// Manage one-time passwords
    Otp {
        #[command(subcommand)]
        command: OtpCommands,
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

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Get a configuration value
    Get(ConfigGetArgs),

    /// Set a configuration value
    Set(ConfigSetArgs),
}

#[derive(Args)]
pub struct ConfigGetArgs {
    /// Name of the configuration option to get
    pub option: ConfigurationOption,
}

#[derive(Args)]
pub struct ConfigSetArgs {
    /// Name of the configuration option to set
    pub option: ConfigurationOption,

    /// Value to set the configuration option to
    pub value: String,
}

#[derive(Args)]
pub struct StoreSelectionArgs {
    /// Optional name of store to use. Defaults to the default store or the
    /// first one defined in the local user configuration
    #[arg(short, long, add = completer::store_name(), value_parser = parser::store_name)]
    pub store: Option<String>,
}

#[derive(Subcommand)]
pub enum OtpCommands {
    /// Adds a one-time password
    Add(OtpAddArgs),

    /// Remove a one-time password
    Remove(OtpRemoveArgs),

    /// List one-time passwords
    List(OtpListArgs),

    /// Show a one-time password
    Show(OtpShowArgs),

    /// Copy a one-time password from old-path to new-path
    Copy(OtpCopyArgs),

    /// Move a one-time password from old-path to new-path
    Move(OtpMoveArgs),
}

#[derive(Args)]
pub struct OtpAddArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing one-time password without prompting
    #[arg(short, long)]
    pub force: bool,

    /// Parse an otpauth URL
    #[arg(long, conflicts_with_all = ["qrcode"])]
    pub url: Option<String>,

    /// Parse a QR code containing an otpauth URL
    #[arg(long, value_hint = FilePath, value_parser = parser::existing_file, conflicts_with_all = ["url"])]
    pub qrcode: Option<PathBuf>,

    /// The base secret of the one-time password
    #[arg(long, conflicts_with_all = ["url", "qrcode"])]
    pub secret: Option<String>,

    /// The type of the one-time password
    #[arg(long = "type", conflicts_with_all = ["url", "qrcode"])]
    pub otp_type: Option<OneTimePasswordType>,

    /// The algorithm of the one-time password
    #[arg(long, conflicts_with_all = ["url", "qrcode"])]
    pub algorithm: Option<OneTimePasswordAlgorithm>,

    /// The digits of the one-time password
    #[arg(long, conflicts_with_all = ["url", "qrcode"])]
    pub digits: Option<u8>,

    /// The period of the one-time password
    #[arg(long, conflicts_with_all = ["url", "qrcode", "counter"])]
    pub period: Option<u64>,

    /// The skew of the one-time password
    #[arg(long, group = "manual", conflicts_with_all = ["url", "qrcode", "counter"])]
    pub skew: Option<u64>,

    /// The counter of the one-time password
    #[arg(long, group = "manual", conflicts_with_all = ["url", "qrcode", "period"])]
    pub counter: Option<u64>,

    /// The path of the one-time password within the selected store
    pub password_path: String,
}

#[derive(Args)]
pub struct OtpRemoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Delete an existing one-time password without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The path of the one-time password within the selected store
    pub password_path: String,
}

#[derive(Args)]
pub struct OtpListArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Toggle to display one-time passwords as a tree
    #[arg(short, long)]
    pub tree: bool,
}

#[derive(Args)]
pub struct OtpShowArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Copy one-time password to clipboard
    #[arg(short, long)]
    pub clip: bool,

    /// The path of the one-time password within the selected store
    pub password_path: String,
}

#[derive(Args)]
pub struct OtpCopyArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing one-time password without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The source path of the one-time password
    pub source_path: String,

    /// The target path of the one-time password
    pub target_path: String,
}

#[derive(Args)]
pub struct OtpMoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing one-time password without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The current path of the secret
    pub current_path: String,

    /// The new path of the secret
    pub new_path: String,
}

#[derive(Subcommand)]
pub enum IdentityCommands {
    /// Adds an identity either to a single store or to your global
    /// configuration file.
    Add(IdentityAddArgs),

    /// Remove an identity
    Remove(IdentityRemoveArgs),

    /// List identities
    List(IdentityListArgs),
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

    /// Don't fail on unknown identities
    #[arg(short, long)]
    pub ignore_unknown: bool,
}

#[derive(Args)]
pub struct IdentityListArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Show only the global identities
    #[arg(short, long)]
    pub global: bool,
}

#[derive(Subcommand)]
pub enum RecipientCommands {
    /// Adds a recipient
    Add(RecipientAddArgs),

    /// Remove a recipient
    Remove(RecipientRemoveArgs),

    /// Lists all recipients
    List(RecipientListArgs),
}

#[derive(Args)]
pub struct RecipientAddArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    #[command(flatten)]
    pub keys: RecipientKeysArgs,

    /// The name of the new recipient
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
pub struct RecipientKeysArgs {
    /// The public key of the new recipient
    #[arg(short, long)]
    pub public_key: Option<String>,

    /// Read public key of recipient from a file
    #[arg(short, long)]
    pub file: Option<String>,

    /// The Codeberg username to add as recipient
    #[arg(long)]
    pub codeberg: Option<String>,

    /// The GitHub username to add as recipient
    #[arg(long)]
    pub github: Option<String>,

    /// The GitLab username to add as recipient
    #[arg(long)]
    pub gitlab: Option<String>,
}

#[derive(Args)]
pub struct RecipientRemoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Don't fail on unknown recipients
    #[arg(short, long)]
    pub ignore_unknown: bool,

    /// The public key of the recipient to remove
    pub public_key: String,
}

#[derive(Args)]
pub struct RecipientListArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Subcommand)]
pub enum SecretCommands {
    /// Add a new secret or overwrite an existing one
    Add(SecretAddArgs),

    /// Copy secret from old-path to new-path
    Copy(SecretCopyArgs),

    /// Edit an existing secret
    Edit(SecretEditArgs),

    /// Generate a secret and add it into the store
    Generate(SecretGenerateArgs),

    /// Grep for a search-string in secrets when decrypted
    Grep(SecretGrepArgs),

    /// List all secrets
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
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing secrets without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The path of an existing secret
    pub source_path: String,

    /// The target path for the copied secret
    pub target_path: String,
}

#[derive(Args)]
pub struct SecretEditArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecretGenerateArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing secrets without prompting
    #[arg(short, long)]
    pub force: bool,

    /// Overwrite just the password of an existing secret without prompting
    #[arg(short, long)]
    pub inplace: bool,

    /// The path of the secret within the selected store
    pub secret_path: String,

    /// The length of the generated passwords.
    #[arg(short, long, default_value_t = 25)]
    pub length: usize,

    /// Passwords are allowed to, or must if the strict is true, contain a number.
    #[arg(short, long, default_value_t = true)]
    pub numbers: bool,

    /// Passwords are allowed to, or must if the strict is true, contain a lowercase letter.
    #[arg(short = 'j', long, default_value_t = true)]
    pub lowercase_letters: bool,

    /// Passwords are allowed to, or must if the strict is true, contain an uppercase letter.
    #[arg(short, long, default_value_t = true)]
    pub uppercase_letters: bool,

    /// Passwords are allowed to, or must if the strict is true, contain a symbol.
    #[arg(short = 'y', long, default_value_t = false)]
    pub symbols: bool,

    /// Passwords are allowed to, or must if the strict is true, contain a space.
    #[arg(short = 'w', long, default_value_t = false)]
    pub spaces: bool,

    /// Whether to exclude similar characters, iI1loO0"'`|`
    #[arg(short, long, default_value_t = true)]
    pub exclude_similar_characters: bool,

    /// Whether the password rules are strict.
    #[arg(short = 't', long, default_value_t = true)]
    pub strict: bool,
}

#[derive(Args)]
pub struct SecretGrepArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Whether the search string should be used as a regular expression
    #[arg(short, long)]
    pub regex: bool,

    /// The string to search in all secrets
    pub search_string: String,
}

#[derive(Args)]
pub struct SecretAddArgs {
    /// Toggle multiline edit mode
    #[arg(short, long)]
    pub multiline: bool,

    /// Overwrite an existing secrets without prompting
    #[arg(short, long)]
    pub force: bool,

    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Args)]
pub struct SecretListArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Toggle to display secrets as a tree
    #[arg(short, long)]
    pub tree: bool,
}

#[derive(Args)]
pub struct SecretMoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Overwrite an existing secrets without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The current path of the secret
    pub current_path: String,

    /// The new path of the secret
    pub new_path: String,
}

#[derive(Args)]
pub struct SecretRemoveArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Delete an existing secrets without prompting
    #[arg(short, long)]
    pub force: bool,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Args)]
pub struct SecretShowArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Toggle to display secrets as QR code
    #[arg(short = 'o', long)]
    pub qrcode: bool,

    /// Copy secret to clipboard
    #[arg(short, long)]
    pub clip: bool,

    /// Show only the specified line, or skip lines when given a negative number
    #[arg(short, long)]
    pub line: Option<isize>,

    /// The path of the secret within the selected store
    pub secret_path: String,
}

#[derive(Subcommand)]
pub enum StoreCommands {
    /// Adds a new store
    Add(StoreAddArgs),

    /// Remove an existing store
    Remove(StoreRemoveArgs),

    /// Mark a store as default
    SetDefault(StoreSetDefaultArgs),

    /// Decrypt a store and print its content
    Decrypt(StoreDecryptArgs),

    /// List all available stores
    List(StoreListArgs),

    /// Sets the synchronizer for a store
    SetSynchronizer(StoreSetSynchronizerArgs),

    /// Synchronizes the store with its remote counterpart
    Sync(StoreSyncArgs),

    /// Executes a command inside the directory of a store
    Exec(StoreExecArgs),
}

#[derive(Args)]
pub struct StoreAddArgs {
    /// The path on your local system for the new store
    #[arg(short, long, value_hint = DirPath)]
    pub path: PathBuf,

    /// The name for the new store
    #[arg(short, long)]
    pub name: String,

    /// The synchronizer to use
    #[arg(long, default_value_t, value_enum)]
    pub synchronizer: Synchronizers,

    /// Whether the new store should be the default store
    #[arg(short, long)]
    pub default: bool,
}

#[derive(Args)]
pub struct StoreRemoveArgs {
    /// Optional name of store to use. Defaults to the default store or the
    /// first one defined in the local user configuration
    #[arg(add = completer::store_name(), value_parser = parser::store_name)]
    pub store: Option<String>,

    /// Whether the store should be removed from the local file system
    #[arg(short, long)]
    pub remove_data: bool,
}

#[derive(Args)]
pub struct StoreSetDefaultArgs {
    /// The name of the store to use as default
    #[arg(value_parser = parser::store_name)]
    pub name: String,
}

#[derive(Args)]
pub struct StoreSetSynchronizerArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The synchronizer to use
    #[arg(value_enum)]
    pub synchronizer: Synchronizers,
}

#[derive(Args)]
pub struct StoreSyncArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// Toggle whether changes from the remote store should be pulled
    #[arg(long, default_missing_value="true", default_value("false"), num_args=0..=1)]
    pub pull: Option<bool>,

    /// Toggle whether local changes should be pushed to the remote store
    #[arg(long, default_missing_value="true", default_value("false"), num_args=0..=1)]
    pub push: Option<bool>,
}

#[derive(Args)]
pub struct StoreExecArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,

    /// The command to execute inside the store
    #[arg(num_args=0..)]
    pub command: Vec<String>,
}

#[derive(Args)]
pub struct StoreDecryptArgs {
    #[command(flatten)]
    pub store_selection: StoreSelectionArgs,
}

#[derive(Args)]
pub struct StoreListArgs {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
