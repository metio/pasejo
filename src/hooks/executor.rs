// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Context;

use crate::cli::i18n;
use crate::hooks::files::{last_pull_paths, last_push_paths, should_execute, write_last_execution};
use crate::models::configuration::{Configuration, StoreRegistration};

pub struct HookExecutor<'a> {
    pub configuration: &'a Configuration,
    pub registration: &'a StoreRegistration,
    pub offline: bool,
    pub force: bool,
}

impl HookExecutor<'_> {
    pub fn execute_pull_commands(&self) -> anyhow::Result<()> {
        if !&self.configuration.pull_commands.is_empty()
            || !&self.registration.pull_commands.is_empty()
        {
            if let Some(store_name) = self.registration.path().file_name() {
                if self.force || (!self.offline && self.should_pull(store_name)?) {
                    i18n::execute_pull_hooks(&self.registration.name);
                    self.execute(&self.configuration.pull_commands)?;
                    self.execute(&self.registration.pull_commands)?;
                    if self.force {
                        write_last_execution(last_pull_paths(store_name))?;
                    }
                }
                Ok(())
            } else {
                anyhow::bail!(i18n::error_cannot_determine_store_name())
            }
        } else {
            Ok(())
        }
    }

    pub fn execute_push_commands(&self) -> anyhow::Result<()> {
        if !&self.configuration.push_commands.is_empty()
            || !&self.registration.push_commands.is_empty()
        {
            if let Some(store_name) = self.registration.path().file_name() {
                if self.force || (!self.offline && self.should_push(store_name)?) {
                    i18n::execute_push_hooks(&self.registration.name);
                    self.execute(&self.configuration.push_commands)?;
                    self.execute(&self.registration.push_commands)?;
                    if self.force {
                        write_last_execution(last_push_paths(store_name))?;
                    }
                }
                Ok(())
            } else {
                anyhow::bail!(i18n::error_cannot_determine_store_name())
            }
        } else {
            Ok(())
        }
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

            let output = Command::new(binary)
                .args(rest)
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

    fn should_pull(&self, store_name: &OsStr) -> anyhow::Result<bool> {
        should_execute(
            self.configuration.pull_interval_seconds,
            last_pull_paths(store_name),
        )
    }

    fn should_push(&self, store_name: &OsStr) -> anyhow::Result<bool> {
        should_execute(
            self.configuration.push_interval_seconds,
            last_push_paths(store_name),
        )
    }
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
}
