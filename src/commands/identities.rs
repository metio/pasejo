use std::path::{absolute, PathBuf};

use clap::error::ErrorKind;

use crate::cli::errors::error_exit;
use crate::cli::printer;
use crate::models::configuration::{Configuration, Identity};

pub fn add(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
    global: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    if configuration.has_identity(&identity, store_name, global) {
        error_exit(
            "identity",
            "add",
            ErrorKind::InvalidValue,
            &format!(
                "invalid value '{}' for '--file <FILE>': file was already added as an identity",
                identity_file.display()
            ),
        )
    } else {
        let result = configuration.add_identity(identity, store_name, global);
        printer::identity_added();
        result
    }
}

pub fn remove(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
    global: bool,
    ignore_missing: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    if configuration.has_identity(&identity, store_name, global) {
        let result = configuration.remove_identity(&identity, store_name, global);
        printer::identity_removed();
        result
    } else if ignore_missing {
        Ok(())
    } else {
        error_exit(
            "identity",
            "remove",
            ErrorKind::InvalidValue,
            &format!(
                "invalid value '{}' for '--file <FILE>': file does not match any known identity",
                identity_file.display()
            ),
        )
    }
}
