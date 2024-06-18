use anyhow::Result;
use clap::Parser;
use human_panic::Metadata;
use human_panic::setup_panic;

use pasejo::cli::arguments::Cli;
use pasejo::cli::configuration::Configuration;
use pasejo::commands::dispatcher::dispatch_command;

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
