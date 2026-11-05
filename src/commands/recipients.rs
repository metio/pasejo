// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::time::Duration;

use anyhow::{Context, Result};

use crate::cli::i18n;
use crate::commands::store_op::{OpenStore, StoreMutation, open_store, require_store, with_store};
use crate::hooks::executor::HookExecutor;
use crate::models::cli::RecipientCommands;
use crate::models::configuration::{
    Configuration, DEFAULT_KEY_DOWNLOAD_TIMEOUT_SECONDS, encrypt_store,
};
use crate::models::password_store::{PasswordStore, Recipient};
use crate::recipients;
use crate::recipients::public_key;

pub fn dispatch(
    command: &RecipientCommands,
    configuration: &Configuration,
    offline: bool,
) -> Result<()> {
    match command {
        RecipientCommands::Add(args) => {
            let download_timeout = Duration::from_secs(
                configuration
                    .key_download_timeout_seconds
                    .unwrap_or(DEFAULT_KEY_DOWNLOAD_TIMEOUT_SECONDS),
            );
            add(
                configuration,
                args.store_selection.store.as_ref(),
                &public_key::get(&args.keys, download_timeout)?,
                args.name.as_ref(),
                offline,
            )
        }
        RecipientCommands::Remove(args) => remove(
            configuration,
            args.store_selection.store.as_ref(),
            &args.public_key,
            args.ignore_unknown,
            offline,
        ),
        RecipientCommands::List(args) => {
            list(configuration, args.store_selection.store.as_ref(), offline)
        }
    }
}

/// Recipient `add` is the bootstrap path: if the store file does not yet
/// exist on disk we cannot pull or decrypt, so we start from an empty
/// `PasswordStore`. This makes it the one command that can't use
/// [`with_store`]; an existing store still goes through [`open_store`] so
/// the update happens under the store lock.
fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    public_keys: &[(String, String)],
    name: Option<&String>,
    offline: bool,
) -> Result<()> {
    let selected = require_store(configuration, store_name)?;
    let OpenStore {
        registration,
        hooks,
        lock,
        mut store,
    } = if selected.path().exists() {
        open_store(configuration, store_name, offline)?
    } else {
        OpenStore {
            registration: selected,
            hooks: HookExecutor::new(configuration, selected, offline),
            lock: None,
            store: PasswordStore::default(),
        }
    };

    for public_key in public_keys {
        if let Some(recipient) = store
            .recipients
            .iter_mut()
            .find(|recipient| public_key::normalize(&recipient.public_key) == public_key.0)
        {
            if let Some(name) = name {
                name.clone_into(&mut recipient.name);
            }
        } else {
            store.recipients.push(Recipient {
                public_key: public_key.0.clone(),
                name: name.map_or(&public_key.1, |value| value).clone(),
            });
        }
    }

    encrypt_store(registration, &store).context(i18n::error_cannot_encrypt_store())?;
    drop(lock);

    for public_key in public_keys {
        i18n::recipient_added(&public_key.0);
    }

    if configuration.identities.is_empty() && registration.identities.is_empty() {
        i18n::no_identities_exist_yet(&registration.name);
    }

    hooks.execute_push_commands()?;
    Ok(())
}

fn remove(
    configuration: &Configuration,
    store_name: Option<&String>,
    public_key: &str,
    ignore_unknown: bool,
    offline: bool,
) -> Result<()> {
    let public_key = public_key::normalize(public_key);
    let matches =
        |recipient: &Recipient| public_key::normalize(&recipient.public_key) == public_key;
    with_store(configuration, store_name, offline, |_, store| {
        if !store.recipients.is_empty() && store.recipients.iter().all(matches) {
            anyhow::bail!(i18n::error_cannot_remove_last_recipient())
        }
        if !store.recipients.iter().any(matches) {
            if ignore_unknown {
                i18n::recipient_does_not_exist_ignored(&public_key);
                return Ok(((), StoreMutation::Unchanged));
            }
            anyhow::bail!(i18n::error_recipient_not_found_in_store());
        }
        store.recipients.retain(|recipient| !matches(recipient));
        i18n::recipient_removed(&public_key);
        Ok(((), StoreMutation::Modified))
    })
}

fn list(configuration: &Configuration, store_name: Option<&String>, offline: bool) -> Result<()> {
    with_store(configuration, store_name, offline, |_, store| {
        for recipient in &store.recipients {
            println!(
                "{}",
                recipients::format_recipient(&recipient.public_key, &recipient.name)
            );
        }
        Ok(((), StoreMutation::Unchanged))
    })
}
