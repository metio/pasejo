// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::process::ExitCode;

use clap::Parser;

/// Syncs pasejo remote stores, manages organizations and audits access.
#[derive(Parser)]
#[command(version, about)]
struct Cli {}

fn main() -> ExitCode {
    Cli::parse();
    eprintln!("pasejo-server serves nothing yet; design/server.md describes what it will do");
    ExitCode::FAILURE
}
