// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Opening a remote store as a `PasswordStore` and committing what a
//! command changed in it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use pasejo_client::keys::KeyRing;
use pasejo_client::store::{Changes, Kind, Snapshot};
use pasejo_protocol::Id;
use pasejo_protocol::api::ErrorCode;
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::store::{ObjectPlain, OtpParams, Pin};

use crate::cli::{atomic_write, i18n};
use crate::models::configuration::{
    Configuration, OrgRegistration, RemoteLocation, StoreRegistration,
};
use crate::models::password_store::{
    OneTimePassword, OneTimePasswordAlgorithm, OneTimePasswordType, PasswordStore,
};
use crate::remote::connect::{self, Remote, parse_id};
use crate::remote::errors::Localize;

/// What a command reads from a remote store. File stores read everything
/// regardless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fetch {
    /// Names only; every value is a placeholder.
    Names,
    /// The values of these entries; missing names are ignored.
    Named {
        secrets: Vec<String>,
        otp: Vec<String>,
    },
    /// Every value.
    Everything,
    /// The lookup and policy fields of every secret, from the lookup
    /// object, under a placeholder password. Commands reading this never
    /// write.
    Lookup,
}

impl Fetch {
    /// The values of the named secrets.
    pub fn secrets<I: IntoIterator<Item = S>, S: Into<String>>(names: I) -> Self {
        Self::Named {
            secrets: names.into_iter().map(Into::into).collect(),
            otp: Vec::new(),
        }
    }

    /// The values of the named one-time passwords.
    pub fn otp<I: IntoIterator<Item = S>, S: Into<String>>(names: I) -> Self {
        Self::Named {
            secrets: Vec::new(),
            otp: names.into_iter().map(Into::into).collect(),
        }
    }
}

/// The password line above a lookup text, which never carries the real
/// password.
const LOOKUP_PASSWORD: &str = "*";

/// A remote store opened for one command.
pub struct Opened {
    pub remote: Remote,
    pub org: OrgRegistration,
    pub store: Id,
    pub identities: Vec<Box<dyn age::Identity>>,
    pub log: OrgLog,
    pub ring: KeyRing,
    pub snapshot: Snapshot,
    base: PasswordStore,
    known: BTreeSet<(Kind, String)>,
    pin_path: PathBuf,
    read_only: bool,
}

/// The identifiers of the remote store at `location`.
///
/// # Errors
///
/// Fails when the server or organization is not registered.
pub fn locate<'a>(
    configuration: &'a Configuration,
    location: &RemoteLocation,
) -> Result<(
    &'a crate::models::configuration::ServerRegistration,
    &'a OrgRegistration,
    Id,
)> {
    let server = connect::server(configuration, Some(&location.server))?;
    let org = server
        .orgs
        .iter()
        .find(|org| org.id == location.org)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_org(&location.org)))?;
    Ok((server, org, parse_id(&location.store)?))
}

/// Connects to the remote store of `registration` and verifies its log,
/// keys and index, without fetching any value.
///
/// # Errors
///
/// Fails when the device is not signed in, the server refuses, or what it
/// returns does not verify.
pub fn connect(
    configuration: &Configuration,
    registration: &StoreRegistration,
    location: &RemoteLocation,
) -> Result<Opened> {
    let identities = configuration.read_identities(registration)?;
    connect_with(configuration, registration, location, identities)
}

/// [`connect`] with identities the caller has read.
///
/// # Errors
///
/// Fails like [`connect`].
pub fn connect_with(
    configuration: &Configuration,
    registration: &StoreRegistration,
    location: &RemoteLocation,
    identities: Vec<Box<dyn age::Identity>>,
) -> Result<Opened> {
    let (server, org, store) = locate(configuration, location)?;
    let remote = Remote::connect(server)?;
    let log = remote.log(org)?;
    let ring = remote.client.keys(&log, store, &identities).localized()?;
    fs::create_dir_all(registration.path()).with_context(|| {
        i18n::error_failed_to_create_directory(&registration.path().display().to_string())
    })?;
    let pin_path = registration.path().join("pin.json");
    let snapshot = remote
        .client
        .index(&log, &ring, read_pin(&pin_path)?)
        .localized()?;
    write_pin(&pin_path, snapshot.pin())?;
    let base = build(&snapshot, &BTreeMap::new())?;
    Ok(Opened {
        remote,
        org: org.clone(),
        store,
        identities,
        log,
        ring,
        snapshot,
        base,
        known: BTreeSet::new(),
        pin_path,
        read_only: false,
    })
}

