use crate::models::configuration::{Configuration, ConfigurationOption};

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
    }
    configuration.save_configuration()
}
