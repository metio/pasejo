// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::logs;
use crate::hooks::files::{last_pull_paths, last_push_paths, should_execute, write_last_execution};
use crate::models::configuration::{Configuration, StoreRegistration};
use anyhow::Context;
use duct::cmd;
use std::ffi::OsStr;

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
                    logs::execute_pull_hooks(&self.registration.name);
                    self.execute(&self.configuration.pull_commands)?;
                    self.execute(&self.registration.pull_commands)?;
                    if self.force {
                        write_last_execution(last_push_paths(store_name))?;
                    }
                }
                Ok(())
            } else {
                anyhow::bail!("Cannot determine store name")
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
                    logs::execute_push_hooks(&self.registration.name);
                    self.execute(&self.configuration.push_commands)?;
                    self.execute(&self.registration.push_commands)?;
                }
                Ok(())
            } else {
                anyhow::bail!("Cannot determine store name")
            }
        } else {
            Ok(())
        }
    }

    fn execute(&self, commands: &Vec<String>) -> anyhow::Result<()> {
        if !commands.is_empty() {
            if let Some(parent) = self.registration.path().parent() {
                for command in commands {
                    let replaced = self
                        .registration
                        .path()
                        .to_str()
                        .map_or_else(|| command.to_owned(), |path| command.replace("%p", path));

                    if let Some(parts) = shlex::split(&replaced) {
                        let binary = &parts[0];
                        let args = &parts[1..];

                        cmd(binary, args)
                            .stdout_null()
                            .stderr_null()
                            .dir(parent)
                            .run()
                            .context("Failed to run command")?;
                    } else {
                        anyhow::bail!("Cannot parse command: {:?}", command);
                    }
                }
            } else {
                anyhow::bail!("Cannot determine parent of store path");
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
