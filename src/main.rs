// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

mod cli;
mod commands;
mod downloader;
mod exporters;
mod hooks;
mod identities;
mod models;
mod one_time_passwords;
mod recipients;
mod secrets;

use crate::cli::i18n;
use crate::commands::stores;
use crate::models::cli::Commands;
use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use human_panic::{Metadata, setup_panic};
use models::cli::Cli;
use models::configuration::Configuration;

fn main() -> Result<()> {
    setup_panic!(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .authors("metio.wtf <pasejo@metio.wtf>")
            .homepage("https://github.com/metio/pasejo")
            .support("- Open a support request by creating a ticket on GitHub")
    );

    i18n::init()?;

    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    env_logger::builder()
        .filter_level(cli.verbose.log_level_filter())
        .format_timestamp(None)
        .format_level(false)
        .format_module_path(false)
        .format_source_path(false)
        .format_target(false)
        .init();
    let configuration = Configuration::cached()?;

    match &cli.command {
        Commands::Config { command } => commands::config::dispatch(command, configuration.clone()),
        Commands::Export { command } => {
            commands::export::dispatch(command, configuration, cli.offline)
        }
        Commands::Hook { command } => commands::hooks::dispatch(command, configuration.clone()),
        Commands::Identity { command } => {
            commands::identities::dispatch(command, configuration.clone())
        }
        Commands::Otp { command } => {
            commands::one_time_passwords::dispatch(command, &cli, configuration)
        }
        Commands::Recipient { command } => {
            commands::recipients::dispatch(command, configuration, cli.offline)
        }
        Commands::Secret { command } => {
            commands::secrets::dispatch(command, configuration, cli.offline)
        }
        Commands::Store { command } => {
            stores::dispatch(command, configuration.clone(), cli.offline)
        }
    }
}
