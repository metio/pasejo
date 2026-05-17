// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::i18n;
use crate::hooks::executor::HookExecutor;
use crate::models::configuration::{Configuration, StoreRegistration, encrypt_store};
use crate::models::password_store::PasswordStore;
use anyhow::{Context, Result};
use std::fs::File;
use std::io;
use std::path::Path;

/// Acquires an exclusive advisory lock on the store file at `path`. The
/// returned `File` holds the lock; dropping it releases the lock. Returns
/// `Ok(None)` when the store file does not yet exist — that is the
/// bootstrap path (a freshly registered store before its first encrypt),
/// where there is no file content to race over.
///
/// This is the serialization primitive for the `decrypt → mutate → encrypt`
/// window. Holding it across that window keeps concurrent invocations from
/// both reading the same HOTP counter, both bumping it, and losing one of
/// the increments when re-encrypting.
fn acquire_store_lock(path: &Path) -> Result<Option<File>> {
    let path_display = path.display().to_string();
    match File::open(path) {
        Ok(file) => {
            file.lock()
                .with_context(|| i18n::error_could_not_acquire_store_lock(&path_display))?;
            Ok(Some(file))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| i18n::error_could_not_open_store_for_lock(&path_display))
        }
    }
}

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
/// Bails with the localized "no store" error when no store can be selected.
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
        .context(i18n::error_no_store_in_configuration())?;
    let hooks = HookExecutor {
        configuration,
        registration,
        offline,
        force: false,
    };
    hooks.execute_pull_commands()?;
    // Hold an exclusive advisory lock on the store file across decrypt →
    // mutate → encrypt so concurrent invocations cannot both read the same
    // store state, both mutate, and have the later writer's encrypt clobber
    // the earlier writer's changes (most visibly: a HOTP counter advancing
    // by one instead of two for two parallel `otp show` calls). The lock is
    // released before push hooks so long-running network syncs do not
    // serialize unnecessarily.
    let lock = acquire_store_lock(registration.path())?;
    let mut store = configuration.decrypt_store(registration)?;
    let (value, mutation) = f(registration, &mut store)?;
    if matches!(mutation, StoreMutation::Modified) {
        encrypt_store(registration, &store).context(i18n::error_cannot_encrypt_store())?;
    }
    drop(lock);
    then(&value)?;
    if matches!(mutation, StoreMutation::Modified) {
        hooks.execute_push_commands()?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn acquire_store_lock_returns_some_for_existing_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();

        let lock = acquire_store_lock(file.path()).unwrap();
        assert!(lock.is_some(), "existing file should yield a held lock");
        drop(lock);
    }

    #[test]
    fn acquire_store_lock_returns_none_for_missing_file() {
        // A freshly registered store before its first encrypt does not yet
        // have a file on disk. The lock acquisition must succeed-with-None
        // so the caller can proceed to the existing "no file" error path
        // instead of being short-circuited by the lock layer.
        let temp = TempDir::new().unwrap();
        let missing = temp.child("never-existed");

        let lock = acquire_store_lock(missing.path()).unwrap();
        assert!(lock.is_none());
    }

    #[test]
    fn acquire_store_lock_is_reacquirable_after_drop() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();

        let first = acquire_store_lock(file.path()).unwrap();
        assert!(first.is_some());
        drop(first);
        // Re-acquisition on the same path must succeed once the previous
        // guard has been dropped.
        let second = acquire_store_lock(file.path()).unwrap();
        assert!(second.is_some());
    }

    #[test]
    fn acquire_store_lock_blocks_concurrent_acquisition_until_first_drops() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();
        let path = file.path().to_path_buf();

        let guard = acquire_store_lock(&path).unwrap();
        assert!(guard.is_some());

        let (tx, rx) = mpsc::channel();
        let worker_path = path.clone();
        let worker = std::thread::spawn(move || {
            let second = acquire_store_lock(&worker_path).unwrap();
            assert!(second.is_some());
            // Signal the moment the worker acquires the lock.
            tx.send(()).unwrap();
            // Hold the lock until the test verifies the signal, then drop.
            drop(second);
        });

        // The worker must NOT have signalled within 150ms — proving its
        // `acquire_store_lock` call is blocked on the first guard.
        assert!(
            rx.recv_timeout(Duration::from_millis(150)).is_err(),
            "second acquisition completed while the first guard was still held"
        );

        // Releasing the first guard must unblock the worker.
        drop(guard);
        rx.recv_timeout(Duration::from_secs(2))
            .expect("second acquisition should succeed within 2s of first being released");
        worker.join().unwrap();
    }
}
