// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use log::{debug, error, info, warn};
use std::path::Path;
use std::time::Duration;

pub fn recipient_added(public_key: &str) {
    info!("Recipient for '{public_key}' added");
}

pub fn recipient_removed(public_key: &str) {
    info!("Recipient for '{public_key}' removed");
}

pub fn secret_added(secret_path: &str) {
    info!("Added secret at '{secret_path}'");
}

pub fn secret_edited(secret_path: &str) {
    info!("Edited secret at '{secret_path}'");
}

pub fn one_time_password_added(password_path: &str) {
    info!("Added one-time password at '{password_path}'");
}

pub fn one_time_password_copied(source_path: &str, target_path: &str) {
    info!("Copied one-time password from '{source_path}' to '{target_path}'");
}

pub fn one_time_password_moved(source_path: &str, target_path: &str) {
    info!("Moved one-time password from '{source_path}' to '{target_path}'");
}

pub fn one_time_password_removed(password_path: &str) {
    info!("Removed one-time password at '{password_path}'");
}

pub fn secret_generated(secret_path: &str) {
    info!("Generated secret at '{secret_path}'");
}

pub fn secret_show_as_qrcode(secret_path: &str) {
    debug!("Showing secret at '{secret_path}' as QR code");
}

pub fn secret_show_as_text(secret_path: &str) {
    debug!("Showing secret at '{secret_path}' as text");
}

pub fn secret_copy_into_clipboard(secret_path: &str, duration: &Duration) {
    info!(
        "Copying secret at '{secret_path}' into clipboard. Will be removed after {duration:?} or once you terminate this process."
    );
}

pub fn one_time_password_show(password_path: &str) {
    debug!("Showing one-time password at '{password_path}'");
}

pub fn identity_added(identity_file: &Path) {
    info!("Identity using file '{}' added", identity_file.display());
}

pub fn identity_removed(identity_file: &Path) {
    info!("Identity using file '{}' removed", identity_file.display());
}

pub fn store_add_success(store_name: &str, store_path: &str) {
    info!("Store '{store_name}' added at '{store_path}'");
}

pub fn store_set_default(store_name: &str) {
    info!("Store '{store_name}' is now the default");
}

pub fn store_remove_success(store_name: &str) {
    info!("Store '{store_name}' removed");
}

pub fn execute_pull_hooks(store_name: &str) {
    debug!("Executing pull hooks for store '{store_name}'");
}

pub fn execute_push_hooks(store_name: &str) {
    debug!("Executing push hooks for store '{store_name}'");
}

pub fn recipient_does_not_exist_ignored(public_key: &str) {
    info!("Recipient for '{public_key}' does not exist in store - ignoring");
}

pub fn no_identities_exist_yet(store_name: &String) {
    warn!(
        "There are no identities in the store '{store_name}' yet. Please add one using 'pasejo identity add ...'"
    );
}

pub fn merge_conflict_recipient_names(
    public_key: &String,
    first_name: &String,
    second_name: &String,
) {
    error!(
        "Merge conflict for recipient with public key '{public_key}': names '{first_name}' and '{second_name}' differ",
    );
}

pub fn merge_conflict_recipient_removed_and_renamed(public_key: &String, new_name: &String) {
    error!(
        "Merge conflict for recipient with public key '{public_key}': recipient was removed in one version and renamed to '{new_name}' in the other"
    );
}

pub fn merge_conflict_values(value_type: &str, secret_path: &str) {
    error!("Merge conflict for {value_type} at '{secret_path}': values differ in the two versions");
}

pub fn merge_conflict_removed_and_modified(value_type: &str, secret_path: &str) {
    error!(
        "Merge conflict for {value_type} at '{secret_path}': {value_type} was removed in one version and modified in the other"
    );
}

pub fn secret_copied(source_path: &str, target_path: &str) {
    info!("Copied secret from '{source_path}' to '{target_path}'");
}

pub fn secret_moved(source_path: &str, target_path: &str) {
    info!("Moved secret from '{source_path}' to '{target_path}'");
}

pub fn secret_removed(secret_path: &str) {
    info!("Removed secret at '{secret_path}'");
}
