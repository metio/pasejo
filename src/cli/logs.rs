// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::Path;

use log::info;

pub fn recipient_added(public_key: &str) {
    info!("Recipient for '{public_key}' added");
}

pub fn recipient_updated(public_key: &str) {
    info!("Recipient for '{public_key}' updated");
}

pub fn recipients_file_created(recipients_file_path: &Path) {
    info!(
        "Created .recipients file at '{}'",
        recipients_file_path.display()
    );
}

pub fn recipients_file_replaced(recipients_file_path: &Path) {
    info!(
        "Replaced .recipients file at '{}'",
        recipients_file_path.display()
    );
}

pub fn recipients_file_use_existing(recipients_file_path: &Path) {
    info!(
        "Using existing .recipients file at '{}'",
        recipients_file_path.display()
    );
}

pub fn identity_added(identity_file: &Path) {
    info!("Identity using file '{}' added", identity_file.display());
}

pub fn identity_removed(identity_file: &Path) {
    info!("Identity using file '{}' removed", identity_file.display());
}

pub fn store_initialized(store_path: &str) {
    info!("Store initialized at '{store_path}'");
}

pub fn store_set_default(store_name: &str) {
    info!("Store '{store_name}' is now the default");
}

pub fn store_removed(store_name: &str) {
    info!("Store '{store_name}' removed");
}
