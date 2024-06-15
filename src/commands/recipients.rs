// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::logs;
use crate::models::configuration::Configuration;
use crate::models::password_store::{PasswordStore, Recipient};
use crate::recipients;
use std::path::Path;

pub fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    public_key: &(String, String),
    name: Option<&String>,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        let mut store = if store_path.exists() {
            if !offline {
                synchronizer.pull()?;
            }
            configuration.decrypt_store(registration)?
        } else {
            PasswordStore::default()
        };

        if let Some(recipient) = store
            .recipients
            .iter_mut()
            .find(|recipient| recipient.public_key == public_key.0)
        {
            if let Some(name) = name {
                name.clone_into(&mut recipient.name);
            }
        } else {
            store.recipients.push(Recipient {
                public_key: public_key.0.clone(),
                name: name.map_or(&public_key.1, |value| value).to_string(),
            });
        }

        Configuration::encrypt_store(registration, &store)?;

        if !offline {
            synchronizer.push()?;
        }

        logs::recipient_added(&public_key.0);
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    public_key: &str,
    ignore_unknown: bool,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        let store_path = Path::new(&registration.path);
        let synchronizer = registration.synchronizer.select_implementation(store_path);

        let mut store = if store_path.exists() {
            if !offline {
                synchronizer.pull()?;
            }
            configuration.decrypt_store(registration)?
        } else {
            PasswordStore::default()
        };

        if store.recipients.len() == 1 && store.recipients[0].public_key == public_key {
            anyhow::bail!(
                "Cannot remove the last recipient from the store. Please add a new recipient before removing this one."
            )
        }

        if !ignore_unknown
            && !store
                .recipients
                .iter()
                .any(|recipient| recipient.public_key == public_key)
        {
            anyhow::bail!("Recipient not found in the store");
        }

        store
            .recipients
            .retain(|recipient| recipient.public_key != public_key);

        Configuration::encrypt_store(registration, &store)?;

        if !offline {
            synchronizer.push()?;
        }

        // logs::recipient_removed(public_key);
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}

pub fn list(
    configuration: &Configuration,
    store_name: Option<&String>,
    offline: bool,
) -> anyhow::Result<()> {
    if let Some(registration) = configuration.select_store(store_name) {
        if !offline {
            let store_path = Path::new(&registration.path);
            let synchronizer = registration.synchronizer.select_implementation(store_path);
            synchronizer.pull()?;
        }

        let store = configuration.decrypt_store(registration)?;

        for recipient in &store.recipients {
            println!(
                "{}",
                recipients::format_recipient(&recipient.public_key, &recipient.name)
            );
        }

        // logs::recipient_removed(public_key);
        Ok(())
    } else {
        anyhow::bail!(
            "No store found in configuration. Run 'pasejo store add ...' first to add one"
        )
    }
}
