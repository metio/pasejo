// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::models::cli::{ConfigCommands, ConfigurationOption};
use crate::models::configuration::Configuration;

pub fn dispatch(command: &ConfigCommands, configuration: Configuration) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Get(args) => {
            get(&configuration, &args.option);
            Ok(())
        }
        ConfigCommands::Set(args) => set(configuration, &args.option, &args.value),
    }
}

fn get(configuration: &Configuration, option: &ConfigurationOption) {
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
        ConfigurationOption::ClipboardNotify => {
            println!("{}", configuration.clipboard_notify.unwrap_or(true));
        }
        ConfigurationOption::KeyDownloadTimeoutSeconds => {
            println!(
                "{}",
                configuration.key_download_timeout_seconds.unwrap_or(30)
            );
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

fn set(
    mut configuration: Configuration,
    option: &ConfigurationOption,
    value: &str,
) -> anyhow::Result<()> {
    match option {
        ConfigurationOption::IgnoreMissingIdentities => {
            if value.is_empty() {
                configuration.ignore_missing_identities = None;
            } else {
                configuration.ignore_missing_identities = Some(parse_bool(value)?);
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
        ConfigurationOption::ClipboardNotify => {
            if value.is_empty() {
                configuration.clipboard_notify = None;
            } else {
                configuration.clipboard_notify = Some(parse_bool(value)?);
            }
        }
        ConfigurationOption::KeyDownloadTimeoutSeconds => {
            if value.is_empty() {
                configuration.key_download_timeout_seconds = None;
            } else {
                let timeout = value.parse::<u64>()?;
                configuration.key_download_timeout_seconds = Some(timeout);
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

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => anyhow::bail!(
            "Invalid boolean value '{value}'. Expected one of: true, false, 1, 0, yes, no, y, n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bool_accepts_canonical_truthy_values() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
    }

    #[test]
    fn parse_bool_accepts_canonical_falsy_values() {
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
    }

    #[test]
    fn parse_bool_is_case_insensitive() {
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("True").unwrap());
        assert!(parse_bool("YES").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("No").unwrap());
    }

    #[test]
    fn parse_bool_accepts_short_aliases() {
        assert!(parse_bool("y").unwrap());
        assert!(parse_bool("n").is_ok());
        assert!(!parse_bool("n").unwrap());
    }

    #[test]
    fn parse_bool_short_aliases_are_case_insensitive() {
        assert!(parse_bool("Y").unwrap());
        assert!(!parse_bool("N").unwrap());
    }

    #[test]
    fn parse_bool_rejects_typos() {
        assert!(parse_bool("tru").is_err());
        assert!(parse_bool("fals").is_err());
        assert!(parse_bool("yez").is_err());
        assert!(parse_bool("nope").is_err());
        assert!(parse_bool("ye").is_err());
        assert!(parse_bool("no!").is_err());
    }

    #[test]
    fn parse_bool_rejects_whitespace_padded_input() {
        assert!(parse_bool("true ").is_err());
        assert!(parse_bool(" true").is_err());
        assert!(parse_bool(" true ").is_err());
        assert!(parse_bool("True ").is_err());
    }

    #[test]
    fn parse_bool_rejects_empty_string() {
        assert!(parse_bool("").is_err());
    }

    #[test]
    fn parse_bool_rejects_other_numerics() {
        assert!(parse_bool("2").is_err());
        assert!(parse_bool("-1").is_err());
        assert!(parse_bool("01").is_err());
    }

    #[test]
    fn parse_bool_error_message_lists_accepted_values() {
        let error = parse_bool("maybe").unwrap_err().to_string();
        assert!(error.contains("maybe"));
        assert!(error.contains("true"));
        assert!(error.contains("false"));
    }
}
