// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use directories::BaseDirs;

use crate::cli::environment_variables;

pub fn should_execute(
    interval_seconds: Option<u64>,
    paths: Option<(PathBuf, PathBuf)>,
) -> anyhow::Result<bool> {
    if let Some((last_directory, last_file)) = paths {
        if last_file.exists() {
            let last_content = fs::read_to_string(&last_file)?;
            let last_seconds: u64 = last_content.parse()?;
            let epoch_seconds = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs();
            let seconds_since_last_execution = epoch_seconds.saturating_sub(last_seconds);
            let interval = interval_seconds.unwrap_or(60 * 60 * 24);
            let should_execute = seconds_since_last_execution > interval;

            if should_execute {
                fs::write(last_file, epoch_seconds.to_string())?;
                return Ok(should_execute);
            }
            return Ok(false);
        }
        fs::create_dir_all(last_directory)?;
        let epoch_seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        fs::write(last_file, epoch_seconds.to_string())?;
    }

    Ok(true)
}

pub fn write_last_execution(paths: Option<(PathBuf, PathBuf)>) -> anyhow::Result<()> {
    if let Some((last_directory, last_file)) = paths {
        if !last_file.exists() {
            fs::create_dir_all(last_directory)?;
        }
        let epoch_seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();
        Ok(fs::write(last_file, epoch_seconds.to_string())?)
    } else {
        Ok(())
    }
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
    fn should_execute_returns_true_when_no_paths_given() {
        assert!(should_execute(None, None).unwrap());
        assert!(should_execute(Some(60), None).unwrap());
    }

    #[test]
    fn should_execute_creates_marker_file_and_returns_true_when_missing() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);

        let executed = should_execute(None, Some((dir, file.clone()))).unwrap();

        assert!(executed);
        assert!(file.exists(), "marker file should be created");
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
    fn should_execute_returns_true_after_interval_and_updates_marker() {
        let temp = TempDir::new().unwrap();
        let (dir, file) = paths_in(&temp);
        std::fs::create_dir_all(&dir).unwrap();
        // Long-past timestamp guarantees we are outside the interval.
        std::fs::write(&file, "1").unwrap();

        let executed = should_execute(Some(60), Some((dir, file.clone()))).unwrap();

        assert!(executed);
        let written: u64 = std::fs::read_to_string(&file).unwrap().parse().unwrap();
        assert!(
            written >= now_seconds() - 5,
            "marker should be refreshed to current time, got {written}"
        );
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

        // First call creates the marker; second call sees it within interval
        // and returns false
        let executed_first =
            should_execute(Some(60 * 60), last_pull_paths(OsStr::new("store-y"))).unwrap();
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
