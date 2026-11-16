// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The membership log of an organization.
//!
//! An append-only, hash-chained list of signed entries decides who belongs
//! to an organization, which devices may receive keys, and who may read or
//! write each store. Clients and the server replay it with this module, so
//! both reach the same verdict on every entry.

use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::device::{DeviceKind, DeviceRecord, recovery_id, validate_recipient};
use crate::encoding::{Hash, Id};
use crate::envelope::{Context, Envelope, verify};
use crate::store::IndexPayload;
use crate::{Error, Millis};

/// How new devices of organization members get approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Approval {
    #[default]
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Settings {
    pub device_approval: Approval,
    pub two_admin: bool,
    pub auto_join: bool,
    /// Whether devices may keep an offline copy of secret values. A read
    /// from that copy reaches the audit log only when the device reports
    /// it after reconnecting.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub offline_reads: bool,
}

/// Organization roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Approver,
    Auditor,
    Member,
}

/// A role change; `None` removes the user from the organization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoleChange {
    Admin,
    Approver,
    Auditor,
    Member,
    None,
}

impl RoleChange {
    #[must_use]
    pub const fn role(self) -> Option<Role> {
        match self {
            Self::Admin => Some(Role::Admin),
            Self::Approver => Some(Role::Approver),
            Self::Auditor => Some(Role::Auditor),
            Self::Member => Some(Role::Member),
            Self::None => None,
        }
    }
}

/// Store roles, ordered by what they allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoreRole {
    Reader,
    Writer,
    Admin,
}

/// A grant change; `None` removes the grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StoreRoleChange {
    Reader,
    Writer,
    Admin,
    None,
}

impl StoreRoleChange {
    #[must_use]
    pub const fn role(self) -> Option<StoreRole> {
        match self {
            Self::Reader => Some(StoreRole::Reader),
            Self::Writer => Some(StoreRole::Writer),
            Self::Admin => Some(StoreRole::Admin),
            Self::None => None,
        }
    }
}

/// Who a store grant is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Principal {
    User(Id),
    Team(Id),
}

/// The kind-specific part of an entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body")]
pub enum Body {
    #[serde(rename = "org.genesis")]
    Genesis {
        name: String,
        founder: Id,
        device: DeviceRecord,
        settings: Settings,
    },
    #[serde(rename = "org.settings")]
    OrgSettings { settings: Settings },
    #[serde(rename = "member.role")]
    MemberRole { user: Id, role: RoleChange },
    #[serde(rename = "device.add")]
    DeviceAdd { device: DeviceRecord },
    #[serde(rename = "device.revoke")]
    DeviceRevoke { device: Id },
    #[serde(rename = "recovery.set")]
    RecoverySet { recipient: Option<String> },
    #[serde(rename = "team.create")]
    TeamCreate { team: Id, name: String },
    #[serde(rename = "team.member")]
    TeamMember { team: Id, user: Id, member: bool },
    #[serde(rename = "team.delete")]
    TeamDelete { team: Id },
    #[serde(rename = "store.create")]
    StoreCreate { store: Id },
    #[serde(rename = "store.grant")]
    StoreGrant {
        store: Id,
        principal: Principal,
        role: StoreRoleChange,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expires: Option<Millis>,
    },
    #[serde(rename = "store.epoch")]
    StoreEpoch {
        store: Id,
        epoch: u64,
        names_commit: Hash,
        data_commit: Hash,
        full: Vec<Id>,
        names_only: Vec<Id>,
    },
    #[serde(rename = "store.delete")]
    StoreDelete { store: Id },
}

