// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs::{File, TryLockError};
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::cli::i18n;
use crate::hooks::executor::HookExecutor;
use crate::models::configuration::{Configuration, StoreRegistration, encrypt_store};
use crate::models::password_store::PasswordStore;

/// Selects the store for an operation, distinguishing "the requested or
/// default store name matches no registration" from "you have no stores
/// configured at all" — the two cases the bare `select_store` `Option`
/// collapses into one unhelpful "run `pasejo store add`" message even when
/// the user merely typo'd an existing store's name. Mirrors the host's
/// `unknown_store` vs `no_store_configured` split so the CLI and the browser
/// host agree.
pub fn require_store<'a>(
    configuration: &'a Configuration,
    store_name: Option<&String>,
) -> Result<&'a StoreRegistration> {
    if let Some(registration) = configuration.select_store(store_name) {
        return Ok(registration);
    }
    configuration.missing_store_name(store_name).map_or_else(
        || Err(anyhow::anyhow!(i18n::error_no_store_in_configuration())),
        |name| Err(anyhow::anyhow!(i18n::error_unknown_store(&name))),
    )
}

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
///
/// Every encrypt renames a fresh file over `path`, so a waiter that opened
/// the previous file can be granted a lock on an inode `path` no longer
/// names, while a newcomer locks the current one. The lock therefore only
/// counts once the locked handle is still the file at `path`; otherwise it
/// is dropped and the open is retried.
pub fn acquire_store_lock(path: &Path) -> Result<Option<File>> {
    lock_store_file(path, |file, path_display| {
        file.lock()
            .with_context(|| i18n::error_could_not_acquire_store_lock(path_display))
    })
}

/// [`acquire_store_lock`] for callers that must not wait on another
/// process indefinitely: a lock still held after `wait` is an error.
pub fn try_acquire_store_lock(path: &Path, wait: Duration) -> Result<Option<File>> {
    let deadline = Instant::now() + wait;
    lock_store_file(path, |file, path_display| {
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(()),
                Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(LOCK_RETRY_INTERVAL);
                }
                Err(TryLockError::WouldBlock) => {
                    anyhow::bail!(i18n::error_store_lock_wait_timed_out(path_display));
                }
                Err(TryLockError::Error(error)) => {
                    return Err(error)
                        .with_context(|| i18n::error_could_not_acquire_store_lock(path_display));
                }
            }
        }
    })
}

const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(50);

fn lock_store_file(path: &Path, lock: impl Fn(&File, &str) -> Result<()>) -> Result<Option<File>> {
    let path_display = path.display().to_string();
    loop {
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| i18n::error_could_not_open_store_for_lock(&path_display));
            }
        };
        lock(&file, &path_display)?;
        if is_file_at(&file, path)
            .with_context(|| i18n::error_could_not_open_store_for_lock(&path_display))?
        {
            return Ok(Some(file));
        }
    }
}

fn is_file_at(file: &File, path: &Path) -> io::Result<bool> {
    let current = match same_file::Handle::from_path(path) {
        Ok(current) => current,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(same_file::Handle::from_file(file.try_clone()?)? == current)
}

/// Whether the closure passed to [`with_store`] modified the password store.
/// `Modified` triggers re-encryption and the push hooks; `Unchanged` skips
/// both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreMutation {
    Unchanged,
    Modified,
}

/// A store opened for an operation. Produced by [`open_store`], the shared
/// front half of every store flow: the selected `registration`, a throttled
/// `hooks` executor, the held advisory `lock` (`None` only on the bootstrap
/// path where the file does not exist yet), and the decrypted `store`. The
/// caller continues under the lock and decides when to encrypt, drop the
/// lock, and run push hooks.
pub struct OpenStore<'a> {
    pub registration: &'a StoreRegistration,
    pub hooks: HookExecutor<'a>,
    pub lock: Option<File>,
    pub store: PasswordStore,
}

/// The shared opening sequence — require a store, run pull hooks (throttled),
/// take the advisory file lock, decrypt. This is the front half of
/// [`with_store_then`]; commands whose tail differs from the standard
/// `mutate → encrypt → push` (e.g. OTP `show`, which must persist the bumped
/// HOTP counter only *after* delivering the code) call this directly and
/// hand-roll only their tail, so the select/pull/lock/decrypt order has one
/// definition instead of drifting copies.
pub fn open_store<'a>(
    configuration: &'a Configuration,
    store_name: Option<&String>,
    offline: bool,
) -> Result<OpenStore<'a>> {
    let registration = require_store(configuration, store_name)?;
    let hooks = HookExecutor::new(configuration, registration, offline);
    hooks.execute_pull_commands()?;
    let lock = acquire_store_lock(registration.path())?;
    let store = configuration.decrypt_store(registration)?;
    Ok(OpenStore {
        registration,
        hooks,
        lock,
        store,
    })
}

