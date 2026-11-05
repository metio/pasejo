// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use anyhow::Context;

use crate::cli::i18n;
use crate::hooks::files::{last_pull_paths, last_push_paths, should_execute, write_last_execution};
use crate::models::configuration::{Configuration, StoreRegistration};

pub struct HookExecutor<'a> {
    configuration: &'a Configuration,
    registration: &'a StoreRegistration,
    offline: bool,
    /// When true, throttling is bypassed and hooks always run (used by the
    /// explicit `hook run` command); when false, the per-store interval
    /// gate in `hooks::files` applies.
    force: bool,
}

impl<'a> HookExecutor<'a> {
    /// Throttled executor — pull/push hooks honor the configured interval.
    /// This is what every implicit pull/push around a store operation uses.
    pub const fn new(
        configuration: &'a Configuration,
        registration: &'a StoreRegistration,
        offline: bool,
    ) -> Self {
        Self {
            configuration,
            registration,
            offline,
            force: false,
        }
    }

    /// Forced executor — bypasses the throttle so the hooks run now. Used by
    /// the explicit `hook run` command, where the user asked for execution.
    pub const fn forced(
        configuration: &'a Configuration,
        registration: &'a StoreRegistration,
        offline: bool,
    ) -> Self {
        Self {
            configuration,
            registration,
            offline,
            force: true,
        }
    }
}

/// Which side of synchronization a hook run belongs to. Pull and push share
/// the same throttle/log/stamp protocol; the phase selects which command
/// lists, interval, log line, and marker files apply, so that protocol has
/// one definition instead of two copies that must stay in lockstep.
#[derive(Clone, Copy)]
enum Phase {
    Pull,
    Push,
}

impl HookExecutor<'_> {
    pub fn execute_pull_commands(&self) -> anyhow::Result<()> {
        self.execute_phase(Phase::Pull)
    }

    pub fn execute_push_commands(&self) -> anyhow::Result<()> {
        self.execute_phase(Phase::Push)
    }

    fn execute_phase(&self, phase: Phase) -> anyhow::Result<()> {
        let (global_commands, store_commands) = match phase {
            Phase::Pull => (
                &self.configuration.pull_commands,
                &self.registration.pull_commands,
            ),
            Phase::Push => (
                &self.configuration.push_commands,
                &self.registration.push_commands,
            ),
        };
        if global_commands.is_empty() && store_commands.is_empty() {
            return Ok(());
        }
        let Some(store_name) = marker_name(self.registration.path()) else {
            anyhow::bail!(i18n::error_cannot_determine_store_name())
        };
        let store_name = store_name.as_os_str();
        let guard = self.registration.path().parent().map(directory_guard);
        let _held = guard
            .as_deref()
            .map(|guard| guard.lock().unwrap_or_else(PoisonError::into_inner));
        if self.force || (!self.offline && self.is_due(phase, store_name)?) {
            match phase {
                Phase::Pull => i18n::execute_pull_hooks(&self.registration.name),
                Phase::Push => i18n::execute_push_hooks(&self.registration.name),
            }
            self.execute(global_commands)?;
            self.execute(store_commands)?;
            // Stamp the throttle marker only now that the commands have
            // succeeded — a `?` above bails before this, so a failed hook
            // leaves the previous timestamp intact and the next invocation
            // retries instead of silently skipping for the rest of the
            // interval.
            write_last_execution(Self::marker_paths(phase, store_name))?;
        }
        Ok(())
    }

    fn execute(&self, commands: &[String]) -> anyhow::Result<()> {
        if commands.is_empty() {
            return Ok(());
        }

        let store_path = self.registration.path();
        let store_path_display = store_path.display().to_string();
        let parent = store_path
            .parent()
            .with_context(|| i18n::error_cannot_determine_store_parent_path(&store_path_display))?;

        for command in commands {
            let command_display = format!("{command:?}");
            let Some(template) = shlex::split(command) else {
                anyhow::bail!(i18n::error_cannot_parse_hook_command(&command_display));
            };
            let args = build_args(&template, store_path)?;
            let (binary, rest) = args
                .split_first()
                .with_context(|| i18n::error_empty_hook_command(&command_display))?;

            // Hooks are non-interactive sync commands. Closing stdin keeps an
            // accidentally-interactive hook (a `git` that wants a passphrase)
            // from blocking forever on a read instead of failing fast, and —
            // critically — stops a hook from consuming the parent's stdin,
            // which under `browser host` is the native-messaging byte stream
            // the host is mid-parse on.
            let output = Command::new(binary)
                .args(rest)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .current_dir(parent)
                .output()
                .with_context(|| i18n::error_failed_to_run_hook(&command_display))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let detail = stderr.trim();
                let exit = output
                    .status
                    .code()
                    .map_or_else(|| String::from("signal"), |c| c.to_string());
                if detail.is_empty() {
                    anyhow::bail!(i18n::error_hook_failed_no_detail(&command_display, &exit));
                }
                anyhow::bail!(i18n::error_hook_failed_with_detail(
                    &command_display,
                    &exit,
                    detail
                ));
            }
        }

        Ok(())
    }

    /// Whether enough time has elapsed since the last successful run of this
    /// phase for it to run again (subject to the per-side interval).
    fn is_due(&self, phase: Phase, store_name: &OsStr) -> anyhow::Result<bool> {
        let interval = match phase {
            Phase::Pull => self.configuration.pull_interval_seconds,
            Phase::Push => self.configuration.push_interval_seconds,
        };
        should_execute(interval, Self::marker_paths(phase, store_name))
    }

    /// The (directory, file) throttle-marker paths for this phase, or `None`
    /// when throttling is disabled.
    fn marker_paths(
        phase: Phase,
        store_name: &OsStr,
    ) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        match phase {
            Phase::Pull => last_pull_paths(store_name),
            Phase::Push => last_push_paths(store_name),
        }
    }
}

