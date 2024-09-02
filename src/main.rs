mod adapters;
mod cli;
mod commands;
mod models;

use anyhow::Result;
use clap::Parser;
use human_panic::{setup_panic, Metadata};

use commands::dispatcher::dispatch_command;
use models::cli::Cli;
use models::configuration::Configuration;

fn main() -> Result<()> {
    setup_panic!(
        Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .authors("metio.wtf <whatup@mmetio.wtf>")
            .homepage("https://github.com/metio/pasejo")
            .support("- Open a support request by creating a ticket on GitHub")
    );

    let cli = Cli::parse();
    let configuration = Configuration::load();

    dispatch_command(cli, configuration)
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert();
}
