// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::Write;

mod cli;
mod commands;
mod downloader;
mod identities;
mod models;
mod one_time_passwords;
mod recipients;
mod secrets;
mod synchronizers;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use commands::dispatcher::dispatch_command;
use human_panic::{setup_panic, Metadata};
use models::cli::Cli;
use models::configuration::Configuration;

fn main() -> Result<()> {
    setup_panic!(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .authors("metio.wtf <whatup@mmetio.wtf>")
            .homepage("https://github.com/metio/pasejo")
            .support("- Open a support request by creating a ticket on GitHub")
    );

    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .format(|buf, record| {
            writeln!(
                buf,
                "{}: {}",
                record.level().to_string().to_ascii_lowercase(),
                record.args()
            )
        })
        .init();
    let configuration = Configuration::load_configuration()?;

    dispatch_command(&cli, configuration)
}
