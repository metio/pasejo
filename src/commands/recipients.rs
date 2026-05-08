// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::logs;
use crate::commands::store_op::{NO_STORE_FOUND_ERROR, StoreMutation, with_store};
use crate::hooks::executor::HookExecutor;
use crate::models::cli::RecipientCommands;
use crate::models::configuration::{Configuration, encrypt_store};
use crate::models::password_store::{PasswordStore, Recipient};
use crate::recipients;
use crate::recipients::public_key;
use anyhow::{Context, Result};

pub fn dispatch(
    command: &RecipientCommands,
    configuration: &Configuration,
    offline: bool,
) -> Result<()> {
    match command {
        RecipientCommands::Add(args) => add(
            configuration,
            args.store_selection.store.as_ref(),
            &public_key::get(&args.keys)?,
            args.name.as_ref(),
            offline,
        ),
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
/// [`with_store`].
fn add(
    configuration: &Configuration,
    store_name: Option<&String>,
    public_keys: &[(String, String)],
    name: Option<&String>,
    offline: bool,
) -> Result<()> {
    let registration = configuration
        .select_store(store_name)
        .context(NO_STORE_FOUND_ERROR)?;
    let store_path = registration.path();
    let hooks = HookExecutor {
        configuration,
        registration,
        offline,
        force: false,
    };

    let mut store = if store_path.exists() {
        hooks.execute_pull_commands()?;
        configuration.decrypt_store(registration)?
    } else {
        PasswordStore::default()
    };

    for public_key in public_keys {
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
                name: name.map_or(&public_key.1, |value| value).clone(),
            });
        }
    }

    encrypt_store(registration, &store).context("Cannot encrypt store")?;

    for public_key in public_keys {
        logs::recipient_added(&public_key.0);
    }

    if configuration.identities.is_empty() && registration.identities.is_empty() {
        logs::no_identities_exist_yet(&registration.name);
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
    with_store(configuration, store_name, offline, |_, store| {
        if store.recipients.len() == 1 && store.recipients[0].public_key == public_key {
            anyhow::bail!(
                "Cannot remove the last recipient from the store. Please add a new recipient before removing this one."
            )
        }
        if !store
            .recipients
            .iter()
            .any(|recipient| recipient.public_key == public_key)
        {
            if ignore_unknown {
                logs::recipient_does_not_exist_ignored(public_key);
                return Ok(((), StoreMutation::Unchanged));
            }
            anyhow::bail!("Recipient not found in the store");
        }
        store
            .recipients
            .retain(|recipient| recipient.public_key != public_key);
        logs::recipient_removed(public_key);
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
