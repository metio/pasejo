// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::hooks::executor::HookExecutor;
use crate::models::configuration::{Configuration, StoreRegistration};
use crate::models::password_store::PasswordStore;
use anyhow::{Context, Result};

pub(super) const NO_STORE_FOUND_ERROR: &str =
    "No store found in configuration. Run 'pasejo store add ...' first to add one";

/// Whether the closure passed to [`with_store`] modified the password store.
/// `Modified` triggers re-encryption and the push hooks; `Unchanged` skips
/// both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StoreMutation {
    Unchanged,
    Modified,
}

/// Run `f` against the selected store, handling the full lifecycle:
/// select store → pull hooks → decrypt → run `f` → (encrypt + push hooks if
/// `f` returned [`StoreMutation::Modified`]). Returns the closure's value.
///
/// Bails with [`NO_STORE_FOUND_ERROR`] when no store can be selected.
pub(super) fn with_store<F, T>(
    configuration: &Configuration,
    store_name: Option<&String>,
    offline: bool,
    f: F,
) -> Result<T>
where
    F: FnOnce(&StoreRegistration, &mut PasswordStore) -> Result<(T, StoreMutation)>,
{
    with_store_then(configuration, store_name, offline, f, |_| Ok(()))
}

/// Like [`with_store`], but runs `then` after encryption and *before* the
/// push hooks. This preserves the user-visible log order for commands that
/// need to perform an action (display a secret, copy to clipboard, …) using
/// data extracted from the store: pull-hook log → encrypt → action → push-hook
/// log.
///
/// `then` always runs, regardless of [`StoreMutation`]. For read-only flows
/// it executes after the closure with no encrypt/push around it.
pub(super) fn with_store_then<F, P, T>(
    configuration: &Configuration,
    store_name: Option<&String>,
    offline: bool,
    f: F,
    then: P,
) -> Result<T>
where
    F: FnOnce(&StoreRegistration, &mut PasswordStore) -> Result<(T, StoreMutation)>,
    P: FnOnce(&T) -> Result<()>,
{
    let registration = configuration
        .select_store(store_name)
        .context(NO_STORE_FOUND_ERROR)?;
    let hooks = HookExecutor {
        configuration,
        registration,
        offline,
        force: false,
    };
    hooks.execute_pull_commands()?;
    let mut store = configuration.decrypt_store(registration)?;
    let (value, mutation) = f(registration, &mut store)?;
    if matches!(mutation, StoreMutation::Modified) {
        Configuration::encrypt_store(registration, &store).context("Cannot encrypt store")?;
    }
    then(&value)?;
    if matches!(mutation, StoreMutation::Modified) {
        hooks.execute_push_commands()?;
    }
    Ok(value)
}
