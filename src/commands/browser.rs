// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::browser::host;
use crate::browser::install_manifest;
use crate::browser::matcher::find_matches;
use crate::browser::protocol::QueryResponse;
use crate::commands::store_op::{StoreMutation, with_store};
use crate::models::cli::{BrowserCommands, QueryArgs};
use crate::models::configuration::Configuration;

pub fn dispatch(
    command: &BrowserCommands,
    configuration: &Configuration,
    offline: bool,
) -> anyhow::Result<()> {
    match command {
        BrowserCommands::Query(args) => query(args, configuration, offline),
        BrowserCommands::Host(args) => host::run(configuration, args, offline),
        BrowserCommands::InstallManifest(args) => install_manifest::run(args),
    }
}

fn query(args: &QueryArgs, configuration: &Configuration, offline: bool) -> anyhow::Result<()> {
    let response = with_store(
        configuration,
        args.store_selection.store.as_ref(),
        offline,
        |registration, store| {
            let results = find_matches(
                store,
                &registration.name,
                &args.domain,
                &args.uri_keys,
                &args.username_keys,
            );
            Ok((QueryResponse { results }, StoreMutation::Unchanged))
        },
    )?;
    let json = if args.pretty.unwrap_or(false) {
        serde_json::to_string_pretty(&response)?
    } else {
        serde_json::to_string(&response)?
    };
    println!("{json}");
    Ok(())
}
