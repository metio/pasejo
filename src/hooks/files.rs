// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use directories::BaseDirs;

use crate::cli::{atomic_write, environment_variables};
use crate::models::configuration;

/// Decides whether a throttled hook is due to run. PURE: it never writes
/// the throttle marker — the caller advances the marker via
/// [`write_last_execution`] only AFTER the hook command actually
/// succeeds, so a failing hook doesn't bump the timestamp and silently
/// suppress the retry for the rest of the interval. `paths == None`
/// means throttling is disabled (`PASEJO_DISABLE_HOOK_THROTTLING`) and
/// the hook always runs.
pub fn should_execute(
    interval_seconds: Option<u64>,
    paths: Option<(PathBuf, PathBuf)>,
) -> anyhow::Result<bool> {
    let Some((_last_directory, last_file)) = paths else {
        // Throttling disabled — always run.
        return Ok(true);
    };
    // A missing marker is the first run for this store. An unparseable one
    // is treated as due too, so the next successful run overwrites it
    // instead of every operation on the store failing.
    let last_seconds = match fs::read_to_string(&last_file) {
        Ok(contents) => contents.trim().parse::<u64>().ok(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let Some(last_seconds) = last_seconds else {
        return Ok(true);
    };
    Ok(is_due(
        interval_seconds,
        last_seconds,
        current_epoch_seconds()?,
    ))
}

/// The pure throttle decision, factored out of the file/clock plumbing so
/// the boundary behaviour is directly testable. Given the configured
/// `interval_seconds` (`None` → the 24h default), the marker's timestamp,
/// and the current time (all epoch seconds), the hook is due once *strictly
/// more* than one interval has elapsed. Saturating subtraction tolerates a
/// marker dated ahead of `now` (clock skew, NTP rollback, or a marker file
/// copied from a faster machine) without underflowing.
fn is_due(interval_seconds: Option<u64>, last_seconds: u64, now_seconds: u64) -> bool {
    let elapsed = now_seconds.saturating_sub(last_seconds);
    let interval = interval_seconds.unwrap_or(configuration::DEFAULT_HOOK_INTERVAL_SECONDS);
    elapsed > interval
}

pub fn write_last_execution(paths: Option<(PathBuf, PathBuf)>) -> anyhow::Result<()> {
    if let Some((_last_directory, last_file)) = paths {
        atomic_write::write(&last_file, current_epoch_seconds()?.to_string().as_bytes())
    } else {
        Ok(())
    }
}

/// Wall-clock seconds since the Unix epoch — the one place the throttle
/// logic reads the clock, so the pure decision in [`is_due`] can be tested
/// with explicit timestamps.
fn current_epoch_seconds() -> anyhow::Result<u64> {
    Ok(SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs())
}

pub fn last_pull_paths(store_name: &OsStr) -> Option<(PathBuf, PathBuf)> {
    last_paths(store_name, "last-pull")
}

pub fn last_push_paths(store_name: &OsStr) -> Option<(PathBuf, PathBuf)> {
    last_paths(store_name, "last-push")
}

fn last_paths(store_name: &OsStr, directory: &str) -> Option<(PathBuf, PathBuf)> {
    if env::var_os(environment_variables::PASEJO_DISABLE_HOOK_THROTTLING).is_some() {
        return None;
    }

    let data_local_dir: Option<PathBuf> = env::var_os(environment_variables::PASEJO_DATA_DIR)
        .map(PathBuf::from)
        .or_else(|| BaseDirs::new().map(|base_dirs| base_dirs.data_local_dir().to_path_buf()));

    data_local_dir.map(|dir| {
        let last_pulls_directory = dir.join(env!("CARGO_PKG_NAME")).join(directory);
        let last_pull_file = last_pulls_directory.join(store_name);
        (last_pulls_directory, last_pull_file)
    })
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    fn now_seconds() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn paths_in(temp: &TempDir) -> (PathBuf, PathBuf) {
        let dir = temp.child("dir");
        let file = dir.child("last");
        (dir.path().to_path_buf(), file.path().to_path_buf())
    }

    #[test]
    fn is_due_is_false_exactly_at_the_interval_boundary() {
        // Strictly-greater: elapsed == interval is NOT yet due.
        assert!(!is_due(Some(60), 1_000, 1_060));
    }

    #[test]
    fn is_due_is_true_one_second_past_the_interval() {
        assert!(is_due(Some(60), 1_000, 1_061));
    }

    #[test]
    fn is_due_tolerates_a_marker_dated_ahead_of_now() {
        // Saturating subtraction: a future-dated marker yields zero elapsed,
        // never an underflow, so the hook is not (spuriously) due.
        assert!(!is_due(Some(60), 5_000, 1_000));
    }

    #[test]
    fn is_due_uses_the_24h_default_when_interval_is_none() {
        let day = 60 * 60 * 24;
        assert!(!is_due(None, 0, day)); // exactly one day: not yet
        assert!(is_due(None, 0, day + 1)); // a second past a day: due
    }

    #[test]
    fn is_due_with_zero_interval_requires_at_least_one_second() {
        assert!(!is_due(Some(0), 1_000, 1_000)); // 0 elapsed is not > 0
        assert!(is_due(Some(0), 1_000, 1_001)); // 1s elapsed is > 0
    }

    #[test]
    fn should_execute_returns_true_when_no_paths_given() {
        assert!(should_execute(None, None).unwrap());
        assert!(should_execute(Some(60), None).unwrap());
    }

    #[test]
    fn should_execute_returns_true_when_marker_missing_without_creating_it() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);

        let executed = should_execute(None, Some((dir, file.clone()))).unwrap();

        assert!(executed);
        // `should_execute` is pure now: creating the marker is the caller's
        // job (`write_last_execution`, only after the hook succeeds).
        assert!(
            !file.exists(),
            "should_execute must not write the marker itself"
        );
    }

    #[test]
    fn should_execute_treats_an_empty_or_garbled_marker_as_due() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        for contents in ["", "garbage"] {
            std::fs::write(&file, contents).unwrap();
            assert!(
                should_execute(Some(60), Some((dir.clone(), file.clone()))).unwrap(),
                "marker {contents:?} should make the hook due"
            );
        }
    }

    #[test]
    fn concurrent_marker_writes_never_expose_a_partial_marker_to_readers() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        write_last_execution(Some((dir.clone(), file.clone()))).unwrap();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writer = {
            let (dir, file, stop) = (dir.clone(), file.clone(), stop.clone());
            std::thread::spawn(move || {
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    write_last_execution(Some((dir.clone(), file.clone()))).unwrap();
                }
            })
        };
        let mut torn_reads = 0;
        for _ in 0..5_000 {
            let contents = std::fs::read_to_string(&file).unwrap();
            if contents.parse::<u64>().is_err() {
                torn_reads += 1;
            }
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        writer.join().unwrap();
        assert_eq!(torn_reads, 0, "readers observed a partially written marker");
    }

    #[test]
    fn should_execute_returns_false_within_interval() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, now_seconds().to_string()).unwrap();

        let executed = should_execute(Some(60 * 60), Some((dir, file))).unwrap();
        assert!(!executed);
    }

    #[test]
    fn should_execute_returns_true_after_interval_and_leaves_marker_untouched() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        // Long-past timestamp guarantees we are outside the interval.
        std::fs::write(&file, "1").unwrap();

        let executed = should_execute(Some(60), Some((dir, file.clone()))).unwrap();

        assert!(executed);
        // The marker is NOT refreshed by should_execute — only the caller
        // advances it, and only after the hook command succeeds.
        let written: u64 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert_eq!(written, 1, "should_execute must not refresh the marker");
    }

    #[test]
    fn should_execute_with_future_dated_marker_does_not_panic_and_returns_false() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        // A marker can legitimately sit ahead of `now` after clock skew, NTP
        // rollback, or when the marker file is copied from a machine with a
        // faster clock. The subtraction inside `should_execute` must not
        // underflow in that case.
        let future = now_seconds() + 60 * 60 * 24 * 365;
        std::fs::write(&file, future.to_string()).unwrap();

        let executed = should_execute(Some(60), Some((dir, file.clone()))).unwrap();

        assert!(
            !executed,
            "future-dated marker should not trigger execution"
        );
        // The marker is left untouched — we did not refresh it.
        let written: u64 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert_eq!(written, future);
    }

    #[test]
    fn should_execute_with_marker_equal_to_now_returns_false() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, now_seconds().to_string()).unwrap();

        // Zero seconds since last execution, regardless of interval — should
        // never trip the `> interval` check (interval=0 is also not strictly
        // greater).
        let executed = should_execute(Some(0), Some((dir, file))).unwrap();
        assert!(!executed);
    }

    #[test]
    fn should_execute_uses_default_interval_when_none_given() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, now_seconds().to_string()).unwrap();

        // Default interval is 24 hours — a fresh marker should not trigger execution.
        let executed = should_execute(None, Some((dir, file))).unwrap();
        assert!(!executed);
    }

    #[test]
    fn write_last_execution_is_noop_when_no_paths_given() {
        write_last_execution(None).unwrap();
    }

    #[test]
    fn write_last_execution_creates_directory_and_file() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);

        write_last_execution(Some((dir.clone(), file.clone()))).unwrap();

        assert!(dir.exists());
        assert!(file.exists());
        let written: u64 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert!(written > 0);
        assert!(written <= now_seconds());
    }

    #[test]
    fn write_last_execution_overwrites_existing_file() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "0").unwrap();

        write_last_execution(Some((dir, file.clone()))).unwrap();

        let written: u64 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert!(written > 0);
    }

    /// Helper for the env-var-driven tests below. The `last_paths` function
    /// reads two environment variables (`PASEJO_DATA_DIR` and
    /// `PASEJO_DISABLE_HOOK_THROTTLING`); we clear both so each test starts
    /// from a known-clean state regardless of what an earlier test left
    /// behind.
    ///
    /// SAFETY: `env::remove_var` is unsafe in Rust 2024 because env access
    /// is not thread-safe. Every caller is inside a `#[serial]` group, so
    /// no other test mutating the same variables can be running.
    unsafe fn clear_hook_env_vars() {
        unsafe {
            env::remove_var(environment_variables::PASEJO_DISABLE_HOOK_THROTTLING);
            env::remove_var(environment_variables::PASEJO_DATA_DIR);
        }
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn last_pull_paths_and_last_push_paths_use_distinct_subdirectories() {
        unsafe { clear_hook_env_vars() };
        // BaseDirs::new() may legitimately return None on some unusual setups,
        // but on a normal CI/dev machine both should resolve.
        if let (Some((pull_dir, pull_file)), Some((push_dir, push_file))) = (
            last_pull_paths(OsStr::new("store-x")),
            last_push_paths(OsStr::new("store-x")),
        ) {
            assert_ne!(pull_dir, push_dir);
            assert_ne!(pull_file, push_file);
            assert!(pull_dir.ends_with("last-pull"));
            assert!(push_dir.ends_with("last-push"));
            assert!(pull_file.ends_with("store-x"));
            assert!(push_file.ends_with("store-x"));
        }
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn disable_hook_throttling_env_var_returns_none_paths() {
        unsafe { clear_hook_env_vars() };
        // SAFETY: see clear_hook_env_vars; `#[serial]` keeps this section
        // race-free.
        unsafe {
            env::set_var(environment_variables::PASEJO_DISABLE_HOOK_THROTTLING, "1");
        }

        let pull = last_pull_paths(OsStr::new("store-x"));
        let push = last_push_paths(OsStr::new("store-x"));

        unsafe { clear_hook_env_vars() };

        assert!(pull.is_none());
        assert!(push.is_none());
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn disable_hook_throttling_makes_should_execute_always_run_without_marker() {
        unsafe { clear_hook_env_vars() };
        unsafe {
            env::set_var(environment_variables::PASEJO_DISABLE_HOOK_THROTTLING, "1");
        }

        // With throttling disabled, last_pull_paths returns None — and
        // should_execute(_, None) is documented to always allow execution.
        let executed_first =
            should_execute(Some(60 * 60), last_pull_paths(OsStr::new("store-y"))).unwrap();
        let executed_second =
            should_execute(Some(60 * 60), last_pull_paths(OsStr::new("store-y"))).unwrap();

        unsafe { clear_hook_env_vars() };

        assert!(executed_first);
        assert!(executed_second);
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn pasejo_data_dir_env_var_overrides_base_dirs() {
        let temp = TempDir::new().unwrap();
        unsafe { clear_hook_env_vars() };
        unsafe {
            env::set_var(environment_variables::PASEJO_DATA_DIR, temp.path());
        }

        let (pull_dir, pull_file) = last_pull_paths(OsStr::new("store-x")).unwrap();
        let (push_dir, push_file) = last_push_paths(OsStr::new("store-x")).unwrap();

        unsafe { clear_hook_env_vars() };

        assert!(pull_dir.starts_with(temp.path()));
        assert!(push_dir.starts_with(temp.path()));
        assert!(pull_dir.ends_with("last-pull"));
        assert!(push_dir.ends_with("last-push"));
        assert!(pull_file.ends_with("store-x"));
        assert!(push_file.ends_with("store-x"));
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn pasejo_data_dir_override_persists_marker_in_target_directory() {
        let temp = TempDir::new().unwrap();
        unsafe { clear_hook_env_vars() };
        unsafe {
            env::set_var(environment_variables::PASEJO_DATA_DIR, temp.path());
        }

        // First call is due (no marker yet). Recording success via
        // write_last_execution stamps the marker under the override dir;
        // the second call then sees it within the interval and is not due.
        let executed_first =
            should_execute(Some(60 * 60), last_pull_paths(OsStr::new("store-y"))).unwrap();
        write_last_execution(last_pull_paths(OsStr::new("store-y"))).unwrap();
        let executed_second =
            should_execute(Some(60 * 60), last_pull_paths(OsStr::new("store-y"))).unwrap();

        let (_dir, file) = last_pull_paths(OsStr::new("store-y")).unwrap();
        let marker_under_override = file.starts_with(temp.path());
        let marker_exists = file.exists();

        unsafe { clear_hook_env_vars() };

        assert!(executed_first);
        assert!(!executed_second);
        assert!(marker_under_override);
        assert!(marker_exists);
    }

    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn disable_throttling_takes_precedence_over_data_dir_override() {
        let temp = TempDir::new().unwrap();
        unsafe { clear_hook_env_vars() };
        unsafe {
            env::set_var(environment_variables::PASEJO_DATA_DIR, temp.path());
            env::set_var(environment_variables::PASEJO_DISABLE_HOOK_THROTTLING, "1");
        }

        let pull = last_pull_paths(OsStr::new("store-x"));

        unsafe { clear_hook_env_vars() };

        assert!(
            pull.is_none(),
            "PASEJO_DISABLE_HOOK_THROTTLING must short-circuit even when PASEJO_DATA_DIR is set"
        );
    }
}
