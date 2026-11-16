// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `pasejo audit`: printing an organization's audit events and verifying
//! their chain against the server's signed checkpoints.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use pasejo_protocol::encoding::{b64, unb64};
use pasejo_protocol::{Hash, Millis};
use serde::{Deserialize, Serialize};

use crate::cli::{atomic_write, i18n};
use crate::commands::org::OrgContext;
use crate::commands::store_op::require_store;
use crate::models::cli::{AuditCommands, AuditLogArgs};
use crate::models::configuration::Configuration;
use crate::remote::connect::parse_id;
use crate::remote::errors::Localize;

pub fn dispatch(command: &AuditCommands, configuration: &Configuration) -> Result<()> {
    match command {
        AuditCommands::Log(args) => log(configuration, args),
        AuditCommands::Verify(args) => verify(configuration, args.org.as_deref()),
    }
}

/// `time` as an RFC 3339 timestamp in UTC.
pub fn rfc3339(time: Millis) -> String {
    let seconds = time / 1000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let rest = seconds % 86_400;
    // Civil from days, after Howard Hinnant's algorithm.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn log(configuration: &Configuration, args: &AuditLogArgs) -> Result<()> {
    let store = match &args.store {
        Some(name) => {
            let registration = require_store(configuration, Some(name))?;
            let Some(location) = &registration.remote else {
                anyhow::bail!(i18n::error_remote_store_required(&registration.name));
            };
            Some(parse_id(&location.store)?)
        }
        None => None,
    };
    let context = OrgContext::open(configuration, args.org_selection.org.as_deref())?;
    let users = context.remote.client.users(context.id()).localized()?;
    let events = context
        .remote
        .client
        .audit_events(context.id(), args.since)
        .localized()?;
    for (from, until) in context.log.offline_read_periods() {
        match until {
            Some(until) => {
                i18n::remote_audit_offline_reads_closed(&rfc3339(from), &rfc3339(until));
            }
            None => i18n::remote_audit_offline_reads_open(&rfc3339(from)),
        }
    }
    for event in events
        .iter()
        .filter(|event| store.is_none() || event.store == store)
    {
        let actor = event.actor_user.map_or_default(|user| {
            context.describe(&pasejo_protocol::log::Principal::User(user), &users)
        });
        i18n::remote_audit_line(
            &event.seq.to_string(),
            &rfc3339(event.time),
            &actor,
            &event.action,
            &event.outcome,
            &event.store.map_or_default(|id| id.to_string()),
            event.detail.as_deref().unwrap_or_default(),
        );
    }
    Ok(())
}

/// The last event a verification covered, kept so the next one checks only
/// what came after.
#[derive(Serialize, Deserialize)]
struct Trusted {
    seq: u64,
    hash: String,
}

fn trusted_path(context: &OrgContext) -> Result<PathBuf> {
    let base = crate::cli::data_dir::local_data_dir()
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_no_data_dir()))?;
    Ok(base
        .join("servers")
        .join(&context.remote.server.device)
        .join("audit")
        .join(format!("{}.json", context.id())))
}

fn verify(configuration: &Configuration, org: Option<&str>) -> Result<()> {
    let context = OrgContext::open(configuration, org)?;
    let key = context.server_key()?;
    let path = trusted_path(&context)?;
    let trusted = fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Trusted>(&text).ok())
        .and_then(|trusted| {
            let hash: [u8; 32] = unb64(&trusted.hash).ok()?.try_into().ok()?;
            Some((trusted.seq, Hash(hash)))
        });
    let verified = context
        .remote
        .client
        .verify_audit(context.id(), &key, Some(&context.log), trusted)
        .localized()?;
    if let Some((seq, hash)) = verified.last {
        let text = serde_json::to_string(&Trusted {
            seq,
            hash: b64(&hash.0),
        })?;
        atomic_write::write(&path, text.as_bytes())?;
    }
    i18n::remote_audit_verified(
        &verified.events.to_string(),
        &verified.checkpoints.to_string(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_render_in_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_868_799_000), "2000-02-29T23:59:59Z");
        assert_eq!(rfc3339(1_794_873_600_000), "2026-11-17T00:00:00Z");
    }
}