impl Body {
    /// The kind as it appears on the wire.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Genesis { .. } => "org.genesis",
            Self::OrgSettings { .. } => "org.settings",
            Self::MemberRole { .. } => "member.role",
            Self::DeviceAdd { .. } => "device.add",
            Self::DeviceRevoke { .. } => "device.revoke",
            Self::RecoverySet { .. } => "recovery.set",
            Self::TeamCreate { .. } => "team.create",
            Self::TeamMember { .. } => "team.member",
            Self::TeamDelete { .. } => "team.delete",
            Self::StoreCreate { .. } => "store.create",
            Self::StoreGrant { .. } => "store.grant",
            Self::StoreEpoch { .. } => "store.epoch",
            Self::StoreDelete { .. } => "store.delete",
        }
    }

    /// The store an entry concerns, if any.
    #[must_use]
    pub const fn store(&self) -> Option<Id> {
        match self {
            Self::StoreCreate { store }
            | Self::StoreGrant { store, .. }
            | Self::StoreEpoch { store, .. }
            | Self::StoreDelete { store } => Some(*store),
            _ => None,
        }
    }
}

/// The signed payload of a log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub org: Id,
    pub seq: u64,
    pub prev: Option<Hash>,
    pub time: Millis,
    pub author: Id,
    #[serde(flatten)]
    pub body: Body,
}

/// Why the log refused an entry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rejection {
    #[error("the entry is not valid JSON of a known kind")]
    Malformed,
    #[error("the entry belongs to another organization")]
    WrongOrganization,
    #[error("the entry's sequence number is not the next one")]
    WrongSequence,
    #[error("the entry does not chain to the current head")]
    WrongPredecessor,
    #[error("the entry is older than its predecessor")]
    TimeWentBackwards,
    #[error("the author is not the first signer")]
    AuthorNotFirstSigner,
    #[error("a signer is not an active device")]
    InactiveSigner,
    #[error("a device signed twice")]
    DuplicateSigner,
    #[error("a signature does not verify")]
    BadSignature,
    #[error("the first entry must be the genesis, and only the first")]
    Genesis,
    #[error("the author may not make this change: {0}")]
    Unauthorized(&'static str),
    #[error("this change needs a second admin's signature")]
    SecondAdminRequired,
    #[error("unknown user")]
    UnknownUser,
    #[error("unknown device")]
    UnknownDevice,
    #[error("unknown team")]
    UnknownTeam,
    #[error("unknown or deleted store")]
    UnknownStore,
    #[error("the identifier is already in use")]
    DuplicateIdentifier,
    #[error("the change would leave the organization without an admin")]
    LastAdmin,
    #[error("the change would leave the store without an admin")]
    LastStoreAdmin,
    #[error("the user is already a member")]
    AlreadyMember,
    #[error("the device record is invalid")]
    InvalidDevice,
    #[error("the team name is invalid")]
    InvalidName,
    #[error("the grant does not exist")]
    UnknownGrant,
    #[error("the expiry lies in the past")]
    ExpiryInPast,
    #[error("the epoch is not the next one")]
    WrongEpoch,
    #[error("the epoch wraps keys for a device that is not entitled to them")]
    UnentitledRecipient,
    #[error("the recovery recipient is invalid")]
    InvalidRecovery,
}

/// A device and whether it is still active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceState {
    pub record: DeviceRecord,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub name: String,
    pub members: BTreeSet<Id>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    pub role: StoreRole,
    pub expires: Option<Millis>,
}

impl Grant {
    #[must_use]
    pub fn effective(&self, now: Millis) -> bool {
        self.expires.is_none_or(|expires| expires > now)
    }
}

/// What a `store.epoch` entry recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochRecord {
    pub names_commit: Hash,
    pub data_commit: Hash,
    pub full: BTreeSet<Id>,
    pub names_only: BTreeSet<Id>,
    pub seq: u64,
    pub time: Millis,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoreState {
    pub grants: BTreeMap<Principal, Grant>,
    pub epochs: Vec<EpochRecord>,
    pub deleted: bool,
    pub created_seq: u64,
}

impl StoreState {
    #[must_use]
    pub const fn current_epoch(&self) -> Option<u64> {
        (self.epochs.len() as u64).checked_sub(1)
    }
}

/// The devices that should hold an epoch's keys.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Recipients {
    pub full: BTreeSet<Id>,
    pub names_only: BTreeSet<Id>,
}