/// The lock serializing hook runs in `directory` within this process. Hooks
/// run with the store's directory as working directory, so two stores in
/// one git work tree would otherwise run `git pull` there concurrently when
/// the browser host answers their queries on parallel workers.
fn directory_guard(directory: &Path) -> Arc<Mutex<()>> {
    static GUARDS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> =
        LazyLock::new(Mutex::default);
    let mut guards = GUARDS.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(guards.entry(directory.to_path_buf()).or_default())
}

/// The throttle-marker file name for the store at `path`: its file name,
/// for readability, plus a hash of the whole path, so stores in different
/// directories that share a file name keep separate markers.
fn marker_name(path: &Path) -> Option<OsString> {
    let mut name = path.file_name()?.to_os_string();
    name.push(format!(
        "-{:016x}",
        fnv1a_64(path.as_os_str().as_encoded_bytes())
    ));
    Some(name)
}

fn fnv1a_64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Resolve `%p` placeholders in a tokenized hook command. A token equal to
/// `"%p"` is replaced with the raw store path passed as a single argument,
/// so paths containing shell metacharacters (quotes, semicolons, spaces)
/// can never escape their argument boundary. Tokens that merely contain
/// `%p` (e.g. `--git-dir=%p/.git`) get string substitution and require the
/// store path to be valid UTF-8.
fn build_args(template: &[String], path: &Path) -> anyhow::Result<Vec<OsString>> {
    let path_display = path.display().to_string();
    let mut out = Vec::with_capacity(template.len());
    for token in template {
        if token == "%p" {
            out.push(path.as_os_str().to_os_string());
        } else if token.contains("%p") {
            let token_display = format!("{token:?}");
            let path_str = path
                .to_str()
                .with_context(|| i18n::error_store_path_not_utf8(&token_display, &path_display))?;
            out.push(OsString::from(token.replace("%p", path_str)));
        } else {
            out.push(OsString::from(token));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn build_args_passes_path_with_metacharacters_as_single_arg() {
        let template = shlex::split("git add %p").unwrap();
        let path = PathBuf::from("/tmp/pasejo'; touch /tmp/PWNED #/store.age");

        let args = build_args(&template, &path).unwrap();

        assert_eq!(args.len(), 3);
        assert_eq!(args[0], OsString::from("git"));
        assert_eq!(args[1], OsString::from("add"));
        assert_eq!(args[2], path.as_os_str());
    }

    #[test]
    fn build_args_substitutes_inside_token() {
        let template = shlex::split("git --git-dir=%p/.git status").unwrap();
        let path = PathBuf::from("/tmp/store");

        let args = build_args(&template, &path).unwrap();

        assert_eq!(args.len(), 3);
        assert_eq!(args[0], OsString::from("git"));
        assert_eq!(args[1], OsString::from("--git-dir=/tmp/store/.git"));
        assert_eq!(args[2], OsString::from("status"));
    }

    #[test]
    fn build_args_leaves_unrelated_tokens_alone() {
        let template = shlex::split("echo hello world").unwrap();
        let path = PathBuf::from("/tmp/store");

        let args = build_args(&template, &path).unwrap();

        assert_eq!(
            args,
            vec![
                OsString::from("echo"),
                OsString::from("hello"),
                OsString::from("world"),
            ]
        );
    }

    #[test]
    fn build_args_handles_quoted_token_with_path() {
        // The user wrote: git commit -m "msg %p"
        // shlex strips the quotes, so the message becomes one token.
        let template = shlex::split("git commit -m \"changed %p\"").unwrap();
        let path = PathBuf::from("/tmp/store");

        let args = build_args(&template, &path).unwrap();

        assert_eq!(args[0], OsString::from("git"));
        assert_eq!(args[1], OsString::from("commit"));
        assert_eq!(args[2], OsString::from("-m"));
        assert_eq!(args[3], OsString::from("changed /tmp/store"));
    }

    #[test]
    fn marker_name_keeps_the_file_name_and_differs_per_directory() {
        let first = marker_name(Path::new("/a/store.age")).unwrap();
        let second = marker_name(Path::new("/b/store.age")).unwrap();
        assert_ne!(first, second);
        assert!(first.to_string_lossy().starts_with("store.age-"));
        assert_eq!(first, marker_name(Path::new("/a/store.age")).unwrap());
    }

    #[test]
    fn fnv1a_64_matches_the_reference_vectors() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[cfg(unix)]
    #[test]
    #[serial_test::serial(pasejo_hook_env)]
    fn stores_sharing_a_file_name_are_throttled_independently() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        // SAFETY: the `pasejo_hook_env` serial group keeps every test that
        // touches these variables from running concurrently.
        unsafe {
            std::env::remove_var(crate::cli::environment_variables::PASEJO_DISABLE_HOOK_THROTTLING);
            std::env::set_var(
                crate::cli::environment_variables::PASEJO_DATA_DIR,
                temp.path().join("data"),
            );
        }
        let registration = |name: &str| {
            let directory = temp.path().join(name);
            std::fs::create_dir_all(&directory).unwrap();
            StoreRegistration {
                path: directory.join("store.age"),
                name: name.to_string(),
                identities: vec![],
                pull_commands: vec![format!(
                    "touch {}",
                    temp.path().join(format!("pulled-{name}")).display()
                )],
                push_commands: vec![],
            }
        };
        let first = registration("first");
        let second = registration("second");
        let configuration = Configuration::default();

        HookExecutor::new(&configuration, &first, false)
            .execute_pull_commands()
            .unwrap();
        HookExecutor::new(&configuration, &second, false)
            .execute_pull_commands()
            .unwrap();

        unsafe {
            std::env::remove_var(crate::cli::environment_variables::PASEJO_DATA_DIR);
        }
        assert!(temp.path().join("pulled-first").exists());
        assert!(
            temp.path().join("pulled-second").exists(),
            "the second store's pull hook was throttled by the first store's marker"
        );
    }
}
