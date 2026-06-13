// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fmt::Write as _;
use std::path::{Path, PathBuf, absolute};

use clap::error::ErrorKind;

use crate::cli::errors::error_exit;
use crate::cli::i18n;
use crate::models::cli::IdentityCommands;
use crate::models::configuration::{Configuration, Identity};

pub fn dispatch(command: &IdentityCommands, configuration: &Configuration) -> anyhow::Result<()> {
    match command {
        IdentityCommands::Add(args) => {
            let mut owned = configuration.clone();
            add(
                &mut owned,
                args.store_selection.store.as_ref(),
                args.file.as_path(),
                args.global,
            )
        }
        IdentityCommands::Remove(args) => {
            let mut owned = configuration.clone();
            remove(
                &mut owned,
                args.store_selection.store.as_ref(),
                args.file.as_path(),
                args.global,
                args.ignore_unknown,
            )
        }
        IdentityCommands::List(args) => {
            list(
                configuration,
                args.store_selection.store.as_ref(),
                args.global,
            );
            Ok(())
        }
    }
}

fn add(
    configuration: &mut Configuration,
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
            &i18n::error_identity_file_already_added(&identity_file.display().to_string()),
        )
    } else {
        if global {
            configuration.identities.push(identity);
            configuration.save_configuration()?;
        } else if let Some(store) = configuration.select_store_mut(store_name) {
            store.identities.push(identity);
            configuration.save_configuration()?;
        } else {
            anyhow::bail!(i18n::error_cannot_identify_store());
        }

        i18n::identity_added(absolute_path.as_path());
        Ok(())
    }
}

fn remove(
    configuration: &mut Configuration,
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
        i18n::identity_removed(absolute_path.as_path());
        Ok(())
    } else if ignore_missing {
        Ok(())
    } else {
        let mut message = i18n::error_identity_file_not_known(&identity_file.display().to_string());
        // Identity paths are matched verbatim. If a registration in this
        // scope resolves to the same file under a different spelling, name
        // it so the user isn't left guessing which form was stored.
        if let Some(stored) =
            suggest_registered_spelling(configuration, store_name, global, identity_file)
        {
            let _ = write!(
                message,
                ". {}",
                i18n::error_identity_did_you_mean(&stored.display().to_string())
            );
        }
        error_exit("identity", "remove", ErrorKind::InvalidValue, &message)
    }
}

/// Best-effort suggestion for a `remove` that found no exact match.
///
/// Because identity paths are matched verbatim, a key registered under one
/// spelling cannot be removed under an equivalent-but-different one (a
/// symlink, a `..` form). When the supplied file exists on disk, resolve
/// it and the in-scope registrations to their real paths and return the
/// stored spelling that points at the same file. Paths that do not exist
/// are skipped — this only reads to build a hint and never changes which
/// identities actually match.
fn suggest_registered_spelling(
    configuration: &Configuration,
    store_name: Option<&String>,
    global: bool,
    given: &Path,
) -> Option<PathBuf> {
    let target = std::fs::canonicalize(given).ok()?;
    let candidates: &[Identity] = if global {
        configuration.identities.as_slice()
    } else {
        configuration
            .select_store(store_name)
            .map_or(&[], |registration| registration.identities.as_slice())
    };
    candidates
        .iter()
        .find(|identity| {
            std::fs::canonicalize(&identity.file).is_ok_and(|resolved| resolved == target)
        })
        .map(|identity| identity.file.clone())
}

fn list(configuration: &Configuration, store_name: Option<&String>, global: bool) {
    // Global identities apply to every store, so always list them — even
    // with no store configured yet, where `identity add --global` is
    // valid. Listing is read-only, so "nothing to show" is an empty
    // result and exit 0, never the "no store or --global" error that the
    // mutating `add`/`remove` paths raise when they need a target scope.
    for identity in &configuration.identities {
        i18n::list_global_identity(&identity.file);
    }
    if global {
        return;
    }
    if let Some(registration) = configuration.select_store(store_name) {
        for identity in &registration.identities {
            // A file registered both globally and on this store is listed
            // under each scope. The store line is flagged so the redundancy
            // is visible — a global identity already covers every store, so
            // the per-store copy adds nothing but is harmless (decrypt
            // dedups via `all_identity_files`).
            if configuration
                .identities
                .iter()
                .any(|global_identity| global_identity.file == identity.file)
            {
                i18n::list_store_identity_also_global(&identity.file);
            } else {
                i18n::list_store_identity(&identity.file);
            }
        }
    }
}