/// The state the log describes after some prefix of entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgState {
    pub org: Id,
    pub name: String,
    pub settings: Settings,
    pub members: BTreeMap<Id, Role>,
    pub devices: BTreeMap<Id, DeviceState>,
    pub recovery: Option<String>,
    pub teams: BTreeMap<Id, Team>,
    pub stores: BTreeMap<Id, StoreState>,
    /// Identifiers ever used for devices, teams or stores; never reused.
    used: BTreeSet<Id>,
    pub next_seq: u64,
    pub head: Hash,
    pub last_time: Millis,
}

impl OrgState {
    /// The active device `device`, if any.
    #[must_use]
    pub fn active_device(&self, device: Id) -> Option<&DeviceRecord> {
        self.devices
            .get(&device)
            .filter(|state| state.active)
            .map(|state| &state.record)
    }

    /// The organization role of the user owning active device `device`.
    #[must_use]
    pub fn device_role(&self, device: Id) -> Option<Role> {
        self.active_device(device)
            .and_then(|record| self.members.get(&record.user).copied())
    }

    /// Active devices of `user`.
    pub fn devices_of(&self, user: Id) -> impl Iterator<Item = &DeviceRecord> {
        self.devices
            .values()
            .filter(move |state| state.active && state.record.user == user)
            .map(|state| &state.record)
    }

    /// Whether `user` belongs to team `team`.
    #[must_use]
    pub fn in_team(&self, team: Id, user: Id) -> bool {
        self.teams
            .get(&team)
            .is_some_and(|team| team.members.contains(&user))
    }

    /// The strongest effective grant `user` holds on `store` at `now`,
    /// directly or through a team. Non-members hold nothing.
    #[must_use]
    pub fn store_role(&self, store: Id, user: Id, now: Millis) -> Option<StoreRole> {
        if !self.members.contains_key(&user) {
            return None;
        }
        let state = self.stores.get(&store).filter(|state| !state.deleted)?;
        state
            .grants
            .iter()
            .filter(|(_, grant)| grant.effective(now))
            .filter(|(principal, _)| match principal {
                Principal::User(id) => *id == user,
                Principal::Team(team) => self.in_team(*team, user),
            })
            .map(|(_, grant)| grant.role)
            .max()
    }

    /// Whether any member holds an effective admin grant on `store`.
    #[must_use]
    pub fn store_has_admin(&self, store: Id, now: Millis) -> bool {
        self.members
            .keys()
            .any(|user| self.store_role(store, *user, now) == Some(StoreRole::Admin))
    }

    /// Users with an effective grant on `store`.
    #[must_use]
    pub fn store_users(&self, store: Id, now: Millis) -> BTreeSet<Id> {
        self.members
            .keys()
            .copied()
            .filter(|user| self.store_role(store, *user, now).is_some())
            .collect()
    }

    /// The devices entitled to `store`'s keys at `now`: age devices of users
    /// with a grant and the recovery recipient receive full keys; browser
    /// devices of those users and every device of organization admins and
    /// auditors receive names only.
    #[must_use]
    pub fn entitled(&self, store: Id, now: Millis) -> Recipients {
        let mut recipients = Recipients::default();
        if self.stores.get(&store).is_none_or(|state| state.deleted) {
            return recipients;
        }
        for user in self.store_users(store, now) {
            for device in self.devices_of(user) {
                match device.kind {
                    DeviceKind::Age => recipients.full.insert(device.id),
                    DeviceKind::Hpke => recipients.names_only.insert(device.id),
                };
            }
        }
        if let Some(recipient) = &self.recovery {
            recipients.full.insert(recovery_id(recipient));
        }
        for (user, role) in &self.members {
            if matches!(role, Role::Admin | Role::Auditor) {
                for device in self.devices_of(*user) {
                    if !recipients.full.contains(&device.id) {
                        recipients.names_only.insert(device.id);
                    }
                }
            }
        }
        recipients
    }

    /// Whether `id` names the current recovery recipient.
    #[must_use]
    pub fn is_recovery(&self, id: Id) -> bool {
        self.recovery
            .as_deref()
            .is_some_and(|recipient| recovery_id(recipient) == id)
    }

    fn admin_count(&self) -> usize {
        self.members
            .values()
            .filter(|role| **role == Role::Admin)
            .count()
    }
}

