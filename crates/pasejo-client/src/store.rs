// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Remote stores: the verified index, the lookup object, secret objects,
//! commits built from a set of changes, epochs and wrappings.

use std::collections::{BTreeMap, BTreeSet};

use pasejo_protocol::api::{
    ChainLink, Commit, Committed, EpochUpload, ErrorCode, IndexResponse, LookupResponse, ObjectRef,
    ObjectsRequest, ObjectsResponse, Rotation, StoreSummary, Stores, WrappingUpload,
    WrappingsUpload,
};
use pasejo_protocol::crypto::{EpochKeys, data_commitment, names_commitment, seal_chain};
use pasejo_protocol::device::{DeviceKind, recovery_id};
use pasejo_protocol::encoding::{Bytes, Hash, unb64};
use pasejo_protocol::log::{Body, OrgLog, OrgState, Principal, StoreRoleChange};
use pasejo_protocol::store::{
    IndexEntry, IndexPayload, IndexPlain, LookupPlain, ObjectPlain, OtpParams, Pin, SealedObject,
    StoreSettings, check_chain, open_index, open_lookup, open_object, seal_index, seal_lookup,
    seal_object, sign_index,
};
use pasejo_protocol::wrapping::{wrap_age, wrap_hpke};
use pasejo_protocol::{Id, Millis};

use crate::error::{Error, Result};
use crate::keys::KeyRing;
use crate::transport::Client;

/// The browser policy fields the lookup object carries beside the lookup
/// keys.
pub const POLICY_KEYS: [&str; 2] = ["pasejo-fill", "pasejo-confirm"];

/// A verified, decrypted index version; version 0 is the empty store
/// before its first commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub store: Id,
    pub version: u64,
    pub head: Option<Hash>,
    pub epoch: u64,
    pub lookup_hash: Option<Hash>,
    pub plain: IndexPlain,
}

impl Snapshot {
    /// The pin a client keeps to refuse an index that goes backwards.
    #[must_use]
    pub fn pin(&self) -> Option<Pin> {
        self.head.map(|hash| Pin {
            version: self.version,
            hash,
        })
    }

    fn empty(store: Id, epoch: u64) -> Self {
        Self {
            store,
            version: 0,
            head: None,
            epoch,
            lookup_hash: None,
            plain: IndexPlain::default(),
        }
    }
}

/// Which map an entry lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Secret,
    Otp,
}

/// Changes to apply in one commit.
#[derive(Debug, Clone, Default)]
pub struct Changes {
    /// Secrets to write (`Some`) or remove (`None`).
    pub secrets: BTreeMap<String, Option<String>>,
    /// One-time passwords to write (`Some`) or remove (`None`).
    pub otp: BTreeMap<String, Option<OtpParams>>,
    /// Entries to move to a new name, keeping their identifier. Applied
    /// before writes and removals, which then refer to the new names.
    pub renames: Vec<(Kind, String, String)>,
    pub label: Option<String>,
    pub settings: Option<StoreSettings>,
    /// Entries to rewrite unchanged under the current epoch.
    pub rewrite: BTreeSet<(Kind, String)>,
}

impl Changes {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
            && self.otp.is_empty()
            && self.renames.is_empty()
            && self.label.is_none()
            && self.settings.is_none()
            && self.rewrite.is_empty()
    }
}

/// The lookup text of a secret: its lookup and policy fields, under a blank
/// first line where the password was, so the secret parser reads it as a
/// secret without a password.
#[must_use]
pub fn lookup_text(secret: &str, settings: &StoreSettings) -> String {
    let wanted = |key: &str| {
        let key = key.trim().to_ascii_lowercase();
        settings
            .uri_keys
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(&key))
            || settings
                .username_keys
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&key))
            || POLICY_KEYS.contains(&key.as_str())
    };
    let mut text = String::new();
    for line in secret.lines().skip(1) {
        if let Some((key, _)) = line.split_once(':')
            && wanted(key)
        {
            text.push('\n');
            text.push_str(line);
        }
    }
    text
}

const fn entries_mut(plain: &mut IndexPlain, kind: Kind) -> &mut BTreeMap<String, IndexEntry> {
    match kind {
        Kind::Secret => &mut plain.secrets,
        Kind::Otp => &mut plain.otp,
    }
}

const fn entries(plain: &IndexPlain, kind: Kind) -> &BTreeMap<String, IndexEntry> {
    match kind {
        Kind::Secret => &plain.secrets,
        Kind::Otp => &plain.otp,
    }
}