/// Opens the remote store of `registration`, fetching what `fetch` names.
///
/// # Errors
///
/// Fails like [`connect`], or when an object does not verify.
pub fn open(
    configuration: &Configuration,
    registration: &StoreRegistration,
    location: &RemoteLocation,
    fetch: &Fetch,
) -> Result<(Opened, PasswordStore)> {
    let identities = configuration.read_identities(registration)?;
    open_with(configuration, registration, location, identities, fetch)
}

/// [`open`] with identities the caller has read.
///
/// # Errors
///
/// Fails like [`open`].
pub fn open_with(
    configuration: &Configuration,
    registration: &StoreRegistration,
    location: &RemoteLocation,
    identities: Vec<Box<dyn age::Identity>>,
    fetch: &Fetch,
) -> Result<(Opened, PasswordStore)> {
    let mut opened = connect_with(configuration, registration, location, identities)?;
    if *fetch == Fetch::Lookup {
        let lookup = opened
            .remote
            .client
            .lookup(&opened.log, &opened.ring, &opened.snapshot)
            .localized()?;
        opened.base = build(&opened.snapshot, &BTreeMap::new())?;
        for (name, text) in &lookup.secrets {
            if let Some(value) = opened.base.secrets.get_mut(name) {
                *value = format!("{LOOKUP_PASSWORD}\n{text}");
            }
        }
        opened.read_only = true;
        let store = opened.base.clone();
        return Ok((opened, store));
    }
    let snapshot = &opened.snapshot;
    let wanted: Vec<(Kind, String)> = match fetch {
        Fetch::Names | Fetch::Lookup => Vec::new(),
        Fetch::Named { secrets, otp } => secrets
            .iter()
            .map(|name| (Kind::Secret, name.clone()))
            .chain(otp.iter().map(|name| (Kind::Otp, name.clone())))
            .filter(|(kind, name)| exists(snapshot, *kind, name))
            .collect(),
        Fetch::Everything => snapshot
            .plain
            .secrets
            .keys()
            .map(|name| (Kind::Secret, name.clone()))
            .chain(
                snapshot
                    .plain
                    .otp
                    .keys()
                    .map(|name| (Kind::Otp, name.clone())),
            )
            .collect(),
    };
    let values = opened
        .remote
        .client
        .objects(&opened.log, &opened.ring, snapshot, &wanted)
        .localized()?;
    opened.base = build(snapshot, &values)?;
    opened.known = values.into_keys().collect();
    let store = opened.base.clone();
    Ok((opened, store))
}

fn exists(snapshot: &Snapshot, kind: Kind, name: &str) -> bool {
    match kind {
        Kind::Secret => snapshot.plain.secrets.contains_key(name),
        Kind::Otp => snapshot.plain.otp.contains_key(name),
    }
}

/// A `PasswordStore` holding every name of `snapshot`, with the fetched
/// values and placeholders for the rest.
fn build(
    snapshot: &Snapshot,
    values: &BTreeMap<(Kind, String), ObjectPlain>,
) -> Result<PasswordStore> {
    let mut store = PasswordStore::default();
    for name in snapshot.plain.secrets.keys() {
        let value = match values.get(&(Kind::Secret, name.clone())) {
            Some(ObjectPlain::Secret { secret }) => secret.clone(),
            Some(ObjectPlain::Otp { .. }) => {
                anyhow::bail!(i18n::error_remote_wrong_object_kind(name))
            }
            None => String::new(),
        };
        store.secrets.insert(name.clone(), value);
    }
    for name in snapshot.plain.otp.keys() {
        let value = match values.get(&(Kind::Otp, name.clone())) {
            Some(ObjectPlain::Otp { otp }) => from_params(otp, name)?,
            Some(ObjectPlain::Secret { .. }) => {
                anyhow::bail!(i18n::error_remote_wrong_object_kind(name))
            }
            None => OneTimePassword::default(),
        };
        store.otp.insert(name.clone(), value);
    }
    Ok(store)
}