/// Run `f` against the selected store, handling the full lifecycle:
/// select store → pull hooks → decrypt → run `f` → (encrypt + push hooks if
/// `f` returned [`StoreMutation::Modified`]). Returns the closure's value.
///
/// Bails with the localized "no store" error when no store can be selected.
pub fn with_store<F, T>(
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
pub fn with_store_then<F, P, T>(
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
    // The lock acquired by `open_store` is held across decrypt → mutate →
    // encrypt so concurrent invocations cannot both read the same store
    // state, both mutate, and have the later writer's encrypt clobber the
    // earlier writer's changes (most visibly: a HOTP counter advancing by
    // one instead of two for two parallel `otp show` calls). It is released
    // before push hooks so long-running network syncs do not serialize
    // unnecessarily.
    let OpenStore {
        registration,
        hooks,
        lock,
        mut store,
    } = open_store(configuration, store_name, offline)?;
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
    use std::sync::mpsc;
    use std::time::Duration;

    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    fn registration(name: &str) -> StoreRegistration {
        StoreRegistration {
            path: std::path::PathBuf::from(format!("/tmp/{name}")),
            name: name.to_string(),
            identities: vec![],
            pull_commands: vec![],
            push_commands: vec![],
        }
    }

    fn config_with(stores: Vec<StoreRegistration>) -> Configuration {
        Configuration {
            stores,
            ..Configuration::default()
        }
    }

    #[test]
    fn require_store_returns_the_named_store_when_it_exists() {
        crate::cli::i18n::init_for_tests();
        let cfg = config_with(vec![registration("alpha"), registration("beta")]);
        let got = require_store(&cfg, Some(&String::from("beta"))).unwrap();
        assert_eq!(got.name, "beta");
    }

    #[test]
    fn require_store_falls_back_to_the_first_store_when_no_name_is_given() {
        crate::cli::i18n::init_for_tests();
        let cfg = config_with(vec![registration("alpha"), registration("beta")]);
        let got = require_store(&cfg, None).unwrap();
        assert_eq!(got.name, "alpha");
    }

    #[test]
    fn require_store_names_the_unknown_store_rather_than_claiming_none_configured() {
        // The whole reason `require_store` exists: a typo against a populated
        // config must report the unknown name, not collapse into the
        // empty-config "run `store add`" message.
        crate::cli::i18n::init_for_tests();
        let cfg = config_with(vec![registration("alpha")]);
        let message = require_store(&cfg, Some(&String::from("alfa")))
            .unwrap_err()
            .to_string();
        assert!(message.contains("alfa"), "should name the typo: {message}");
        assert!(
            !message.contains("store add"),
            "should not be the empty-config message: {message}"
        );
    }

    #[test]
    fn require_store_names_an_unregistered_default_store() {
        if std::env::var_os(crate::cli::environment_variables::PASEJO_DEFAULT_STORE).is_some() {
            return;
        }
        crate::cli::i18n::init_for_tests();
        let cfg = Configuration {
            default_store: Some(String::from("gone")),
            ..config_with(vec![registration("alpha")])
        };
        let message = require_store(&cfg, None).unwrap_err().to_string();
        assert!(
            message.contains("gone"),
            "should name the default: {message}"
        );
        assert!(
            !message.contains("store add"),
            "should not be the empty-config message: {message}"
        );
    }

    #[test]
    fn require_store_reports_no_store_when_the_configuration_is_empty() {
        crate::cli::i18n::init_for_tests();
        let cfg = config_with(vec![]);
        let message = require_store(&cfg, None).unwrap_err().to_string();
        assert!(
            message.contains("No store found"),
            "expected the empty-config message: {message}"
        );
    }

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
        let worker = std::thread::spawn(move || {
            let second = acquire_store_lock(&path).unwrap();
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

    #[test]
    fn acquire_store_lock_serializes_a_waiter_and_a_newcomer_after_the_file_is_replaced() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("store");
        file.write_str("payload").unwrap();
        let path = file.path().to_path_buf();

        let guard_a = acquire_store_lock(&path).unwrap();

        let (b_acquired_tx, b_acquired_rx) = mpsc::channel();
        let (b_release_tx, b_release_rx) = mpsc::channel::<()>();
        let b_path = path.clone();
        let waiter_b = std::thread::spawn(move || {
            let guard_b = acquire_store_lock(&b_path).unwrap();
            b_acquired_tx.send(()).unwrap();
            b_release_rx.recv().unwrap();
            drop(guard_b);
        });
        // Give B time to open the original file and block on its lock.
        std::thread::sleep(Duration::from_millis(100));

        crate::cli::atomic_write::write(&path, b"replaced").unwrap();
        drop(guard_a);
        b_acquired_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("B should acquire once A releases");

        let (c_acquired_tx, c_acquired_rx) = mpsc::channel();
        let newcomer_c = std::thread::spawn(move || {
            let guard_c = acquire_store_lock(&path).unwrap();
            c_acquired_tx.send(()).unwrap();
            drop(guard_c);
        });

        assert!(
            c_acquired_rx
                .recv_timeout(Duration::from_millis(150))
                .is_err(),
            "C acquired the lock while B still held it"
        );

        b_release_tx.send(()).unwrap();
        waiter_b.join().unwrap();
        c_acquired_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("C should acquire once B releases");
        newcomer_c.join().unwrap();
    }
}