/// An index version ready to sign, with the objects and lookup it needs.
#[derive(Debug, Clone)]
pub struct Draft {
    pub payload: IndexPayload,
    pub plain: IndexPlain,
    pub lookup: Option<Vec<u8>>,
    pub objects: Vec<SealedObject>,
}

/// Builds the next index version from `snapshot` and `changes`.
///
/// `lookup` is the current lookup plaintext; `values` holds the current
/// values of entries to rewrite. Writes and rewrites are sealed under the
/// ring's current epoch.
///
/// # Errors
///
/// Fails for names-only rings, renames of missing entries, renames onto
/// taken names, and rewrites without a value.
#[allow(clippy::too_many_arguments)]
pub fn draft(
    log: &OrgLog,
    ring: &KeyRing,
    snapshot: &Snapshot,
    mut lookup: LookupPlain,
    changes: &Changes,
    values: &BTreeMap<(Kind, String), ObjectPlain>,
    time: Millis,
) -> Result<Draft> {
    let store = snapshot.store;
    let epoch = ring.current;
    let keys = ring.current();
    let data = keys.data()?;
    let version = snapshot.version + 1;
    let mut plain = snapshot.plain.clone();
    if let Some(label) = &changes.label {
        plain.label.clone_from(label);
    }
    if let Some(settings) = &changes.settings {
        plain.settings = settings.clone();
    }
    apply_renames(&mut plain, &mut lookup, &changes.renames)?;
    let mut sealer = Sealer {
        store,
        epoch,
        data,
        objects: Vec::new(),
        deleted: Vec::new(),
    };
    for (name, value) in &changes.secrets {
        if let Some(secret) = value {
            let object = ObjectPlain::Secret {
                secret: secret.clone(),
            };
            sealer.seal(&mut plain, Kind::Secret, name, &object)?;
            lookup
                .secrets
                .insert(name.clone(), lookup_text(secret, &plain.settings));
        } else {
            sealer.remove(&mut plain, Kind::Secret, name);
        }
    }
    for (name, value) in &changes.otp {
        if let Some(otp) = value {
            sealer.seal(
                &mut plain,
                Kind::Otp,
                name,
                &ObjectPlain::Otp { otp: otp.clone() },
            )?;
        } else {
            sealer.remove(&mut plain, Kind::Otp, name);
        }
    }
    for (kind, name) in &changes.rewrite {
        let written = match kind {
            Kind::Secret => changes.secrets.contains_key(name),
            Kind::Otp => changes.otp.contains_key(name),
        };
        if written || !entries(&plain, *kind).contains_key(name) {
            continue;
        }
        let value = values
            .get(&(*kind, name.clone()))
            .ok_or_else(|| Error::Unavailable(format!("no value of {name} to rewrite")))?;
        sealer.seal(&mut plain, *kind, name, value)?;
        // New lookup keys change every secret's lookup text, so callers
        // changing settings rewrite every secret.
        if let ObjectPlain::Secret { secret } = value {
            lookup
                .secrets
                .insert(name.clone(), lookup_text(secret, &plain.settings));
        }
    }
    lookup
        .secrets
        .retain(|name, _| plain.secrets.contains_key(name));
    let sealed_lookup = seal_lookup(store, epoch, version, data, &lookup)?;
    let payload = IndexPayload {
        store,
        version,
        prev: snapshot.head,
        epoch,
        log_head: log.head(),
        time,
        index: Bytes(seal_index(store, epoch, version, &keys.names, &plain)?),
        lookup_hash: Some(Hash::of(&sealed_lookup)),
        changed: sealer.objects.iter().map(|object| object.id).collect(),
        deleted: sealer.deleted,
    };
    Ok(Draft {
        payload,
        plain,
        lookup: Some(sealed_lookup),
        objects: sealer.objects,
    })
}

/// Moves entries to new names, keeping their identifiers, versions and
/// lookup texts.
fn apply_renames(
    plain: &mut IndexPlain,
    lookup: &mut LookupPlain,
    renames: &[(Kind, String, String)],
) -> Result<()> {
    for (kind, from, to) in renames {
        let map = entries_mut(plain, *kind);
        if map.contains_key(to) {
            return Err(Error::Unavailable(format!("{to} already exists")));
        }
        let entry = map
            .remove(from)
            .ok_or_else(|| Error::Unavailable(format!("{from} does not exist")))?;
        map.insert(to.clone(), entry);
        if *kind == Kind::Secret
            && let Some(text) = lookup.secrets.remove(from)
        {
            lookup.secrets.insert(to.clone(), text);
        }
    }
    Ok(())
}

