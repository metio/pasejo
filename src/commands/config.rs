// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::models::cli::ConfigurationOption;
use crate::models::configuration::Configuration;

pub fn get(configuration: &Configuration, option: &ConfigurationOption) {
    match option {
        ConfigurationOption::IgnoreMissingIdentities => {
            println!(
                "{}",
                configuration.ignore_missing_identities.unwrap_or(true)
            );
        }
        ConfigurationOption::ClipboardTimeout => {
            println!("{}", configuration.clipboard_timeout.unwrap_or(45));
        }
        ConfigurationOption::PullIntervalSeconds => {
            println!(
                "{}",
                configuration.pull_interval_seconds.unwrap_or(60 * 60 * 24)
            );
        }
        ConfigurationOption::PushIntervalSeconds => {
            println!(
                "{}",
                configuration.push_interval_seconds.unwrap_or(60 * 60 * 24)
            );
        }
    }
}

pub fn set(
    mut configuration: Configuration,
    option: &ConfigurationOption,
    value: &str,
) -> anyhow::Result<()> {
    match option {
        ConfigurationOption::IgnoreMissingIdentities => {
            if value.is_empty() {
                configuration.ignore_missing_identities = None;
            } else {
                let truthy = matches!(value.to_ascii_lowercase().as_str(), "true" | "1" | "yes");
                configuration.ignore_missing_identities = Some(truthy);
            }
        }
        ConfigurationOption::ClipboardTimeout => {
            if value.is_empty() {
                configuration.clipboard_timeout = None;
            } else {
                let timeout = value.parse::<u64>()?;
                configuration.clipboard_timeout = Some(timeout);
            }
        }
        ConfigurationOption::PullIntervalSeconds => {
            if value.is_empty() {
                configuration.pull_interval_seconds = None;
            } else {
                let interval = value.parse::<u64>()?;
                configuration.pull_interval_seconds = Some(interval);
            }
        }
        ConfigurationOption::PushIntervalSeconds => {
            if value.is_empty() {
                configuration.push_interval_seconds = None;
            } else {
                let interval = value.parse::<u64>()?;
                configuration.push_interval_seconds = Some(interval);
            }
        }
    }
    configuration.save_configuration()
}