/// A replayed log: its entries, their hashes and the resulting state.
#[derive(Debug, Clone)]
pub struct OrgLog {
    entries: Vec<Envelope>,
    hashes: Vec<Hash>,
    state: OrgState,
}

impl OrgLog {
    /// Starts a log from its genesis entry.
    ///
    /// # Errors
    ///
    /// Fails when the genesis entry is malformed or badly signed.
    pub fn genesis(envelope: Envelope) -> Result<Self, Rejection> {
        let entry: Entry = envelope.open().map_err(|_| Rejection::Malformed)?;
        let Body::Genesis {
            name,
            founder,
            device,
            settings,
        } = &entry.body
        else {
            return Err(Rejection::Genesis);
        };
        if entry.seq != 0 || entry.prev.is_some() {
            return Err(Rejection::Genesis);
        }
        if device.user != *founder || entry.author != device.id {
            return Err(Rejection::Genesis);
        }
        device.validate().map_err(|_| Rejection::InvalidDevice)?;
        if name.trim().is_empty() || name.len() > 200 {
            return Err(Rejection::InvalidName);
        }
        if envelope.signatures.len() != 1 || envelope.signatures[0].device != device.id {
            return Err(Rejection::AuthorNotFirstSigner);
        }
        verify(
            Context::LogEntry,
            &envelope.payload.0,
            &device.signing_key.0,
            &envelope.signatures[0].signature.0,
        )
        .map_err(|_| Rejection::BadSignature)?;
        let hash = envelope.hash();
        let mut members = BTreeMap::new();
        members.insert(*founder, Role::Admin);
        let mut devices = BTreeMap::new();
        devices.insert(
            device.id,
            DeviceState {
                record: device.clone(),
                active: true,
            },
        );
        let state = OrgState {
            org: entry.org,
            name: name.clone(),
            settings: *settings,
            members,
            devices,
            recovery: None,
            teams: BTreeMap::new(),
            stores: BTreeMap::new(),
            used: BTreeSet::from([device.id]),
            next_seq: 1,
            head: hash,
            last_time: entry.time,
        };
        Ok(Self {
            entries: vec![envelope],
            hashes: vec![hash],
            state,
        })
    }

    /// Replays a whole log.
    ///
    /// # Errors
    ///
    /// Fails with the sequence number and reason of the first entry that
    /// does not replay.
    pub fn replay(entries: impl IntoIterator<Item = Envelope>) -> Result<Self, (u64, Rejection)> {
        let mut entries = entries.into_iter();
        let first = entries.next().ok_or((0, Rejection::Genesis))?;
        let mut log = Self::genesis(first).map_err(|rejection| (0, rejection))?;
        for envelope in entries {
            let seq = log.state.next_seq;
            log.append(envelope).map_err(|rejection| (seq, rejection))?;
        }
        Ok(log)
    }

    #[must_use]
    pub const fn state(&self) -> &OrgState {
        &self.state
    }

    #[must_use]
    pub fn entries(&self) -> &[Envelope] {
        &self.entries
    }

    #[must_use]
    pub const fn head(&self) -> Hash {
        self.state.head
    }

    #[must_use]
    pub fn genesis_hash(&self) -> Hash {
        self.hashes[0]
    }

    /// The sequence number of the entry with hash `hash`.
    #[must_use]
    pub fn seq_of(&self, hash: &Hash) -> Option<u64> {
        self.hashes
            .iter()
            .position(|candidate| candidate == hash)
            .map(|index| index as u64)
    }

    /// The periods in which the organization allowed offline reads, from
    /// the time of the entry that allowed them to the time of the entry that
    /// stopped them; the last period is open while they are still allowed.
    #[must_use]
    pub fn offline_read_periods(&self) -> Vec<(Millis, Option<Millis>)> {
        let mut periods: Vec<(Millis, Option<Millis>)> = Vec::new();
        for envelope in &self.entries {
            let Ok(entry) = envelope.open::<Entry>() else {
                continue;
            };
            let (Body::Genesis { settings, .. } | Body::OrgSettings { settings }) = entry.body
            else {
                continue;
            };
            match periods.last_mut() {
                Some((_, end @ None)) if !settings.offline_reads => *end = Some(entry.time),
                Some((_, None)) => {}
                _ if settings.offline_reads => periods.push((entry.time, None)),
                _ => {}
            }
        }
        periods
    }

