// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use clap::ValueEnum;

use crate::cli::i18n;
use crate::models::cli::{ConfigCommands, ConfigurationOption, OptionValueKind};
use crate::models::configuration::{
    Configuration, DEFAULT_CLIPBOARD_NOTIFY, DEFAULT_CLIPBOARD_TIMEOUT_SECONDS,
    DEFAULT_HOOK_INTERVAL_SECONDS, DEFAULT_IGNORE_MISSING_IDENTITIES,
    DEFAULT_KEY_DOWNLOAD_TIMEOUT_SECONDS,
};

pub fn dispatch(command: &ConfigCommands, configuration: &Configuration) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Get(args) => {
            get(configuration, args.option);
            Ok(())
        }
        ConfigCommands::Set(args) => set(args.option, &args.value),
    }
}

fn get(configuration: &Configuration, option: ConfigurationOption) {
    // Each arm reads its own typed field; the default it falls back to is the
    // shared constant the consumption sites also use, so the printed default
    // always matches the effective behaviour.
    match option {
        ConfigurationOption::IgnoreMissingIdentities => println!(
            "{}",
            configuration
                .ignore_missing_identities
                .unwrap_or(DEFAULT_IGNORE_MISSING_IDENTITIES)
        ),
        ConfigurationOption::ClipboardTimeout => println!(
            "{}",
            configuration
                .clipboard_timeout
                .unwrap_or(DEFAULT_CLIPBOARD_TIMEOUT_SECONDS)
        ),
        ConfigurationOption::ClipboardNotify => println!(
            "{}",
            configuration
                .clipboard_notify
                .unwrap_or(DEFAULT_CLIPBOARD_NOTIFY)
        ),
        ConfigurationOption::KeyDownloadTimeoutSeconds => println!(
            "{}",
            configuration
                .key_download_timeout_seconds
                .unwrap_or(DEFAULT_KEY_DOWNLOAD_TIMEOUT_SECONDS)
        ),
        ConfigurationOption::PullIntervalSeconds => println!(
            "{}",
            configuration
                .pull_interval_seconds
                .unwrap_or(DEFAULT_HOOK_INTERVAL_SECONDS)
        ),
        ConfigurationOption::PushIntervalSeconds => println!(
            "{}",
            configuration
                .push_interval_seconds
                .unwrap_or(DEFAULT_HOOK_INTERVAL_SECONDS)
        ),
    }
}

/// Parses `value` per the option's value kind and writes it surgically under
/// the option's TOML key, preserving unknown keys already on the file (see
/// [`Configuration::set_top_level_key`]). The key and the parse strategy both
/// come from the option's own metadata, so there is no per-option arm to keep
/// in sync. An empty value clears the key, resetting the option to its
/// built-in default.
fn set(option: ConfigurationOption, value: &str) -> anyhow::Result<()> {
    let parsed = match option.value_kind() {
        OptionValueKind::Boolean => optional_bool(option, value)?,
        OptionValueKind::Integer => optional_integer(option, value)?,
    };
    Configuration::set_top_level_key(option.toml_key(), parsed)
}

/// The kebab-case name clap accepted for `option`, for error messages.
fn option_name(option: ConfigurationOption) -> String {
    option
        .to_possible_value()
        .map_or_else(|| String::from("?"), |value| value.get_name().to_owned())
}

fn optional_bool(option: ConfigurationOption, value: &str) -> anyhow::Result<Option<toml::Value>> {
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(toml::Value::Boolean(parse_bool(option, value)?)))
}

fn optional_integer(
    option: ConfigurationOption,
    value: &str,
) -> anyhow::Result<Option<toml::Value>> {
    if value.is_empty() {
        return Ok(None);
    }
    // Parse as u64 first so a negative input is rejected, then narrow to
    // the i64 TOML integers use. Both failure modes surface the same
    // localized "expected a non-negative integer" message naming the
    // option, instead of a raw `ParseIntError`.
    let parsed = value
        .parse::<u64>()
        .ok()
        .and_then(|n| i64::try_from(n).ok())
        .ok_or_else(|| {
            anyhow::anyhow!(i18n::error_config_invalid_integer(
                &option_name(option),
                value
            ))
        })?;
    Ok(Some(toml::Value::Integer(parsed)))
}

fn parse_bool(option: ConfigurationOption, value: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => anyhow::bail!(i18n::error_config_invalid_boolean(
            &option_name(option),
            value
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::i18n;

    const OPT: ConfigurationOption = ConfigurationOption::ClipboardNotify;

    fn parse_bool(value: &str) -> anyhow::Result<bool> {
        super::parse_bool(OPT, value)
    }

    #[test]
    fn every_option_key_maps_to_a_serialized_configuration_field_of_the_right_type() {
        // Pins the single-source metadata to the struct: every option's
        // `toml_key()` must be a real Configuration field, and its
        // `value_kind()` must match how that field serializes. A renamed
        // field or a wrong kind fails here instead of silently writing a key
        // `config get` can never read back.
        let configuration = Configuration {
            ignore_missing_identities: Some(true),
            clipboard_timeout: Some(1),
            clipboard_notify: Some(true),
            key_download_timeout_seconds: Some(1),
            pull_interval_seconds: Some(1),
            push_interval_seconds: Some(1),
            ..Configuration::default()
        };
        let serialized = toml::to_string(&configuration).unwrap();
        let table: toml::Table = toml::from_str(&serialized).unwrap();

        for option in ConfigurationOption::value_variants() {
            let key = option.toml_key();
            let value = table
                .get(key)
                .unwrap_or_else(|| panic!("`{key}` is not a Configuration field"));
            match option.value_kind() {
                OptionValueKind::Boolean => {
                    assert!(value.is_bool(), "`{key}` should serialize as a boolean");
                }
                OptionValueKind::Integer => {
                    assert!(value.is_integer(), "`{key}` should serialize as an integer");
                }
            }
        }
    }

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
    fn parse_bool_error_message_names_the_option_and_value() {
        i18n::init_for_tests();
        let error = parse_bool("maybe").unwrap_err().to_string();
        assert!(
            error.contains("maybe"),
            "error should name the bad value: {error}"
        );
        assert!(
            error.contains("clipboard-notify"),
            "error should name the option: {error}"
        );
    }

    #[test]
    fn optional_integer_rejects_a_negative_value_with_a_localized_message() {
        i18n::init_for_tests();
        let error = optional_integer(ConfigurationOption::ClipboardTimeout, "-1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("-1"));
        assert!(error.contains("clipboard-timeout"));
    }

    #[test]
    fn optional_integer_clears_on_empty_and_parses_a_valid_value() {
        assert!(matches!(
            optional_integer(ConfigurationOption::ClipboardTimeout, ""),
            Ok(None)
        ));
        assert!(matches!(
            optional_integer(ConfigurationOption::ClipboardTimeout, "60"),
            Ok(Some(toml::Value::Integer(60)))
        ));
    }
}
