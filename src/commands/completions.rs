use crate::models::cli::Cli;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io::stdout;

pub fn print(shell: &Shell) -> anyhow::Result<()> {
    let command = &mut Cli::command();
    generate(
        *shell,
        command,
        command.get_name().to_string(),
        &mut stdout(),
    );
    Ok(())
}