    /// Whether the organization allowed offline reads at `time`.
    #[must_use]
    pub fn offline_reads_allowed_at(&self, time: Millis) -> bool {
        self.offline_read_periods()
            .iter()
            .any(|(from, until)| *from <= time && until.is_none_or(|until| time < until))
    }

    /// The state after entry `seq`.
    ///
    /// # Errors
    ///
    /// Fails when `seq` is beyond the log.
    pub fn state_at(&self, seq: u64) -> Result<OrgState, Rejection> {
        let end = usize::try_from(seq).map_err(|_| Rejection::WrongSequence)? + 1;
        if end > self.entries.len() {
            return Err(Rejection::WrongSequence);
        }
        if end == self.entries.len() {
            return Ok(self.state.clone());
        }
        let log = Self::replay(self.entries[..end].iter().cloned())
            .map_err(|(_, rejection)| rejection)?;
        Ok(log.state)
    }

    /// Appends an entry after checking its chain, signatures and the
    /// author's rights. A refused entry leaves the log unchanged.
    ///
    /// # Errors
    ///
    /// Returns the reason the entry was refused.
    pub fn append(&mut self, envelope: Envelope) -> Result<Entry, Rejection> {
        let entry: Entry = envelope.open().map_err(|_| Rejection::Malformed)?;
        let state = &self.state;
        if entry.org != state.org {
            return Err(Rejection::WrongOrganization);
        }
        if matches!(entry.body, Body::Genesis { .. }) {
            return Err(Rejection::Genesis);
        }
        if entry.seq != state.next_seq {
            return Err(Rejection::WrongSequence);
        }
        if entry.prev != Some(state.head) {
            return Err(Rejection::WrongPredecessor);
        }
        if entry.time < state.last_time {
            return Err(Rejection::TimeWentBackwards);
        }
        let signers = check_signatures(state, &envelope, entry.author)?;
        let mut next = state.clone();
        apply(&mut next, &entry, &signers)?;
        let hash = envelope.hash();
        next.next_seq += 1;
        next.head = hash;
        next.last_time = entry.time;
        self.state = next;
        self.entries.push(envelope);
        self.hashes.push(hash);
        Ok(entry)
    }

    /// Checks an index version against the log: its signature by an author
    /// device that was active and held a writer or admin grant at the log
    /// head the version names, and its epoch.
    ///
    /// # Errors
    ///
    /// Fails when the index cannot be attributed to an authorized writer.
    pub fn check_index(&self, envelope: &Envelope) -> Result<IndexPayload, Error> {
        let payload: IndexPayload = envelope.open()?;
        let author = envelope.signatures.first().ok_or(Error::Signature)?.device;
        let seq = self
            .seq_of(&payload.log_head)
            .ok_or(Error::Invalid("index names an unknown log head"))?;
        let state = self.state_at(seq)?;
        let device = state
            .active_device(author)
            .ok_or(Error::Invalid("index author is not an active device"))?;
        envelope.verify_by(Context::Index, author, &device.signing_key.0)?;
        match state.store_role(payload.store, device.user, payload.time) {
            Some(StoreRole::Writer | StoreRole::Admin) => {}
            _ => return Err(Error::Invalid("index author may not write the store")),
        }
        let epochs = state
            .stores
            .get(&payload.store)
            .map_or(0, |store| store.epochs.len() as u64);
        if payload.epoch >= epochs {
            return Err(Error::Invalid("index names an unknown epoch"));
        }
        Ok(payload)
    }

    /// The epoch record that commits to `store`'s keys of `epoch`.
    #[must_use]
    pub fn epoch(&self, store: Id, epoch: u64) -> Option<&EpochRecord> {
        self.state
            .stores
            .get(&store)
            .and_then(|state| state.epochs.get(usize::try_from(epoch).ok()?))
    }
}

