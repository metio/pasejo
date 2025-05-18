// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::Path;

use crate::synchronizers::synchronizer::Synchronizers;
use log::{debug, info, warn};

pub fn recipient_added(public_key: &str) {
    info!("Recipient for '{public_key}' added");
}

pub fn recipient_removed(public_key: &str) {
    info!("Recipient for '{public_key}' removed");
}

pub fn secret_inserted(secret_path: &str) {
    info!("Inserted secret at '{secret_path}'");
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

pub fn store_set_synchronizer(store_name: &str, synchronizer: &Synchronizers) {
    info!("Store '{store_name}' now synchronizes with {synchronizer:?}");
}

pub fn store_remove_success(store_name: &str) {
    info!("Store '{store_name}' removed");
}

pub fn store_sync_start(store_name: &str) {
    debug!("Pulling changes from remote for store '{store_name}'");
}

pub fn recipient_does_not_exist_ignored(public_key: &str) {
    info!("Recipient for '{public_key}' does not exist in store - ignoring");
}

pub fn no_identities_exist_yet(store_name: &String) {
    warn!(
        "There are no identities in the store '{store_name}' yet. Please add one using 'pasejo identity add ...'"
    );
}
