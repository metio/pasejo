mod adapters;
mod cli;
mod commands;
mod models;

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
    let configuration = Configuration::load();

    dispatch_command(cli, configuration)
}