/// Seals objects of one draft and collects what the commit lists.
struct Sealer<'a> {
    store: Id,
    epoch: u64,
    data: &'a pasejo_protocol::crypto::Key,
    objects: Vec<SealedObject>,
    deleted: Vec<Id>,
}

impl Sealer<'_> {
    /// Seals `object` as the next version of `name`, or as a new entry.
    fn seal(
        &mut self,
        plain: &mut IndexPlain,
        kind: Kind,
        name: &str,
        object: &ObjectPlain,
    ) -> Result<()> {
        let map = entries_mut(plain, kind);
        let (id, version) = map
            .get(name)
            .map_or_else(|| (Id::random(), 1), |entry| (entry.id, entry.version + 1));
        let ciphertext = seal_object(self.store, self.epoch, id, version, self.data, object)?;
        map.insert(
            name.to_owned(),
            IndexEntry {
                id,
                version,
                hash: Hash::of(&ciphertext),
            },
        );
        self.objects.push(SealedObject {
            id,
            version,
            epoch: self.epoch,
            ciphertext: Bytes(ciphertext),
        });
        Ok(())
    }

    fn remove(&mut self, plain: &mut IndexPlain, kind: Kind, name: &str) {
        if let Some(entry) = entries_mut(plain, kind).remove(name) {
            self.deleted.push(entry.id);
        }
    }
}

/// The wrapping of `keys` for `device`, full or names-only as the log
/// entitles it.
///
/// # Errors
///
/// Fails for unknown devices and unusable recipients.
pub fn wrapping_for(
    state: &OrgState,
    store: Id,
    epoch: u64,
    keys: &EpochKeys,
    device: Id,
    full: bool,
) -> Result<WrappingUpload> {
    if state.is_recovery(device) {
        let recipient = state.recovery.as_deref().unwrap_or_default();
        let parsed = crate::recipient::parse(recipient)?;
        return Ok(WrappingUpload {
            device,
            full: true,
            ciphertext: Bytes(wrap_age(store, epoch, keys, parsed.as_ref())?),
        });
    }
    let record = state
        .active_device(device)
        .ok_or_else(|| Error::Unavailable(format!("device {device} is not active")))?;
    let ciphertext = match record.kind {
        DeviceKind::Age => {
            let parsed = crate::recipient::parse(&record.recipient)?;
            let wrapped = if full {
                keys.clone()
            } else {
                keys.names_only()
            };
            wrap_age(store, epoch, &wrapped, parsed.as_ref())?
        }
        DeviceKind::Hpke => {
            let public: [u8; 32] = unb64(&record.recipient)?
                .try_into()
                .map_err(|_| Error::verification("malformed hpke recipient"))?;
            wrap_hpke(store, epoch, device, &public, &keys.names_only())?
        }
    };
    Ok(WrappingUpload {
        device,
        full: full && record.kind == DeviceKind::Age,
        ciphertext: Bytes(ciphertext),
    })
}

impl Client {
    fn store_path(log: &OrgLog, store: Id, rest: &str) -> String {
        format!("/v1/orgs/{}/stores/{store}/{rest}", log.state().org)
    }

