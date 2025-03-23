// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;
use std::path::PathBuf;

use clap::error::ErrorKind;

use crate::adapters::file_system;
use crate::cli::{errors, logs};
use crate::models::configuration::Store;
use crate::recipients::{files, format, upsert};

pub fn add(
    store: &Store,
    public_key: &String,
    name: Option<&String>,
    path_to_secret: Option<&PathBuf>,
) -> anyhow::Result<()> {
    let path_is_directory = path_to_secret.as_ref().is_none_or(|path| if store.secret_at_path_exists(path) {
        let absolute_path_to_secret_directory = store.resolve_path(path);
        absolute_path_to_secret_directory.is_dir()
    } else {
        errors::error_exit(
            "recipient",
            "add",
            ErrorKind::InvalidValue,
            &format!("invalid value '{}' for '--path <PATH>': path does not match any secret or folder in the store", path.display()));
    });

    let recipients_file = files::for_secret_path(path_to_secret, path_is_directory);
    let absolute_path_to_recipients_file = store.resolve_path(&recipients_file);

    if absolute_path_to_recipients_file.is_file() {
        // update existing .recipients file
        let recipients = fs::read_to_string(&absolute_path_to_recipients_file)?;
        let (updated_recipients, _) = upsert::recipient(recipients, public_key, name);
        fs::write(&absolute_path_to_recipients_file, updated_recipients)?;
    } else {
        // create new .recipients file
        let recipient = format::recipient(public_key, name);
        file_system::append_file(&absolute_path_to_recipients_file, &recipient)?;
        logs::recipient_added(public_key);
    }

    store
        .vcs
        .select_implementation(PathBuf::from(&store.path))
        .commit(
            vec![&recipients_file],
            &format!("Added recipient '{public_key}'"),
        )?;
    Ok(())
}