/// Verifies every signature and returns the signers' users, author first.
fn check_signatures(
    state: &OrgState,
    envelope: &Envelope,
    author: Id,
) -> Result<Vec<(Id, Id)>, Rejection> {
    if envelope
        .signatures
        .first()
        .map(|signature| signature.device)
        != Some(author)
    {
        return Err(Rejection::AuthorNotFirstSigner);
    }
    let mut seen = BTreeSet::new();
    let mut signers = Vec::new();
    for signature in &envelope.signatures {
        if !seen.insert(signature.device) {
            return Err(Rejection::DuplicateSigner);
        }
        let device = state
            .active_device(signature.device)
            .ok_or(Rejection::InactiveSigner)?;
        verify(
            Context::LogEntry,
            &envelope.payload.0,
            &device.signing_key.0,
            &signature.signature.0,
        )
        .map_err(|_| Rejection::BadSignature)?;
        signers.push((signature.device, device.user));
    }
    Ok(signers)
}

const fn author_user(signers: &[(Id, Id)]) -> Id {
    signers[0].1
}

fn require_admin(state: &OrgState, user: Id, what: &'static str) -> Result<(), Rejection> {
    if state.members.get(&user) == Some(&Role::Admin) {
        Ok(())
    } else {
        Err(Rejection::Unauthorized(what))
    }
}

/// Requires signatures of two distinct users holding `qualifies`.
fn require_two(signers: &[(Id, Id)], qualifies: impl Fn(Id) -> bool) -> Result<(), Rejection> {
    let users: BTreeSet<Id> = signers
        .iter()
        .map(|(_, user)| *user)
        .filter(|user| qualifies(*user))
        .collect();
    if users.len() >= 2 {
        Ok(())
    } else {
        Err(Rejection::SecondAdminRequired)
    }
}

fn require_member(state: &OrgState, user: Id) -> Result<(), Rejection> {
    if state.members.contains_key(&user) {
        Ok(())
    } else {
        Err(Rejection::UnknownUser)
    }
}

fn live_store(state: &OrgState, store: Id) -> Result<&StoreState, Rejection> {
    state
        .stores
        .get(&store)
        .filter(|store| !store.deleted)
        .ok_or(Rejection::UnknownStore)
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 200 && !name.chars().any(char::is_control)
}

