// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{absolute, Path};

use clap::error::ErrorKind;

use crate::cli::errors::error_exit;
use crate::cli::logs;
use crate::models::cli::IdentityCommands;
use crate::models::configuration::{Configuration, Identity};

pub fn dispatch(command: &IdentityCommands, configuration: Configuration) -> anyhow::Result<()> {
    match command {
        IdentityCommands::Add(args) => add(
            configuration,
            args.store_selection.store.as_ref(),
            args.file.as_path(),
            args.global,
        ),
        IdentityCommands::Remove(args) => remove(
            configuration,
            args.store_selection.store.as_ref(),
            args.file.as_path(),
            args.global,
            args.ignore_unknown,
        ),
        IdentityCommands::List(args) => list(
            &configuration,
            args.store_selection.store.as_ref(),
            args.global,
        ),
    }
}

fn add(
    mut configuration: Configuration,
    store_name: Option<&String>,
    identity_file: &Path,
    global: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.clone(),
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
        if global {
            configuration.identities.push(identity);
            configuration.save_configuration()?;
        } else if let Some(store) = configuration.select_store_mut(store_name) {
            store.identities.push(identity);
            configuration.save_configuration()?;
        } else {
            anyhow::bail!(
                "Cannot identify store. Set a default store, use --store to specify a store or use --global to set the identity globally."
            );
        }

        logs::identity_added(absolute_path.as_path());
        Ok(())
    }
}

fn remove(
    mut configuration: Configuration,
    store_name: Option<&String>,
    identity_file: &Path,
    global: bool,
    ignore_missing: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.clone(),
    };
    if configuration.has_identity(&identity, store_name, global) {
        configuration.remove_identity(&identity, store_name, global)?;
        logs::identity_removed(absolute_path.as_path());
        Ok(())
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

fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    global: bool,
) -> anyhow::Result<()> {
    if global {
        for identity in &configuration.identities {
            println!("global: {}", identity.file.display());
        }
        Ok(())
    } else if let Some(registration) = configuration.select_store(store_name) {
        for identity in &configuration.identities {
            println!("global: {}", identity.file.display());
        }
        for identity in &registration.identities {
            println!("store: {}", identity.file.display());
        }

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration and no --global flag specified. Run 'pasejo store add ...' first to add one"
        )
    }
}