fn read_pin(path: &Path) -> Result<Option<Pin>> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn write_pin(path: &Path, pin: Option<Pin>) -> Result<()> {
    atomic_write::write(path, serde_json::to_string(&pin)?.as_bytes())
}

/// Refuses recipients the server's wrappings could not be encrypted to.
///
/// # Errors
///
/// Fails for anything but an X25519, SSH or plugin recipient.
pub fn validate_recipient(recipient: &str) -> Result<()> {
    let valid = recipient.parse::<age::x25519::Recipient>().is_ok()
        || recipient.parse::<age::ssh::Recipient>().is_ok()
        || recipient.parse::<age::plugin::Recipient>().is_ok();
    if !valid {
        anyhow::bail!(i18n::error_remote_invalid_recipient(recipient));
    }
    Ok(())
}

/// The wire form of a one-time password.
pub fn to_params(otp: &OneTimePassword) -> OtpParams {
    OtpParams {
        secret: otp.secret.clone(),
        otp_type: match otp.otp_type {
            OneTimePasswordType::Totp => "totp",
            OneTimePasswordType::Hotp => "hotp",
        }
        .into(),
        algorithm: match otp.algorithm {
            OneTimePasswordAlgorithm::Sha1 => "sha1",
            OneTimePasswordAlgorithm::Sha256 => "sha256",
            OneTimePasswordAlgorithm::Sha512 => "sha512",
        }
        .into(),
        digits: otp.digits,
        period: otp.period,
        counter: otp.counter,
        skew: otp.skew,
    }
}

/// A one-time password from its wire form.
///
/// # Errors
///
/// Fails for unknown types and algorithms.
pub fn from_params(params: &OtpParams, name: &str) -> Result<OneTimePassword> {
    let otp_type = match params.otp_type.as_str() {
        "totp" => OneTimePasswordType::Totp,
        "hotp" => OneTimePasswordType::Hotp,
        _ => anyhow::bail!(i18n::error_remote_wrong_object_kind(name)),
    };
    let algorithm = match params.algorithm.as_str() {
        "sha1" => OneTimePasswordAlgorithm::Sha1,
        "sha256" => OneTimePasswordAlgorithm::Sha256,
        "sha512" => OneTimePasswordAlgorithm::Sha512,
        _ => anyhow::bail!(i18n::error_remote_wrong_object_kind(name)),
    };
    Ok(OneTimePassword {
        secret: params.secret.clone(),
        otp_type,
        algorithm,
        digits: params.digits,
        period: params.period,
        counter: params.counter,
        skew: params.skew,
    })
}

/// What changed from `base` to `current`. Values outside `known` were
/// never fetched; their placeholders count as unchanged. A removed entry
/// whose known value reappears under a new name becomes a rename, which
/// keeps its identifier.
pub fn diff(
    base: &PasswordStore,
    current: &PasswordStore,
    known: &BTreeSet<(Kind, String)>,
) -> Changes {
    let mut changes = Changes::default();
    let secrets = diff_map(&base.secrets, &current.secrets, known, Kind::Secret);
    let otp = diff_map(&base.otp, &current.otp, known, Kind::Otp);
    changes.renames.extend(secrets.renames);
    changes.renames.extend(otp.renames);
    changes.secrets = secrets.writes;
    changes.otp = otp
        .writes
        .into_iter()
        .map(|(name, value)| (name, value.map(|otp| to_params(&otp))))
        .collect();
    changes
}

struct MapDiff<V> {
    renames: Vec<(Kind, String, String)>,
    writes: BTreeMap<String, Option<V>>,
}

fn diff_map<V: Clone + PartialEq>(
    base: &BTreeMap<String, V>,
    current: &BTreeMap<String, V>,
    known: &BTreeSet<(Kind, String)>,
    kind: Kind,
) -> MapDiff<V> {
    let mut renames = Vec::new();
    let mut writes = BTreeMap::new();
    let mut added: Vec<&String> = current
        .keys()
        .filter(|name| !base.contains_key(*name))
        .collect();
    for (name, value) in base {
        match current.get(name) {
            Some(now) if now != value => {
                writes.insert(name.clone(), Some(now.clone()));
            }
            Some(_) => {}
            None => {
                let moved = known
                    .contains(&(kind, name.clone()))
                    .then(|| {
                        added
                            .iter()
                            .position(|candidate| current.get(*candidate) == Some(value))
                    })
                    .flatten();
                if let Some(index) = moved {
                    renames.push((kind, name.clone(), added.remove(index).clone()));
                } else {
                    writes.insert(name.clone(), None);
                }
            }
        }
    }
    for name in added {
        writes.insert(name.clone(), current.get(name).cloned());
    }
    MapDiff { renames, writes }
}

