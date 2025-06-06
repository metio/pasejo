// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use clap::CommandFactory;
use clap::error::ErrorKind;

use crate::models::cli::Cli;

pub fn error_exit(command: &str, subcommand: &str, kind: ErrorKind, message: &str) -> ! {
    let mut cli = Cli::command();
    cli.build();
    cli.find_subcommand_mut(command)
        .and_then(|cmd| cmd.find_subcommand_mut(subcommand))
        .map_or_else(
            || panic!("No matching command/subcommand found"),
            |command| command.error(kind, message).exit(),
        )
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    #[should_panic(expected = "No matching command/subcommand found")]
    fn invalid_command() {
        error_exit(
            "unknown",
            "unknown",
            ErrorKind::InvalidValue,
            "some message",
        );
    }

    #[test]
    #[should_panic(expected = "No matching command/subcommand found")]
    fn invalid_subcommand() {
        error_exit(
            "recipient",
            "unknown",
            ErrorKind::InvalidValue,
            "some message",
        );
    }
}
