// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Remote stores: creating and registering them, granting access, rekeying,
//! rotation lists and migrating a file store to a server.

use std::collections::BTreeMap;

use anyhow::Result;
use pasejo_client::store::{Changes, Snapshot};
use pasejo_protocol::Id;
use pasejo_protocol::api::Action;
use pasejo_protocol::log::{Principal, StoreRoleChange};

use crate::cli::i18n;
use crate::commands::org::{OrgContext, report};
use crate::commands::store_op::{open_store, require_file_store, require_store};
use crate::models::cli::{
    StoreAddRemoteArgs, StoreCreateRemoteArgs, StoreGrantArgs, StoreMigrateArgs, StoreRoleArg,
    StoreRotationArgs,
};
use crate::models::configuration::{Configuration, RemoteLocation, StoreRegistration};
use crate::remote::errors::Localize;
use crate::remote::funnel::{self, Opened, to_params};

/// `pasejo org stores`: every store this device sees, with its label when
/// the device holds its keys.
pub fn list(configuration: &Configuration, org: Option<&str>) -> Result<()> {
    let context = OrgContext::open(configuration, org)?;
    let identities = OrgContext::identities(configuration).unwrap_or_default();
    let stores = context.remote.client.stores(context.id()).localized()?;
    if stores.is_empty() {
        i18n::remote_no_stores();
    }
    for summary in stores {
        let label = context
            .remote
            .client
            .keys(&context.log, summary.id, &identities)
            .and_then(|ring| context.remote.client.index(&context.log, &ring, None))
            .map_or_default(|snapshot| snapshot.plain.label);
        let role = summary
            .role
            .map_or_else(String::new, |role| format!("{role:?}").to_ascii_lowercase());
        let local = configuration
            .stores
            .iter()
            .find(|store| {
                store
                    .remote
                    .as_ref()
                    .is_some_and(|remote| remote.store == summary.id.to_string())
            })
            .map_or_else(String::new, |store| store.name.clone());
        i18n::remote_store_line(&summary.id.to_string(), &label, &role, &local);
    }
    Ok(())
}

/// Registers `store` of `context` under `name` and saves the configuration.
fn register(
    configuration: &Configuration,
    context: &OrgContext,
    store: Id,
    name: &str,
    default: bool,
) -> Result<Configuration> {
    if configuration.canonical_store_name(name).is_some() {
        anyhow::bail!(i18n::error_remote_store_name_taken(name));
    }
    let mut configuration = configuration.clone();
    configuration.stores.push(StoreRegistration {
        path: context.remote.files.store_cache(store),
        name: name.to_owned(),
        remote: Some(RemoteLocation {
            server: context.remote.server.url.clone(),
            org: context.org.id.clone(),
            store: store.to_string(),
        }),
        ..StoreRegistration::default()
    });
    if default {
        configuration.default_store = Some(name.to_owned());
    }
    configuration.save_configuration()?;
    Ok(configuration)
}

pub fn create(configuration: &Configuration, args: &StoreCreateRemoteArgs) -> Result<()> {
    let mut context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let name = args.name.clone().unwrap_or_else(|| args.label.clone());
    if configuration.canonical_store_name(&name).is_some() {
        anyhow::bail!(i18n::error_remote_store_name_taken(&name));
    }
    let (ring, _) = context
        .remote
        .client
        .create_store(&mut context.log, &args.label)
        .localized()?;
    context.remote.files.save_log(context.id(), &context.log)?;
    register(configuration, &context, ring.store, &name, args.default)?;
    i18n::remote_store_created(&name, &ring.store.to_string());
    Ok(())
}

pub fn add(configuration: &Configuration, args: &StoreAddRemoteArgs) -> Result<()> {
    let context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let store = crate::remote::connect::parse_id(&args.store)?;
    if context
        .log
        .state()
        .stores
        .get(&store)
        .is_none_or(|state| state.deleted)
    {
        anyhow::bail!(i18n::error_remote_unknown_store(&args.store));
    }
    let mut configuration = register(configuration, &context, store, &args.name, args.default)?;
    // Opening the store once verifies that this device holds its keys; a
    // store that does not open is not kept.
    let registration = require_store(&configuration, Some(&args.name))?;
    let location = registration.remote.clone().unwrap_or_default();
    match funnel::connect(&configuration, registration, &location) {
        Ok(opened) => {
            i18n::remote_store_added(&args.name, &opened.snapshot.plain.label);
            Ok(())
        }
        Err(error) => {
            configuration.remove_store(&args.name)?;
            Err(error)
        }
    }
}

/// The remote store selected by `store_name`, connected.
fn connect(configuration: &Configuration, store_name: Option<&String>) -> Result<Opened> {
    let registration = require_store(configuration, store_name)?;
    let Some(location) = &registration.remote else {
        anyhow::bail!(i18n::error_remote_store_required(&registration.name));
    };
    funnel::connect(configuration, registration, location)
}