/// Every entry `changes` touches, under the name it has before them.
fn touched(changes: &Changes) -> Vec<(Kind, String)> {
    let mut names: Vec<(Kind, String)> = changes
        .secrets
        .keys()
        .map(|name| (Kind::Secret, name.clone()))
        .chain(changes.otp.keys().map(|name| (Kind::Otp, name.clone())))
        .collect();
    for (kind, from, to) in &changes.renames {
        names.push((*kind, from.clone()));
        names.push((*kind, to.clone()));
    }
    names
}

fn entry(
    snapshot: &Snapshot,
    kind: Kind,
    name: &str,
) -> Option<pasejo_protocol::store::IndexEntry> {
    match kind {
        Kind::Secret => snapshot.plain.secrets.get(name).cloned(),
        Kind::Otp => snapshot.plain.otp.get(name).cloned(),
    }
}

/// Refuses to reapply `changes` on `fresh` when another writer changed an
/// entry they touch since `old`.
///
/// # Errors
///
/// Fails naming the first such entry.
pub fn check_conflicts(old: &Snapshot, fresh: &Snapshot, changes: &Changes) -> Result<()> {
    for (kind, name) in touched(changes) {
        if entry(old, kind, &name) != entry(fresh, kind, &name) {
            anyhow::bail!(i18n::error_remote_concurrent_change(&name));
        }
    }
    Ok(())
}

impl Opened {
    /// Records a newer snapshot this command committed by other means.
    ///
    /// # Errors
    ///
    /// Fails when the pin cannot be written.
    pub fn advance(&mut self, snapshot: Snapshot) -> Result<()> {
        write_pin(&self.pin_path, snapshot.pin())?;
        self.snapshot = snapshot;
        Ok(())
    }

    /// Commits what changed from the opened content to `current`, retrying
    /// on a newer index when the changed entries are untouched there.
    /// Returns whether anything was committed.
    ///
    /// # Errors
    ///
    /// Fails when another writer changed the same entries, or the server
    /// refuses.
    pub fn save(&mut self, current: &PasswordStore) -> Result<bool> {
        let changes = diff(&self.base, current, &self.known);
        if changes.is_empty() {
            return Ok(false);
        }
        // Lookup texts stand in for values; writing them back would replace
        // every changed secret with its lookup fields.
        if self.read_only {
            anyhow::bail!(i18n::error_remote_unavailable());
        }
        for _ in 0..3 {
            match self
                .remote
                .client
                .commit(&self.log, &self.ring, &self.snapshot, &changes)
            {
                Ok(next) => {
                    self.advance(next)?;
                    self.base = current.clone();
                    return Ok(true);
                }
                Err(error) if error.is(ErrorCode::Conflict) => {
                    self.log = self.remote.log(&self.org)?;
                    self.ring = self
                        .remote
                        .client
                        .keys(&self.log, self.store, &self.identities)
                        .localized()?;
                    let fresh = self
                        .remote
                        .client
                        .index(&self.log, &self.ring, self.snapshot.pin())
                        .localized()?;
                    check_conflicts(&self.snapshot, &fresh, &changes)?;
                    self.snapshot = fresh;
                }
                Err(error) => return Err(error).localized(),
            }
        }
        anyhow::bail!(i18n::error_remote_conflict())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        store.secrets = secrets
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        store
    }

    fn known(names: &[&str]) -> BTreeSet<(Kind, String)> {
        names
            .iter()
            .map(|name| (Kind::Secret, (*name).to_owned()))
            .collect()
    }

    #[test]
    fn unchanged_stores_have_no_changes() {
        let base = store(&[("a", "1"), ("b", "")]);
        assert!(diff(&base, &base.clone(), &known(&["a"])).is_empty());
    }

