use anyhow::Result;
use clap::Parser;

use pasejo::cli::Cli;
use pasejo::configuration::Configuration;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg: Configuration = confy::load("pasejo", None)?;
    dbg!(cfg);

    cli.dispatch_command()
}