    /// Stores the session may see.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn stores(&self, org: Id) -> Result<Vec<StoreSummary>> {
        Ok(self
            .get::<Stores>(&format!("/v1/orgs/{org}/stores"))?
            .stores)
    }

    /// Fetches, verifies and decrypts the current index.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, or the index does not verify against
    /// `log`, goes backwards from `pinned` or does not decrypt.
    pub fn index(&self, log: &OrgLog, ring: &KeyRing, pinned: Option<Pin>) -> Result<Snapshot> {
        let store = ring.store;
        let response: IndexResponse = self.get(&Self::store_path(log, store, "index"))?;
        let Some(envelope) = response.index else {
            if pinned.is_some() {
                return Err(Error::verification("the store's index disappeared"));
            }
            return Ok(Snapshot::empty(store, ring.current));
        };
        let payload = log.check_index(&envelope)?;
        check_chain(&envelope, &payload, store, pinned)?;
        let keys = ring.epoch(payload.epoch)?;
        let plain = open_index(
            store,
            payload.epoch,
            payload.version,
            &keys.names,
            &payload.index.0,
        )?;
        Ok(Snapshot {
            store,
            version: payload.version,
            head: Some(envelope.hash()),
            epoch: payload.epoch,
            lookup_hash: payload.lookup_hash,
            plain,
        })
    }

    /// Fetches and decrypts the lookup object of `snapshot`.
    ///
    /// # Errors
    ///
    /// Fails when the lookup object does not match the index.
    pub fn lookup(&self, log: &OrgLog, ring: &KeyRing, snapshot: &Snapshot) -> Result<LookupPlain> {
        if snapshot.version == 0 {
            return Ok(LookupPlain::default());
        }
        let response: LookupResponse =
            self.get(&Self::store_path(log, snapshot.store, "lookup"))?;
        if response.version != snapshot.version {
            return Err(Error::Api {
                status: 409,
                code: Some(ErrorCode::Conflict),
                message: "the store moved on".into(),
                current_version: Some(response.version),
            });
        }
        match (response.lookup, snapshot.lookup_hash) {
            (None, None) => Ok(LookupPlain::default()),
            (Some(sealed), Some(hash)) if Hash::of(&sealed.0) == hash => Ok(open_lookup(
                snapshot.store,
                snapshot.epoch,
                snapshot.version,
                ring.epoch(snapshot.epoch)?.data()?,
                &sealed.0,
            )?),
            _ => Err(Error::verification(
                "the lookup object does not match the index",
            )),
        }
    }

    /// Fetches and decrypts the named entries of `snapshot`.
    ///
    /// # Errors
    ///
    /// Fails for unknown names, objects whose hash differs from the index,
    /// and objects that do not decrypt.
    pub fn objects(
        &self,
        log: &OrgLog,
        ring: &KeyRing,
        snapshot: &Snapshot,
        names: &[(Kind, String)],
    ) -> Result<BTreeMap<(Kind, String), ObjectPlain>> {
        if names.is_empty() {
            return Ok(BTreeMap::new());
        }
        let mut wanted = BTreeMap::new();
        for (kind, name) in names {
            let entry = entries(&snapshot.plain, *kind)
                .get(name)
                .ok_or_else(|| Error::Unavailable(format!("{name} does not exist")))?;
            wanted.insert(
                (entry.id, entry.version),
                ((*kind, name.clone()), entry.hash),
            );
        }
        let request = ObjectsRequest {
            objects: wanted
                .keys()
                .map(|(id, version)| ObjectRef {
                    id: *id,
                    version: *version,
                })
                .collect(),
        };
        let response: ObjectsResponse = self.call(
            "POST",
            &Self::store_path(log, snapshot.store, "objects"),
            &request,
        )?;
        let mut values = BTreeMap::new();
        for object in response.objects {
            let Some((key, hash)) = wanted.remove(&(object.id, object.version)) else {
                return Err(Error::verification(
                    "the server sent an object nobody asked for",
                ));
            };
            if Hash::of(&object.ciphertext.0) != hash {
                return Err(Error::verification(format!(
                    "{} differs from the index",
                    key.1
                )));
            }
            let data = ring.epoch(object.epoch)?.data()?;
            let opened = open_object(
                snapshot.store,
                object.epoch,
                object.id,
                object.version,
                data,
                &object.ciphertext.0,
            )?;
            values.insert(key, opened);
        }
        if let Some((_, (key, _))) = wanted.into_iter().next() {
            return Err(Error::verification(format!(
                "the server withheld {}",
                key.1
            )));
        }
        Ok(values)
    }

    /// Commits `changes` on top of `snapshot` and returns the new version.
    ///
    /// # Errors
    ///
    /// Fails with a `conflict` API error when the store moved on; callers
    /// fetch the new index and reapply their changes.
    pub fn commit(
        &self,
        log: &OrgLog,
        ring: &KeyRing,
        snapshot: &Snapshot,
        changes: &Changes,
    ) -> Result<Snapshot> {
        let lookup = self.lookup(log, ring, snapshot)?;
        let values = if changes.rewrite.is_empty() {
            BTreeMap::new()
        } else {
            let names: Vec<(Kind, String)> = changes
                .rewrite
                .iter()
                .filter(|(kind, name)| entries(&snapshot.plain, *kind).contains_key(name))
                .cloned()
                .collect();
            self.objects(log, ring, snapshot, &names)?
        };
        let draft = draft(log, ring, snapshot, lookup, changes, &values, self.now())?;
        let commit = Commit {
            index: sign_index(&draft.payload, self.device.id, &self.device.signing_key)?,
            lookup: draft.lookup.clone().map(Bytes),
            objects: draft.objects,
        };
        let committed: Committed = self.call(
            "POST",
            &Self::store_path(log, snapshot.store, "commits"),
            &commit,
        )?;
        Ok(Snapshot {
            store: snapshot.store,
            version: committed.version,
            head: Some(commit.index.hash()),
            epoch: draft.payload.epoch,
            lookup_hash: draft.payload.lookup_hash,
            plain: draft.plain,
        })
    }

    /// Rewrites every entry under the current epoch.
    ///
    /// # Errors
    ///
    /// Fails like [`Client::commit`].
    pub fn rekey(&self, log: &OrgLog, ring: &KeyRing, snapshot: &Snapshot) -> Result<Snapshot> {
        let mut changes = Changes::default();
        for name in snapshot.plain.secrets.keys() {
            changes.rewrite.insert((Kind::Secret, name.clone()));
        }
        for name in snapshot.plain.otp.keys() {
            changes.rewrite.insert((Kind::Otp, name.clone()));
        }
        self.commit(log, ring, snapshot, &changes)
    }

    /// Creates a store, its first epoch and its first index version.
    ///
    /// # Errors
    ///
    /// Fails when the log or the server refuses a step.
    pub fn create_store(&self, log: &mut OrgLog, label: &str) -> Result<(KeyRing, Snapshot)> {
        let store = Id::random();
        self.append_fresh(log, Body::StoreCreate { store })?;
        let ring = self.start_epoch(log, store, None)?;
        let changes = Changes {
            label: Some(label.to_owned()),
            ..Changes::default()
        };
        let snapshot = self.commit(log, &ring, &Snapshot::empty(store, ring.current), &changes)?;
        Ok((ring, snapshot))
    }

    /// Generates the store's next epoch, records it in the log for the
    /// devices entitled now and uploads its chain link and wrappings.
    ///
    /// # Errors
    ///
    /// Fails without the previous epoch's full keys, or when the log or the
    /// server refuses.
    pub fn start_epoch(
        &self,
        log: &mut OrgLog,
        store: Id,
        previous: Option<&KeyRing>,
    ) -> Result<KeyRing> {
        let state = log.state();
        let current = state
            .stores
            .get(&store)
            .and_then(pasejo_protocol::log::StoreState::current_epoch);
        let epoch = current.map_or(0, |epoch| epoch + 1);
        if current.is_some() && previous.is_none_or(|ring| ring.current != current.unwrap_or(0)) {
            return Err(Error::Unavailable(
                "the current epoch's keys are needed".into(),
            ));
        }
        let keys = EpochKeys::generate();
        let entitled = state.entitled(store, self.now());
        let body = Body::StoreEpoch {
            store,
            epoch,
            names_commit: names_commitment(store, epoch, &keys.names),
            data_commit: data_commitment(store, epoch, keys.data()?),
            full: entitled.full.iter().copied().collect(),
            names_only: entitled.names_only.iter().copied().collect(),
        };
        self.append_fresh(log, body)?;
        let chain = match previous {
            Some(ring) => {
                let chain = seal_chain(store, epoch, &keys, ring.current())?;
                Some(ChainLink {
                    epoch,
                    prev_names: Bytes(chain.prev_names),
                    prev_data: Bytes(chain.prev_data),
                })
            }
            None => None,
        };
        let state = log.state();
        let mut wrappings = Vec::new();
        for device in &entitled.full {
            wrappings.push(wrapping_for(state, store, epoch, &keys, *device, true)?);
        }
        for device in &entitled.names_only {
            wrappings.push(wrapping_for(state, store, epoch, &keys, *device, false)?);
        }
        self.call_empty(
            "PUT",
            &Self::store_path(log, store, &format!("epochs/{epoch}")),
            Some(&EpochUpload { chain, wrappings }),
        )?;
        Ok(KeyRing::generated(store, epoch, keys, previous))
    }

    /// Uploads wrappings of the current epoch for `devices`.
    ///
    /// # Errors
    ///
    /// Fails for devices the log does not entitle, or when the server
    /// refuses.
    pub fn wrap(&self, log: &OrgLog, ring: &KeyRing, devices: &[Id]) -> Result<()> {
        let entitled = log.state().entitled(ring.store, self.now());
        let mut wrappings = Vec::new();
        for device in devices {
            let full = entitled.full.contains(device);
            if !full && !entitled.names_only.contains(device) {
                return Err(Error::Unavailable(format!(
                    "device {device} is not entitled"
                )));
            }
            if full && !ring.full() {
                return Err(Error::Unavailable(
                    "this device holds names-only keys".into(),
                ));
            }
            wrappings.push(wrapping_for(
                log.state(),
                ring.store,
                ring.current,
                ring.current(),
                *device,
                full,
            )?);
        }
        self.call_empty(
            "POST",
            &Self::store_path(log, ring.store, "wrappings"),
            Some(&WrappingsUpload {
                epoch: ring.current,
                wrappings,
            }),
        )
    }

    /// Grants `principal` a role on `store`, or takes it away with `None`.
    ///
    /// # Errors
    ///
    /// Fails for non-admins of the store.
    pub fn grant(
        &self,
        log: &mut OrgLog,
        store: Id,
        principal: Principal,
        role: StoreRoleChange,
        expires: Option<Millis>,
    ) -> Result<()> {
        self.append_fresh(
            log,
            Body::StoreGrant {
                store,
                principal,
                role,
                expires,
            },
        )?;
        Ok(())
    }

    /// The rotation list of `user` for `store`.
    ///
    /// # Errors
    ///
    /// Fails for non-admins of the store.
    pub fn rotation(&self, log: &OrgLog, store: Id, user: Id) -> Result<Rotation> {
        self.get(&Self::store_path(
            log,
            store,
            &format!("rotation?user={user}"),
        ))
    }
}

