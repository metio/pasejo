// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::exporters;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::{BitwardenArgs, ExportCommands};
use crate::models::configuration::Configuration;
use anyhow::Context;

pub fn dispatch(
    command: &ExportCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        ExportCommands::Bitwarden(args) => export_as_json(configuration, args, offline),
    }
}

fn export_as_json(
    configuration: &Configuration,
    args: &BitwardenArgs,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(args.store_selection.store.as_ref()) {
        let hooks = HookExecutor {
            configuration,
            registration,
            offline,
            force: false,
        };

        hooks.execute_pull_commands()?;

        let store = configuration
            .decrypt_store(registration)
            .context("Cannot decrypt store")?;

        println!(
            "{}",
            exporters::bitwarden::json(
                &store,
                args.organization_id.as_ref(),
                args.collection_id.as_ref(),
                args.collection_name.as_ref(),
                args.username_keys.as_ref(),
                args.uri_keys.as_ref(),
                args.pretty,
            )?
        );

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
