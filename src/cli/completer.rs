// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use clap_complete::{ArgValueCompleter, CompletionCandidate};

use crate::models::configuration::Configuration;

pub fn store_name() -> ArgValueCompleter {
    ArgValueCompleter::new(|current: &std::ffi::OsStr| {
        Configuration::cached().map_or_else(
            |_| vec![],
            |configuration| {
                let prefix = current.to_str().unwrap_or("");
                configuration
                    .store_names_with_prefix(prefix)
                    .iter()
                    .map(CompletionCandidate::new)
                    .collect()
            },
        )
    })
}