/// The identifier the log uses for the organization's recovery recipient.
#[must_use]
pub fn recovery_device(recipient: &str) -> Id {
    recovery_id(recipient)
}

#[cfg(test)]
mod tests {
    use pasejo_protocol::store::{open_index, open_lookup, open_object};

    use super::*;
    use crate::fixtures::{Fixture, TIME};

    fn ring(fixture: &Fixture) -> KeyRing {
        KeyRing::open(
            &fixture.log,
            fixture.store,
            fixture.device.id,
            &fixture.keys(true),
            &fixture.identities(),
        )
        .unwrap()
    }

    fn write(changes: &mut Changes, name: &str, value: &str) {
        changes.secrets.insert(name.into(), Some(value.into()));
    }

    /// Applies a draft as the server would and returns the next snapshot.
    fn apply(snapshot: &Snapshot, draft: &Draft) -> Snapshot {
        Snapshot {
            store: snapshot.store,
            version: draft.payload.version,
            head: Some(Hash::of(draft.payload.index.0.as_slice())),
            epoch: draft.payload.epoch,
            lookup_hash: draft.payload.lookup_hash,
            plain: draft.plain.clone(),
        }
    }

    #[test]
    fn a_draft_seals_every_part_under_the_current_epoch() {
        let fixture = Fixture::new(1);
        let ring = ring(&fixture);
        let empty = Snapshot::empty(fixture.store, 0);
        let mut changes = Changes {
            label: Some("prod".into()),
            ..Changes::default()
        };
        write(&mut changes, "db", "pw\nurl: https://db.example\nnote: x");
        changes.otp.insert(
            "gh".into(),
            Some(OtpParams {
                secret: "JBSWY3DPEHPK3PXP".into(),
                otp_type: "totp".into(),
                algorithm: "sha1".into(),
                digits: 6,
                period: 30,
                counter: 0,
                skew: 1,
            }),
        );
        let draft = draft(
            &fixture.log,
            &ring,
            &empty,
            LookupPlain::default(),
            &changes,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        let payload = &draft.payload;
        assert_eq!((payload.version, payload.prev, payload.epoch), (1, None, 0));
        assert_eq!(payload.log_head, fixture.log.head());
        assert_eq!(payload.changed.len(), 2);
        assert!(payload.deleted.is_empty());
        let keys = ring.current();
        let plain = open_index(fixture.store, 0, 1, &keys.names, &payload.index.0).unwrap();
        assert_eq!(plain, draft.plain);
        assert_eq!(plain.label, "prod");
        for object in &draft.objects {
            let entry = plain
                .secrets
                .values()
                .chain(plain.otp.values())
                .find(|entry| entry.id == object.id)
                .unwrap();
            assert_eq!(
                (entry.version, entry.hash),
                (object.version, Hash::of(&object.ciphertext.0))
            );
            open_object(
                fixture.store,
                0,
                object.id,
                object.version,
                keys.data().unwrap(),
                &object.ciphertext.0,
            )
            .unwrap();
        }
        let sealed = draft.lookup.unwrap();
        assert_eq!(payload.lookup_hash, Some(Hash::of(&sealed)));
        let lookup = open_lookup(fixture.store, 0, 1, keys.data().unwrap(), &sealed).unwrap();
        assert_eq!(lookup.secrets["db"], "\nurl: https://db.example");
        assert!(!lookup.secrets.contains_key("gh"));
    }

    #[test]
    fn updates_bump_versions_and_removals_list_identifiers() {
        let fixture = Fixture::new(1);
        let ring = ring(&fixture);
        let empty = Snapshot::empty(fixture.store, 0);
        let mut changes = Changes::default();
        write(&mut changes, "a", "1");
        write(&mut changes, "b", "2");
        let first = draft(
            &fixture.log,
            &ring,
            &empty,
            LookupPlain::default(),
            &changes,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        let snapshot = apply(&empty, &first);
        let lookup = open_lookup(
            fixture.store,
            0,
            1,
            ring.current().data().unwrap(),
            first.lookup.as_ref().unwrap(),
        )
        .unwrap();
        let mut changes = Changes::default();
        write(&mut changes, "a", "3");
        changes.secrets.insert("b".into(), None);
        changes.secrets.insert("never-existed".into(), None);
        let second = draft(
            &fixture.log,
            &ring,
            &snapshot,
            lookup,
            &changes,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        assert_eq!(second.payload.prev, snapshot.head);
        assert_eq!(second.plain.secrets["a"].id, snapshot.plain.secrets["a"].id);
        assert_eq!(second.plain.secrets["a"].version, 2);
        assert_eq!(second.payload.deleted, vec![snapshot.plain.secrets["b"].id]);
        assert_eq!(second.payload.changed, vec![snapshot.plain.secrets["a"].id]);
        let lookup = open_lookup(
            fixture.store,
            0,
            2,
            ring.current().data().unwrap(),
            second.lookup.as_ref().unwrap(),
        )
        .unwrap();
        assert_eq!(lookup.secrets.keys().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn renames_keep_identifiers_and_move_lookup_texts() {
        let fixture = Fixture::new(1);
        let ring = ring(&fixture);
        let empty = Snapshot::empty(fixture.store, 0);
        let mut changes = Changes::default();
        write(&mut changes, "old", "pw\nuser: alice");
        write(&mut changes, "taken", "x");
        let first = draft(
            &fixture.log,
            &ring,
            &empty,
            LookupPlain::default(),
            &changes,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        let snapshot = apply(&empty, &first);
        let data = ring.current().data().unwrap();
        let lookup =
            open_lookup(fixture.store, 0, 1, data, first.lookup.as_ref().unwrap()).unwrap();
        let rename = Changes {
            renames: vec![(Kind::Secret, "old".into(), "new".into())],
            ..Changes::default()
        };
        let renamed = draft(
            &fixture.log,
            &ring,
            &snapshot,
            lookup.clone(),
            &rename,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        assert_eq!(renamed.plain.secrets["new"], snapshot.plain.secrets["old"]);
        assert!(renamed.objects.is_empty());
        let moved =
            open_lookup(fixture.store, 0, 2, data, renamed.lookup.as_ref().unwrap()).unwrap();
        assert_eq!(moved.secrets["new"], "\nuser: alice");
        let onto = Changes {
            renames: vec![(Kind::Secret, "old".into(), "taken".into())],
            ..Changes::default()
        };
        assert!(
            draft(
                &fixture.log,
                &ring,
                &snapshot,
                lookup.clone(),
                &onto,
                &BTreeMap::new(),
                TIME
            )
            .is_err()
        );
        let missing = Changes {
            renames: vec![(Kind::Otp, "old".into(), "x".into())],
            ..Changes::default()
        };
        assert!(
            draft(
                &fixture.log,
                &ring,
                &snapshot,
                lookup,
                &missing,
                &BTreeMap::new(),
                TIME
            )
            .is_err()
        );
    }

    #[test]
    fn rewrites_reseal_under_a_new_epoch_and_need_values() {
        let mut fixture = Fixture::new(1);
        let first_ring = ring(&fixture);
        let empty = Snapshot::empty(fixture.store, 0);
        let mut changes = Changes::default();
        write(&mut changes, "a", "1");
        let first = draft(
            &fixture.log,
            &first_ring,
            &empty,
            LookupPlain::default(),
            &changes,
            &BTreeMap::new(),
            TIME,
        )
        .unwrap();
        let snapshot = apply(&empty, &first);
        fixture.add_epoch();
        let second_ring = ring(&fixture);
        let rewrite = Changes {
            rewrite: [(Kind::Secret, "a".to_owned())].into(),
            ..Changes::default()
        };
        assert!(
            draft(
                &fixture.log,
                &second_ring,
                &snapshot,
                LookupPlain::default(),
                &rewrite,
                &BTreeMap::new(),
                TIME
            )
            .is_err()
        );
        let values = BTreeMap::from([(
            (Kind::Secret, "a".to_owned()),
            ObjectPlain::Secret { secret: "1".into() },
        )]);
        let rewritten = draft(
            &fixture.log,
            &second_ring,
            &snapshot,
            LookupPlain::default(),
            &rewrite,
            &values,
            TIME,
        )
        .unwrap();
        assert_eq!(rewritten.payload.epoch, 1);
        assert_eq!(rewritten.objects.len(), 1);
        assert_eq!(rewritten.objects[0].epoch, 1);
        assert_eq!(
            rewritten.plain.secrets["a"].id,
            snapshot.plain.secrets["a"].id
        );
        assert_eq!(rewritten.plain.secrets["a"].version, 2);
        // An entry that a rewrite names but the index lacks is skipped.
        let gone = Changes {
            rewrite: [(Kind::Otp, "nope".to_owned())].into(),
            ..Changes::default()
        };
        assert!(
            draft(
                &fixture.log,
                &second_ring,
                &snapshot,
                LookupPlain::default(),
                &gone,
                &BTreeMap::new(),
                TIME
            )
            .unwrap()
            .objects
            .is_empty()
        );
    }

    #[test]
    fn names_only_rings_cannot_draft() {
        let fixture = Fixture::new(1);
        let ring = KeyRing::open(
            &fixture.log,
            fixture.store,
            fixture.device.id,
            &fixture.keys(false),
            &fixture.identities(),
        )
        .unwrap();
        let empty = Snapshot::empty(fixture.store, 0);
        assert!(
            draft(
                &fixture.log,
                &ring,
                &empty,
                LookupPlain::default(),
                &Changes::default(),
                &BTreeMap::new(),
                TIME
            )
            .is_err()
        );
    }

    #[test]
    fn wrappings_follow_the_device_kind_and_entitlement() {
        let fixture = Fixture::new(1);
        let state = fixture.log.state();
        let keys = &fixture.epochs[0];
        let full = wrapping_for(state, fixture.store, 0, keys, fixture.device.id, true).unwrap();
        assert!(full.full);
        let opened = pasejo_protocol::wrapping::unwrap_age(
            fixture.store,
            0,
            &full.ciphertext.0,
            &fixture.identities(),
        )
        .unwrap();
        assert_eq!(&opened, keys);
        let names = wrapping_for(state, fixture.store, 0, keys, fixture.device.id, false).unwrap();
        assert!(!names.full);
        let opened = pasejo_protocol::wrapping::unwrap_age(
            fixture.store,
            0,
            &names.ciphertext.0,
            &fixture.identities(),
        )
        .unwrap();
        assert!(opened.data().is_err());
        assert!(wrapping_for(state, fixture.store, 0, keys, Id::random(), true).is_err());
    }

    #[test]
    fn lookup_text_keeps_lookup_and_policy_fields_only() {
        let settings = StoreSettings::default();
        let secret =
            "hunter2\nurl: https://example.com\nnote: private\nUser: alice\npasejo-fill: never\n";
        assert_eq!(
            lookup_text(secret, &settings),
            "\nurl: https://example.com\nUser: alice\npasejo-fill: never"
        );
        assert_eq!(lookup_text("only a password", &settings), "");
    }

    #[test]
    fn empty_changes_are_empty() {
        assert!(Changes::default().is_empty());
        let mut changes = Changes::default();
        changes.secrets.insert("a".into(), None);
        assert!(!changes.is_empty());
    }
}