#[allow(clippy::too_many_lines)]
fn apply(state: &mut OrgState, entry: &Entry, signers: &[(Id, Id)]) -> Result<(), Rejection> {
    let author = author_user(signers);
    let now = entry.time;
    match &entry.body {
        Body::Genesis { .. } => return Err(Rejection::Genesis),
        Body::OrgSettings { settings } => {
            require_admin(state, author, "change settings")?;
            if state.settings.two_admin || settings.two_admin {
                require_two(signers, |user| {
                    state.members.get(&user) == Some(&Role::Admin)
                })?;
            }
            state.settings = *settings;
        }
        Body::MemberRole { user, role } => {
            require_admin(state, author, "change roles")?;
            let current = state.members.get(user).copied();
            if current.is_none() && role.role().is_none() {
                return Err(Rejection::UnknownUser);
            }
            if state.settings.two_admin
                && (role.role() == Some(Role::Admin) || current == Some(Role::Admin))
            {
                require_two(signers, |signer| {
                    state.members.get(&signer) == Some(&Role::Admin)
                })?;
            }
            if current == Some(Role::Admin)
                && role.role() != Some(Role::Admin)
                && state.admin_count() == 1
            {
                return Err(Rejection::LastAdmin);
            }
            if let Some(new) = role.role() {
                state.members.insert(*user, new);
            } else {
                state.members.remove(user);
                for device in state.devices.values_mut() {
                    if device.record.user == *user {
                        device.active = false;
                    }
                }
                for team in state.teams.values_mut() {
                    team.members.remove(user);
                }
                for store in state.stores.values_mut() {
                    store.grants.remove(&Principal::User(*user));
                }
            }
        }
        Body::DeviceAdd { device } => {
            device.validate().map_err(|_| Rejection::InvalidDevice)?;
            VerifyingKey::from_bytes(&device.signing_key.0)
                .map_err(|_| Rejection::InvalidDevice)?;
            require_member(state, device.user)?;
            if state.used.contains(&device.id) {
                return Err(Rejection::DuplicateIdentifier);
            }
            let role = state.members.get(&author).copied();
            let approver = matches!(role, Some(Role::Admin | Role::Approver));
            if !approver && author != device.user {
                return Err(Rejection::Unauthorized("add devices of other users"));
            }
            state.used.insert(device.id);
            state.devices.insert(
                device.id,
                DeviceState {
                    record: device.clone(),
                    active: true,
                },
            );
        }
        Body::DeviceRevoke { device } => {
            let target = state
                .active_device(*device)
                .ok_or(Rejection::UnknownDevice)?;
            if target.user != author {
                require_admin(state, author, "revoke devices of other users")?;
            }
            if let Some(device) = state.devices.get_mut(device) {
                device.active = false;
            }
        }
        Body::RecoverySet { recipient } => {
            require_admin(state, author, "set the recovery recipient")?;
            if state.settings.two_admin {
                require_two(signers, |user| {
                    state.members.get(&user) == Some(&Role::Admin)
                })?;
            }
            if let Some(recipient) = recipient {
                validate_recipient(DeviceKind::Age, recipient)
                    .map_err(|_| Rejection::InvalidRecovery)?;
            }
            state.recovery.clone_from(recipient);
        }
        Body::TeamCreate { team, name } => {
            require_admin(state, author, "create teams")?;
            if !valid_name(name) {
                return Err(Rejection::InvalidName);
            }
            if state.used.contains(team) {
                return Err(Rejection::DuplicateIdentifier);
            }
            state.used.insert(*team);
            state.teams.insert(
                *team,
                Team {
                    name: name.clone(),
                    members: BTreeSet::new(),
                },
            );
        }
        Body::TeamMember { team, user, member } => {
            require_admin(state, author, "change teams")?;
            if !state.teams.contains_key(team) {
                return Err(Rejection::UnknownTeam);
            }
            if *member {
                require_member(state, *user)?;
            }
            let members = &mut state
                .teams
                .get_mut(team)
                .ok_or(Rejection::UnknownTeam)?
                .members;
            if *member {
                members.insert(*user);
            } else if !members.remove(user) {
                return Err(Rejection::UnknownUser);
            }
        }
        Body::TeamDelete { team } => {
            require_admin(state, author, "delete teams")?;
            if state.teams.remove(team).is_none() {
                return Err(Rejection::UnknownTeam);
            }
            for store in state.stores.values_mut() {
                store.grants.remove(&Principal::Team(*team));
            }
        }
        Body::StoreCreate { store } => {
            require_member(state, author)?;
            if state.used.contains(store) {
                return Err(Rejection::DuplicateIdentifier);
            }
            state.used.insert(*store);
            let mut grants = BTreeMap::new();
            grants.insert(
                Principal::User(author),
                Grant {
                    role: StoreRole::Admin,
                    expires: None,
                },
            );
            state.stores.insert(
                *store,
                StoreState {
                    grants,
                    epochs: Vec::new(),
                    deleted: false,
                    created_seq: entry.seq,
                },
            );
        }
        Body::StoreGrant {
            store,
            principal,
            role,
            expires,
        } => {
            live_store(state, *store)?;
            let store_admin = state.store_role(*store, author, now) == Some(StoreRole::Admin);
            let orphan_rescue = state.members.get(&author) == Some(&Role::Admin)
                && !state.store_has_admin(*store, now);
            if !store_admin && !orphan_rescue {
                return Err(Rejection::Unauthorized("change grants of this store"));
            }
            match principal {
                Principal::User(user) => {
                    if role.role().is_some() {
                        require_member(state, *user)?;
                    }
                }
                Principal::Team(team) => {
                    if !state.teams.contains_key(team) {
                        return Err(Rejection::UnknownTeam);
                    }
                }
            }
            if let Some(expires) = expires
                && *expires <= now
            {
                return Err(Rejection::ExpiryInPast);
            }
            let grants = &state
                .stores
                .get(store)
                .ok_or(Rejection::UnknownStore)?
                .grants;
            let is_last_admin_grant = grants.get(principal).map(|grant| grant.role)
                == Some(StoreRole::Admin)
                && grants
                    .iter()
                    .filter(|(_, grant)| grant.role == StoreRole::Admin)
                    .count()
                    == 1;
            if is_last_admin_grant && role.role() != Some(StoreRole::Admin) {
                return Err(Rejection::LastStoreAdmin);
            }
            let grants = &mut state
                .stores
                .get_mut(store)
                .ok_or(Rejection::UnknownStore)?
                .grants;
            match role.role() {
                Some(role) => {
                    grants.insert(
                        *principal,
                        Grant {
                            role,
                            expires: *expires,
                        },
                    );
                }
                None => {
                    if grants.remove(principal).is_none() {
                        return Err(Rejection::UnknownGrant);
                    }
                }
            }
        }
        Body::StoreEpoch {
            store,
            epoch,
            names_commit,
            data_commit,
            full,
            names_only,
        } => {
            let current = live_store(state, *store)?.epochs.len() as u64;
            if state.store_role(*store, author, now) != Some(StoreRole::Admin) {
                return Err(Rejection::Unauthorized("rotate this store's keys"));
            }
            if *epoch != current {
                return Err(Rejection::WrongEpoch);
            }
            let entitled = state.entitled(*store, now);
            let full: BTreeSet<Id> = full.iter().copied().collect();
            let names_only: BTreeSet<Id> = names_only.iter().copied().collect();
            let allowed_names: BTreeSet<Id> =
                entitled.full.union(&entitled.names_only).copied().collect();
            if !full.is_subset(&entitled.full)
                || !names_only.is_subset(&allowed_names)
                || !full.is_disjoint(&names_only)
            {
                return Err(Rejection::UnentitledRecipient);
            }
            state
                .stores
                .get_mut(store)
                .ok_or(Rejection::UnknownStore)?
                .epochs
                .push(EpochRecord {
                    names_commit: *names_commit,
                    data_commit: *data_commit,
                    full,
                    names_only,
                    seq: entry.seq,
                    time: entry.time,
                });
        }
        Body::StoreDelete { store } => {
            live_store(state, *store)?;
            if state.store_role(*store, author, now) != Some(StoreRole::Admin) {
                return Err(Rejection::Unauthorized("delete this store"));
            }
            if state.settings.two_admin {
                require_two(signers, |user| {
                    state.members.get(&user) == Some(&Role::Admin)
                        || state.store_role(*store, user, now) == Some(StoreRole::Admin)
                })?;
            }
            if let Some(store) = state.stores.get_mut(store) {
                store.deleted = true;
            }
        }
    }
    Ok(())
}

