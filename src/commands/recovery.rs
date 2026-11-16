// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `pasejo org recover`: decrypting every remote store with the
//! organization's offline recovery identity and writing each one as a file
//! store encrypted to the recovery recipient.

use std::fs;

use anyhow::Result;
use pasejo_client::store::{Kind, recover as recover_store};
use pasejo_protocol::store::ObjectPlain;

use crate::cli::i18n;
use crate::commands::org::OrgContext;
use crate::models::cli::OrgRecoverArgs;
use crate::models::configuration::{Configuration, encrypt_store_to_path};
use crate::models::password_store::{PasswordStore, Recipient};
use crate::remote::errors::Localize;
use crate::remote::funnel::from_params;

pub fn recover(configuration: &Configuration, args: &OrgRecoverArgs) -> Result<()> {
    let context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let Some(recipient) = context.log.state().recovery.clone() else {
        anyhow::bail!(i18n::error_remote_no_recovery_recipient(&context.org.name));
    };
    let identities = configuration.read_identity_files(vec![args.identity.clone()])?;
    let export = context
        .remote
        .client
        .recovery_export(context.id())
        .localized()?;
    let output = args.output.display().to_string();
    fs::create_dir_all(&args.output)
        .map_err(|_| anyhow::anyhow!(i18n::error_failed_to_create_directory(&output)))?;
    for exported in &export.stores {
        let recovered = recover_store(&context.log, exported, &identities).localized()?;
        let mut store = PasswordStore::default();
        store.recipients.push(Recipient {
            name: "recovery".into(),
            public_key: recipient.clone(),
        });
        for ((kind, name), value) in &recovered.values {
            match (kind, value) {
                (Kind::Secret, ObjectPlain::Secret { secret }) => {
                    store.secrets.insert(name.clone(), secret.clone());
                }
                (Kind::Otp, ObjectPlain::Otp { otp }) => {
                    store.otp.insert(name.clone(), from_params(otp, name)?);
                }
                _ => anyhow::bail!(i18n::error_remote_wrong_object_kind(name)),
            }
        }
        let path = args.output.join(format!("{}.age", recovered.store));
        encrypt_store_to_path(&store, &path)?;
        i18n::remote_store_recovered(
            &recovered.label,
            &path.display().to_string(),
            &recovered.values.len().to_string(),
        );
    }
    Ok(())
}