    #[test]
    fn writes_additions_and_removals_are_detected() {
        let base = store(&[("a", "1"), ("b", "2"), ("c", "")]);
        let current = store(&[("a", "9"), ("c", ""), ("d", "4")]);
        let changes = diff(&base, &current, &known(&["a", "b"]));
        assert_eq!(changes.secrets["a"].as_deref(), Some("9"));
        assert_eq!(changes.secrets["b"], None);
        assert_eq!(changes.secrets["d"].as_deref(), Some("4"));
        assert!(!changes.secrets.contains_key("c"));
        assert!(changes.renames.is_empty());
    }

    #[test]
    fn a_known_value_under_a_new_name_is_a_rename() {
        let base = store(&[("old", "pw"), ("x", "")]);
        let current = store(&[("new", "pw"), ("x", "")]);
        let changes = diff(&base, &current, &known(&["old"]));
        assert_eq!(
            changes.renames,
            vec![(Kind::Secret, "old".to_owned(), "new".to_owned())]
        );
        assert!(changes.secrets.is_empty());
    }

    #[test]
    fn placeholders_never_turn_into_renames() {
        // `x` was not fetched, so its empty placeholder matching the new
        // empty secret says nothing about where the value went.
        let base = store(&[("x", "")]);
        let current = store(&[("y", "")]);
        let changes = diff(&base, &current, &known(&[]));
        assert!(changes.renames.is_empty());
        assert_eq!(changes.secrets["x"], None);
        assert_eq!(changes.secrets["y"].as_deref(), Some(""));
    }

    #[test]
    fn one_time_passwords_round_trip_through_their_wire_form() {
        let otp = OneTimePassword {
            secret: "JBSWY3DPEHPK3PXP".into(),
            otp_type: OneTimePasswordType::Hotp,
            algorithm: OneTimePasswordAlgorithm::Sha512,
            digits: 8,
            period: 30,
            counter: 7,
            skew: 1,
        };
        assert_eq!(from_params(&to_params(&otp), "x").unwrap(), otp);
        let mut bad = to_params(&otp);
        bad.algorithm = "md5".into();
        i18n::init_for_tests();
        assert!(from_params(&bad, "x").is_err());
    }

    #[test]
    fn otp_changes_are_converted() {
        let mut base = PasswordStore::default();
        let mut counted = OneTimePassword::default();
        counted.counter = 1;
        base.otp.insert("gh".into(), counted);
        let mut current = base.clone();
        current.otp.get_mut("gh").unwrap().counter = 2;
        let known = [(Kind::Otp, "gh".to_owned())].into();
        let changes = diff(&base, &current, &known);
        assert_eq!(changes.otp["gh"].as_ref().unwrap().counter, 2);
    }

    fn snapshot(entries: &[(&str, u64)]) -> Snapshot {
        let mut plain = pasejo_protocol::store::IndexPlain::default();
        for (name, version) in entries {
            plain.secrets.insert(
                (*name).to_owned(),
                pasejo_protocol::store::IndexEntry {
                    id: Id([1; 16]),
                    version: *version,
                    hash: pasejo_protocol::Hash([0; 32]),
                },
            );
        }
        Snapshot {
            store: Id([0; 16]),
            version: 1,
            head: None,
            epoch: 0,
            lookup_hash: None,
            plain,
        }
    }

    #[test]
    fn concurrent_changes_to_touched_entries_conflict() {
        i18n::init_for_tests();
        let old = snapshot(&[("a", 1), ("b", 1)]);
        let mut changes = Changes::default();
        changes.secrets.insert("a".into(), Some("x".into()));
        assert!(
            check_conflicts(&old, &snapshot(&[("a", 1), ("b", 2), ("c", 1)]), &changes).is_ok()
        );
        let error = check_conflicts(&old, &snapshot(&[("a", 2), ("b", 1)]), &changes).unwrap_err();
        assert!(error.to_string().contains('a'));
        let mut adding = Changes::default();
        adding.secrets.insert("c".into(), Some("x".into()));
        assert!(
            check_conflicts(&old, &snapshot(&[("a", 1), ("b", 1), ("c", 1)]), &adding).is_err()
        );
        let renaming = Changes {
            renames: vec![(Kind::Secret, "b".into(), "d".into())],
            ..Changes::default()
        };
        assert!(check_conflicts(&old, &snapshot(&[("a", 1)]), &renaming).is_err());
    }
}
