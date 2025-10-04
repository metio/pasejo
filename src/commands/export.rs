// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::exporters;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::ExportCommands;
use crate::models::configuration::Configuration;
use anyhow::Context;

pub fn dispatch(
    command: &ExportCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        ExportCommands::Bitwarden(args) => export_as_json(
            configuration,
            args.store_selection.store.as_ref(),
            args.organization_id.as_ref(),
            args.collection_id.as_ref(),
            args.collection_name.as_ref(),
            &args.username_keys,
            &args.uri_keys,
            args.pretty,
            offline,
        ),
    }
}

fn export_as_json(
    configuration: &Configuration,
    store_name: Option<&String>,
    organization_id: Option<&String>,
    collection_id: Option<&String>,
    collection_name: Option<&String>,
    username_keys: &[String],
    uri_keys: &[String],
    pretty: Option<bool>,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
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
                organization_id,
                collection_id,
                collection_name,
                username_keys,
                uri_keys,
                pretty,
            )?
        );

        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