/// Runs the pending actions of `store`: wrapping keys for newly entitled
/// devices, and a new epoch once someone lost access.
fn settle(configuration: &Configuration, context: &mut OrgContext, store: Id) -> Result<()> {
    let identities = OrgContext::identities(configuration)?;
    let key = context.server_key()?;
    let outcomes = context
        .remote
        .client
        .run_actions_where(&mut context.log, &identities, &key, |action| match action {
            Action::WrapKeys { store: target, .. } | Action::RotateEpoch { store: target, .. } => {
                *target == store
            }
            _ => false,
        })
        .localized()?;
    context.remote.files.save_log(context.id(), &context.log)?;
    let users = context.remote.client.users(context.id()).localized()?;
    report(context, &outcomes, &users);
    Ok(())
}

pub fn grant(configuration: &Configuration, args: &StoreGrantArgs) -> Result<()> {
    let registration = require_store(configuration, args.store_selection.store.as_ref())?;
    let Some(location) = &registration.remote else {
        anyhow::bail!(i18n::error_remote_store_required(&registration.name));
    };
    let (_, org, store) = funnel::locate(configuration, location)?;
    let mut context = OrgContext::open(configuration, Some(&org.id))?;
    let principal = if args.principal.contains('@') {
        Principal::User(context.user(&args.principal)?)
    } else {
        Principal::Team(context.team(&args.principal)?)
    };
    let role = match args.role {
        StoreRoleArg::Reader => StoreRoleChange::Reader,
        StoreRoleArg::Writer => StoreRoleChange::Writer,
        StoreRoleArg::Admin => StoreRoleChange::Admin,
        StoreRoleArg::None => StoreRoleChange::None,
    };
    context.change(pasejo_protocol::log::Body::StoreGrant {
        store,
        principal,
        role,
        expires: args.expires,
    })?;
    settle(configuration, &mut context, store)
}

pub fn rekey(configuration: &Configuration, store_name: Option<&String>) -> Result<()> {
    let mut opened = connect(configuration, store_name)?;
    let next = opened
        .remote
        .client
        .rekey(&opened.log, &opened.ring, &opened.snapshot)
        .localized()?;
    let count = next.plain.secrets.len() + next.plain.otp.len();
    opened.advance(next)?;
    i18n::remote_store_rekeyed(&count.to_string());
    Ok(())
}

pub fn rotation(configuration: &Configuration, args: &StoreRotationArgs) -> Result<()> {
    let opened = connect(configuration, args.store_selection.store.as_ref())?;
    let context = OrgContext::open(configuration, Some(&opened.org.id))?;
    let user = context.user(&args.user)?;
    let rotation = opened
        .remote
        .client
        .rotation(&opened.log, opened.store, user)
        .localized()?;
    if rotation.ended.is_none() {
        i18n::remote_rotation_access_continues(&args.user);
        return Ok(());
    }
    let names = names_by_id(&opened.snapshot);
    let mut listed = 0;
    for item in rotation
        .items
        .iter()
        .filter(|item| item.fetched && !item.rewritten)
    {
        let name = names
            .get(&item.id)
            .cloned()
            .unwrap_or_else(|| item.id.to_string());
        i18n::remote_rotation_item(&name);
        listed += 1;
    }
    if listed == 0 {
        i18n::remote_rotation_nothing(&args.user);
    }
    Ok(())
}

fn names_by_id(snapshot: &Snapshot) -> BTreeMap<Id, String> {
    snapshot
        .plain
        .secrets
        .iter()
        .chain(snapshot.plain.otp.iter())
        .map(|(name, entry)| (entry.id, name.clone()))
        .collect()
}

pub fn migrate(
    configuration: &Configuration,
    args: &StoreMigrateArgs,
    offline: bool,
) -> Result<()> {
    let registration = require_store(configuration, args.store_selection.store.as_ref())?;
    require_file_store(registration)?;
    let name = registration.name.clone();
    let old_path = registration.path().display().to_string();
    let source = open_store(configuration, Some(&name), offline)?;
    let migrated_store = source.store.clone();
    drop(source);
    let mut context = OrgContext::open(configuration, Some(&args.to_remote))?;
    let (ring, snapshot) = context
        .remote
        .client
        .create_store(&mut context.log, &name)
        .localized()?;
    context.remote.files.save_log(context.id(), &context.log)?;
    let mut changes = Changes::default();
    for (secret, value) in &migrated_store.secrets {
        changes.secrets.insert(secret.clone(), Some(value.clone()));
    }
    for (otp, value) in &migrated_store.otp {
        changes.otp.insert(otp.clone(), Some(to_params(value)));
    }
    let committed = context
        .remote
        .client
        .commit(&context.log, &ring, &snapshot, &changes)
        .localized()?;
    let mut configuration = configuration.clone();
    let cache = context.remote.files.store_cache(ring.store);
    if let Some(entry) = configuration
        .stores
        .iter_mut()
        .find(|store| store.name == name)
    {
        entry.path = cache;
        entry.pull_commands.clear();
        entry.push_commands.clear();
        entry.remote = Some(RemoteLocation {
            server: context.remote.server.url.clone(),
            org: context.org.id.clone(),
            store: ring.store.to_string(),
        });
    }
    configuration.save_configuration()?;
    let migrated = committed.plain.secrets.len() + committed.plain.otp.len();
    i18n::remote_store_migrated(&name, &migrated.to_string(), &old_path);
    Ok(())
}