/// Builds the next entry of `log` for `body`, authored by `author` at
/// `time`. The time never goes below the head's, so entries made on a
/// client whose clock lags still chain.
#[must_use]
pub fn next_entry(log: &OrgLog, author: Id, time: Millis, body: Body) -> Entry {
    let state = log.state();
    Entry {
        org: state.org,
        seq: state.next_seq,
        prev: Some(state.head),
        time: time.max(state.last_time),
        author,
        body,
    }
}

/// Signs an entry as its author.
///
/// # Errors
///
/// Fails only if the entry cannot be serialized.
pub fn seal_entry(entry: &Entry, key: &SigningKey) -> Result<Envelope, Error> {
    Envelope::seal(Context::LogEntry, entry, entry.author, key)
}

/// Builds and signs a genesis entry.
///
/// # Errors
///
/// Fails only if the entry cannot be serialized.
pub fn genesis(
    org: Id,
    name: &str,
    device: DeviceRecord,
    settings: Settings,
    time: Millis,
    key: &SigningKey,
) -> Result<Envelope, Error> {
    let entry = Entry {
        org,
        seq: 0,
        prev: None,
        time,
        author: device.id,
        body: Body::Genesis {
            name: name.to_owned(),
            founder: device.user,
            device,
            settings,
        },
    };
    seal_entry(&entry, key)
}
