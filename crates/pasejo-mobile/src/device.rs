// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! One installation of a pasejo app as a device of one organization.
//!
//! The device keeps the verified membership log, the session and the index
//! pins in [`Device::state`], which the app stores encrypted. Store keys are
//! opened by [`Device::unlock`] and live only in memory until
//! [`Device::lock`]. Secret values are fetched per read; while the
//! organization allows offline reads, each fetched value is also sealed to
//! the device into an offline copy, which [`Device::unlock`] opens when the
//! server cannot be reached and whose reads are reported on reconnection.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pasejo_client::keys::KeyRing;
use pasejo_client::session::AppLogin;
use pasejo_client::store::{Kind, Snapshot};
use pasejo_client::transport::system_clock;
use pasejo_client::{Client, DeviceIdentity, Session};
use pasejo_protocol::api::{EnrollmentStatus, ErrorCode, OfflineRead};
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::{Hash, b64};
use pasejo_protocol::envelope::{Envelope, Signer};
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::store::{ObjectPlain, Pin, StoreSettings};
use pasejo_protocol::{Id, Millis};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::MobileError;
use crate::fill::{self, DomainVerifier, FillCredential, FillRequest, QueryResult};
use crate::keys::{AgreementIdentity, DeviceSigner, KeyAgreement, SignerAdapter, recipient};
use crate::offline::{self, Record};
use crate::otp::{TotpCode, totp_at};

const STATE_VERSION: u32 = 1;
const TIMEOUT: Duration = Duration::from_secs(30);

/// What the server says about itself.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerInfo {
    pub version: String,
    pub checkpoint_key: String,
    pub oidc: bool,
}

/// Whether the organization has approved this device yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Enrollment {
    pub pending: bool,
    pub active: bool,
}

/// The organization as its verified membership log describes it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OrgInfo {
    pub name: String,
    /// Whether this device is an active device of the organization.
    pub active: bool,
    /// Whether the organization allows offline copies of values.
    pub offline_reads: bool,
}

/// A store this device opened, with its entry names.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StoreView {
    pub id: String,
    pub label: String,
    pub secrets: Vec<String>,
    pub otp: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredSession {
    token: String,
    expires: Millis,
    user: Option<Id>,
}

#[derive(Serialize, Deserialize)]
struct State {
    version: u32,
    server: String,
    device: Id,
    label: String,
    signing_key: String,
    recipient: String,
    checkpoint_key: Option<String>,
    org: Option<Id>,
    genesis: Option<Hash>,
    session: Option<StoredSession>,
    log: Vec<Envelope>,
    pins: BTreeMap<Id, Pin>,
    /// Values sealed to this device while the organization allows offline
    /// reads.
    #[serde(default)]
    offline: Vec<Record>,
    /// Reads from the offline copy the server has not accepted yet.
    #[serde(default)]
    journal: BTreeMap<Id, Vec<OfflineRead>>,
}

impl Drop for State {
    fn drop(&mut self) {
        if let Some(session) = &mut self.session {
            session.token.zeroize();
        }
    }
}

struct Opened {
    ring: KeyRing,
    snapshot: Snapshot,
}

/// The lookup texts of one store, with what matching needs around them.
struct Source {
    store: Id,
    label: String,
    settings: StoreSettings,
    texts: BTreeMap<String, String>,
}

struct Inner {
    client: Client,
    state: State,
    login: Option<AppLogin>,
    log: Option<OrgLog>,
    stores: BTreeMap<Id, Opened>,
    /// The opened offline copy, used instead of `stores` while the server
    /// cannot be reached.
    offline: Option<BTreeMap<Id, offline::Store>>,
}

/// A pasejo app installation.
#[derive(uniffi::Object)]
pub struct Device {
    inner: Mutex<Inner>,
}

/// Accepts `https` URLs, and `http` only for the loopback interface, which
/// tests reach through a forwarded port.
fn check_server_url(url: &str) -> Result<String, MobileError> {
    let url = url.trim().trim_end_matches('/');
    let parsed = ureq::http::Uri::try_from(url).map_err(|_| MobileError::invalid("not a URL"))?;
    let loopback = matches!(parsed.host(), Some("localhost" | "127.0.0.1" | "[::1]"));
    let allowed = match parsed.scheme_str() {
        Some("https") => true,
        Some("http") => loopback,
        _ => false,
    };
    if !allowed || parsed.host().is_none_or(str::is_empty) {
        return Err(MobileError::invalid("the server URL must use https"));
    }
    if parsed.query().is_some() || parsed.path() != "/" && !parsed.path().is_empty() {
        return Err(MobileError::invalid("the server URL must not carry a path"));
    }
    Ok(url.to_owned())
}

fn parse_id(text: &str) -> Result<Id, MobileError> {
    text.trim()
        .parse()
        .map_err(|_| MobileError::invalid("not an identifier"))
}

/// The matches best first, and look-alikes only when nothing matched.
fn query_result(
    mut matches: Vec<fill::FillMatch>,
    mut lookalikes: Vec<String>,
    request: FillRequest,
) -> QueryResult {
    matches.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
    });
    if !matches.is_empty() {
        lookalikes.clear();
    }
    lookalikes.sort();
    lookalikes.dedup();
    QueryResult {
        matches,
        lookalikes,
        request,
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn client_for(state: &State, signer: Arc<dyn Signer>, roots: &[Vec<u8>]) -> Client {
    let identity = DeviceIdentity {
        id: state.device,
        signer,
        kind: DeviceKind::Age,
        recipient: state.recipient.clone(),
        label: state.label.clone(),
    };
    let mut client = Client::with_roots(&state.server, identity, system_clock(), TIMEOUT, roots);
    if let Some(session) = &state.session {
        client.set_session(Some(Session {
            token: zeroize::Zeroizing::new(session.token.clone()),
            expires: session.expires,
            user: session.user,
        }));
    }
    client
}

/// An age identity sharing one [`AgreementIdentity`], so the failure it
/// records can be read after age is done with it.
struct Shared(Arc<AgreementIdentity>);

impl age::Identity for Shared {
    fn unwrap_stanza(
        &self,
        stanza: &age_core::format::Stanza,
    ) -> Option<Result<age_core::format::FileKey, age::DecryptError>> {
        self.0.unwrap_stanza(stanza)
    }
}

impl Device {
    fn lock_inner(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Inner {
    fn org(&self) -> Result<(Id, Hash), MobileError> {
        match (self.state.org, self.state.genesis) {
            (Some(org), Some(genesis)) => Ok((org, genesis)),
            _ => Err(MobileError::invalid(
                "the device has not joined an organization",
            )),
        }
    }

    fn remember_session(&mut self, session: &Session) {
        self.state.session = Some(StoredSession {
            token: session.token.to_string(),
            expires: session.expires,
            user: session.user,
        });
    }

    fn forget(&mut self) {
        self.stores.clear();
        self.offline = None;
        self.login = None;
    }

    /// Converts a client error, and drops the session and every opened key
    /// when the server no longer accepts the session, as it does once the
    /// device is revoked.
    fn checked<T>(&mut self, result: pasejo_client::Result<T>) -> Result<T, MobileError> {
        result.map_err(|error| {
            let error = MobileError::from(error);
            if error == MobileError::NotSignedIn {
                self.forget();
                self.state.session = None;
                self.client.set_session(None);
            }
            error
        })
    }

    fn fetch_log(&mut self) -> Result<&OrgLog, MobileError> {
        let (org, genesis) = self.org()?;
        let known = self
            .log
            .take()
            .or_else(|| OrgLog::replay(self.state.log.clone()).ok())
            .filter(|log| log.genesis_hash() == genesis);
        let fetched = self.client.fetch_log(org, known, Some(genesis));
        let log = self.checked(fetched)?;
        self.state.log = log.entries().to_vec();
        if log.state().active_device(self.state.device).is_none() {
            self.stores.clear();
            self.state.offline.clear();
        }
        if !log.state().settings.offline_reads {
            self.state.offline.clear();
        }
        Ok(self.log.insert(log))
    }

    fn known_log(&mut self) -> Result<&OrgLog, MobileError> {
        if self.log.is_some() {
            return self.log.as_ref().ok_or(MobileError::Locked);
        }
        self.fetch_log()
    }

    /// Fetches the store's current index, refusing one older than the pin.
    fn fresh(&mut self, store: Id) -> Result<&Opened, MobileError> {
        let log = self.log.as_ref().ok_or(MobileError::Locked)?;
        let opened = self.stores.get(&store).ok_or(MobileError::NotFound)?;
        let fetched = self
            .client
            .index(log, &opened.ring, self.state.pins.get(&store).copied());
        let snapshot = self.checked(fetched)?;
        if let Some(pin) = snapshot.pin() {
            self.state.pins.insert(store, pin);
        }
        let opened = self.stores.get_mut(&store).ok_or(MobileError::NotFound)?;
        opened.snapshot = snapshot;
        Ok(opened)
    }

    fn object(&mut self, store: Id, kind: Kind, name: &str) -> Result<ObjectPlain, MobileError> {
        self.fresh(store)?;
        let log = self.log.as_ref().ok_or(MobileError::Locked)?;
        let opened = self.stores.get(&store).ok_or(MobileError::NotFound)?;
        let known = match kind {
            Kind::Secret => opened.snapshot.plain.secrets.contains_key(name),
            Kind::Otp => opened.snapshot.plain.otp.contains_key(name),
        };
        if !known {
            return Err(MobileError::NotFound);
        }
        let key = (kind, name.to_owned());
        let mut values = self.client.objects(
            log,
            &opened.ring,
            &opened.snapshot,
            std::slice::from_ref(&key),
        )?;
        values.remove(&key).ok_or(MobileError::NotFound)
    }

    /// Whether the cached log, verified against the pinned genesis, lets
    /// this device read the offline copy it holds.
    fn offline_allowed(&self) -> bool {
        let Ok((_, genesis)) = self.org() else {
            return false;
        };
        !self.state.offline.is_empty()
            && OrgLog::replay(self.state.log.clone())
                .ok()
                .filter(|log| log.genesis_hash() == genesis)
                .is_some_and(|log| {
                    log.state().settings.offline_reads
                        && log.state().active_device(self.state.device).is_some()
                })
    }

    /// Adds a value fetched from the server to the offline copy while the
    /// organization allows offline reads.
    fn remember(
        &mut self,
        store: Id,
        kind: Kind,
        name: &str,
        object: &ObjectPlain,
    ) -> Result<(), MobileError> {
        let allowed = self
            .log
            .as_ref()
            .is_some_and(|log| log.state().settings.offline_reads);
        if !allowed {
            return Ok(());
        }
        let plain = &self
            .stores
            .get(&store)
            .ok_or(MobileError::NotFound)?
            .snapshot
            .plain;
        let entry = match kind {
            Kind::Secret => plain.secrets.get(name),
            Kind::Otp => plain.otp.get(name),
        }
        .ok_or(MobileError::NotFound)?;
        let record = offline::seal(
            &self.state.recipient,
            store,
            entry.id,
            entry.version,
            &offline::Plain {
                label: plain.label.clone(),
                settings: plain.settings.clone(),
                name: name.to_owned(),
                object: object.clone(),
            },
        )?;
        offline::upsert(&mut self.state.offline, record);
        Ok(())
    }

    /// Reads one value from the server, or from the opened offline copy,
    /// journaling the read for the audit log.
    fn read(&mut self, store: Id, kind: Kind, name: &str) -> Result<ObjectPlain, MobileError> {
        if let Some(stores) = &self.offline {
            let copy = stores.get(&store).ok_or(MobileError::NotFound)?;
            let (id, version, object) = match kind {
                Kind::Secret => copy.secrets.get(name).map(|entry| {
                    let secret = entry.value.to_string();
                    (entry.id, entry.version, ObjectPlain::Secret { secret })
                }),
                Kind::Otp => copy.otp.get(name).map(|entry| {
                    let otp = entry.value.clone();
                    (entry.id, entry.version, ObjectPlain::Otp { otp })
                }),
            }
            .ok_or(MobileError::NotFound)?;
            let time = self.client.now();
            self.state
                .journal
                .entry(store)
                .or_default()
                .push(OfflineRead { id, version, time });
            return Ok(object);
        }
        self.known_log()?;
        let object = self.object(store, kind, name)?;
        self.remember(store, kind, name, &object)?;
        Ok(object)
    }

    /// The label and settings of an opened store, and whether it holds a
    /// one-time password named `name`.
    fn store_meta(
        &self,
        store: Id,
        name: &str,
    ) -> Result<(String, StoreSettings, bool), MobileError> {
        if let Some(stores) = &self.offline {
            let copy = stores.get(&store).ok_or(MobileError::NotFound)?;
            return Ok((
                copy.label.clone(),
                copy.settings.clone(),
                copy.otp.contains_key(name),
            ));
        }
        let plain = &self
            .stores
            .get(&store)
            .ok_or(MobileError::NotFound)?
            .snapshot
            .plain;
        Ok((
            plain.label.clone(),
            plain.settings.clone(),
            plain.otp.contains_key(name),
        ))
    }

    /// The lookup texts of every opened store: from the server, where it
    /// records one lookup per store, or from the offline copy, where
    /// matching reads the text below each secret's password line as the
    /// lookup object holds it.
    fn lookups(&mut self) -> Result<Vec<Source>, MobileError> {
        if let Some(stores) = &self.offline {
            return Ok(stores
                .iter()
                .map(|(store, copy)| Source {
                    store: *store,
                    label: copy.label.clone(),
                    settings: copy.settings.clone(),
                    texts: copy
                        .secrets
                        .iter()
                        .map(|(name, entry)| {
                            let below = entry.value.split_once('\n').map_or("", |(_, rest)| rest);
                            (name.clone(), below.to_owned())
                        })
                        .collect(),
                })
                .collect());
        }
        if self.stores.is_empty() {
            return Err(MobileError::Locked);
        }
        let stores: Vec<Id> = self.stores.keys().copied().collect();
        let mut sources = Vec::new();
        for store in stores {
            self.fresh(store)?;
            let log = self.log.as_ref().ok_or(MobileError::Locked)?;
            let opened = self.stores.get(&store).ok_or(MobileError::NotFound)?;
            let lookup = self.client.lookup(log, &opened.ring, &opened.snapshot)?;
            sources.push(Source {
                store,
                label: opened.snapshot.plain.label.clone(),
                settings: opened.snapshot.plain.settings.clone(),
                texts: lookup.secrets.clone(),
            });
        }
        Ok(sources)
    }

    /// Reports the journaled offline reads. Reads the server refuses, such
    /// as reads outside the periods the log allowed or of a store this
    /// device lost, are dropped; reads that could not be sent stay.
    fn report_journal(&mut self, log: &OrgLog) {
        for (store, reads) in std::mem::take(&mut self.state.journal) {
            if let Err(pasejo_client::Error::Transport(_)) =
                self.client.report_offline_reads(log, store, &reads)
            {
                self.state.journal.insert(store, reads);
            }
        }
    }

    fn views(&self) -> Vec<StoreView> {
        let mut views: Vec<StoreView> = self.offline.as_ref().map_or_else(
            || {
                self.stores
                    .iter()
                    .map(|(id, opened)| StoreView {
                        id: id.to_string(),
                        label: opened.snapshot.plain.label.clone(),
                        secrets: opened.snapshot.plain.secrets.keys().cloned().collect(),
                        otp: opened.snapshot.plain.otp.keys().cloned().collect(),
                    })
                    .collect()
            },
            |stores| {
                stores
                    .iter()
                    .map(|(id, copy)| StoreView {
                        id: id.to_string(),
                        label: copy.label.clone(),
                        secrets: copy.secrets.keys().cloned().collect(),
                        otp: copy.otp.keys().cloned().collect(),
                    })
                    .collect()
            },
        );
        views.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
        views
    }
}

// UniFFI passes arguments from the foreign side as owned values. Each call
// holds the device lock until it returns, so one device makes one request at
// a time and no call sees another's half-updated state.
#[allow(clippy::needless_pass_by_value, clippy::significant_drop_tightening)]
#[uniffi::export]
impl Device {
    /// A new device for the server at `server_url`, with a fresh device
    /// identifier. `recipient_key` is the X25519 public key of the device's
    /// age identity. TLS trusts exactly the DER certificates in
    /// `trust_anchors`, the ones the phone trusts, or the bundled
    /// certificate authorities when it is empty.
    ///
    /// # Errors
    ///
    /// Fails for a URL other than `https`, for malformed keys and for a
    /// signer whose signatures do not verify.
    #[uniffi::constructor]
    pub fn create(
        server_url: String,
        label: String,
        signer: Arc<dyn DeviceSigner>,
        recipient_key: Vec<u8>,
        trust_anchors: Vec<Vec<u8>>,
    ) -> Result<Arc<Self>, MobileError> {
        let server = check_server_url(&server_url)?;
        let signer = SignerAdapter::new(signer)?;
        signer.probe()?;
        let state = State {
            version: STATE_VERSION,
            server,
            device: Id::random(),
            label,
            signing_key: b64(&signer.verifying_key()),
            recipient: recipient(recipient_key)?,
            checkpoint_key: None,
            org: None,
            genesis: None,
            session: None,
            log: Vec::new(),
            pins: BTreeMap::new(),
            offline: Vec::new(),
            journal: BTreeMap::new(),
        };
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                client: client_for(&state, Arc::new(signer), &trust_anchors),
                state,
                login: None,
                log: None,
                stores: BTreeMap::new(),
                offline: None,
            }),
        }))
    }

    /// A device from the state [`Device::state`] returned, trusting
    /// `trust_anchors` as [`Device::create`] does.
    ///
    /// # Errors
    ///
    /// Fails for malformed state and for a signer whose public key is not
    /// the device's.
    #[uniffi::constructor]
    pub fn restore(
        state: String,
        signer: Arc<dyn DeviceSigner>,
        trust_anchors: Vec<Vec<u8>>,
    ) -> Result<Arc<Self>, MobileError> {
        let state: State = serde_json::from_str(&state)
            .map_err(|_| MobileError::invalid("malformed device state"))?;
        if state.version != STATE_VERSION {
            return Err(MobileError::invalid("unsupported device state"));
        }
        check_server_url(&state.server)?;
        let signer = SignerAdapter::new(signer)?;
        if b64(&signer.verifying_key()) != state.signing_key {
            return Err(MobileError::key("the signing key is not this device's"));
        }
        Ok(Arc::new(Self {
            inner: Mutex::new(Inner {
                client: client_for(&state, Arc::new(signer), &trust_anchors),
                state,
                login: None,
                log: None,
                stores: BTreeMap::new(),
                offline: None,
            }),
        }))
    }

    /// The state to store, encrypted, and pass to [`Device::restore`]. It
    /// holds the session token and the cached membership log, never keys or
    /// values.
    #[must_use]
    pub fn state(&self) -> String {
        serde_json::to_string(&self.lock_inner().state).unwrap_or_default()
    }

    #[must_use]
    pub fn device_id(&self) -> String {
        self.lock_inner().state.device.to_string()
    }

    #[must_use]
    pub fn server_url(&self) -> String {
        self.lock_inner().state.server.clone()
    }

    /// The organization this device joined.
    #[must_use]
    pub fn organization(&self) -> Option<String> {
        self.lock_inner().state.org.map(|org| org.to_string())
    }

    /// The fingerprint an approver compares before approving this device.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.lock_inner().client.device.fingerprint()
    }

    /// Fetches the server's description and pins its checkpoint key on
    /// first use, or to `checkpoint_key` when managed configuration
    /// supplies one.
    ///
    /// # Errors
    ///
    /// Fails when the server cannot be reached, or its checkpoint key
    /// differs from the pinned one.
    pub fn connect(&self, checkpoint_key: Option<String>) -> Result<ServerInfo, MobileError> {
        let mut inner = self.lock_inner();
        let info = inner.client.server_info()?;
        let offered = b64(&info.checkpoint_key.0);
        let pinned = inner
            .state
            .checkpoint_key
            .iter()
            .chain(checkpoint_key.iter());
        if pinned
            .into_iter()
            .any(|expected| expected.trim() != offered)
        {
            return Err(MobileError::verification(
                "the server's checkpoint key differs from the pinned one",
            ));
        }
        inner.state.checkpoint_key = Some(offered.clone());
        Ok(ServerInfo {
            version: info.version,
            checkpoint_key: offered,
            oidc: info.oidc,
        })
    }

    /// Starts a sign-in through the system browser and returns the URL to
    /// open. The browser returns to `redirect_uri`.
    #[must_use]
    pub fn start_sign_in(&self, redirect_uri: String) -> String {
        let mut inner = self.lock_inner();
        let login = inner.client.start_app_login(&redirect_uri);
        let url = login.url.clone();
        inner.login = Some(login);
        url
    }

    /// Finishes the sign-in with the redirect the browser returned to.
    ///
    /// # Errors
    ///
    /// Fails without a sign-in in progress, for a redirect of another
    /// sign-in, and when the server refuses the code.
    pub fn finish_sign_in(&self, redirect: String) -> Result<(), MobileError> {
        let mut inner = self.lock_inner();
        let login = inner
            .login
            .take()
            .ok_or_else(|| MobileError::invalid("no sign-in is in progress"))?;
        let session = inner.client.finish_app_login(&login, &redirect)?;
        inner.remember_session(&session);
        Ok(())
    }

    #[must_use]
    pub fn signed_in(&self) -> bool {
        let inner = self.lock_inner();
        inner
            .state
            .session
            .as_ref()
            .is_some_and(|session| session.expires > inner.client.now())
    }

    /// Ends the session and locks the stores.
    ///
    /// # Errors
    ///
    /// Fails when the server cannot be told; the session is forgotten
    /// locally either way.
    pub fn sign_out(&self) -> Result<(), MobileError> {
        let mut inner = self.lock_inner();
        if !inner.state.journal.is_empty()
            && let Ok(log) = inner.fetch_log().cloned()
        {
            inner.report_journal(&log);
        }
        inner.state.offline.clear();
        inner.forget();
        let session = inner.state.session.take();
        let result = if session.is_some() {
            inner.client.logout().map_err(MobileError::from)
        } else {
            Ok(())
        };
        inner.client.set_session(None);
        result
    }

    /// Joins `org`, whose genesis must hash to `genesis`, the fingerprint
    /// an admin showed or managed configuration supplied, and enrolls this
    /// device unless the organization already knows it.
    ///
    /// # Errors
    ///
    /// Fails for malformed input, an organization other than the one this
    /// device joined, and when the server refuses.
    pub fn join(&self, org: String, genesis: String) -> Result<Enrollment, MobileError> {
        let org = parse_id(&org)?;
        let genesis: Hash = genesis
            .trim()
            .parse()
            .map_err(|_| MobileError::invalid("not an organization fingerprint"))?;
        let mut inner = self.lock_inner();
        if inner.state.org.is_some_and(|joined| joined != org)
            || inner.state.genesis.is_some_and(|pinned| pinned != genesis)
        {
            return Err(MobileError::invalid(
                "this device belongs to another organization",
            ));
        }
        inner.state.org = Some(org);
        inner.state.genesis = Some(genesis);
        let status = match inner.client.enrollment_status(org) {
            Ok(status) => status,
            Err(error) if error.is(ErrorCode::NotFound) => EnrollmentStatus {
                pending: false,
                active: false,
            },
            Err(error) => return Err(error.into()),
        };
        if status.active || status.pending {
            return Ok(Enrollment {
                pending: status.pending,
                active: status.active,
            });
        }
        inner.client.enroll(org)?;
        let status = inner.client.enrollment_status(org)?;
        Ok(Enrollment {
            pending: status.pending,
            active: status.active,
        })
    }

    /// Whether the organization approved this device yet.
    ///
    /// # Errors
    ///
    /// Fails before [`Device::join`] and when the server refuses.
    pub fn enrollment(&self) -> Result<Enrollment, MobileError> {
        let inner = self.lock_inner();
        let (org, _) = inner.org()?;
        let status = inner.client.enrollment_status(org)?;
        Ok(Enrollment {
            pending: status.pending,
            active: status.active,
        })
    }

    /// Fetches and verifies the membership log against the pinned genesis.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, and when the log does not verify.
    pub fn refresh(&self) -> Result<OrgInfo, MobileError> {
        let mut inner = self.lock_inner();
        let device = inner.state.device;
        let log = inner.fetch_log()?;
        let state = log.state();
        Ok(OrgInfo {
            name: state.name.clone(),
            active: state.active_device(device).is_some(),
            offline_reads: state.settings.offline_reads,
        })
    }

    /// Opens the keys of every store this device holds a wrapping for,
    /// using `identity`, and returns the stores with their entry names. The
    /// keys stay in memory until [`Device::lock`].
    ///
    /// # Errors
    ///
    /// Fails when `identity` refuses, when keys or indexes do not verify,
    /// and when the server refuses.
    pub fn unlock(&self, identity: Arc<dyn KeyAgreement>) -> Result<Vec<StoreView>, MobileError> {
        let identity = Arc::new(AgreementIdentity::new(identity)?);
        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(Shared(Arc::clone(&identity)))];
        let mut inner = self.lock_inner();
        inner.offline = None;
        let log = match inner.fetch_log().cloned() {
            Ok(log) => log,
            Err(MobileError::Network { reason }) => {
                if !inner.offline_allowed() {
                    return Err(MobileError::Network { reason });
                }
                let stores = offline::view(&Shared(Arc::clone(&identity)), &inner.state.offline)
                    .map_err(|error| identity.take_failure().unwrap_or(error))?;
                inner.stores.clear();
                inner.offline = Some(stores);
                return Ok(inner.views());
            }
            Err(error) => return Err(error),
        };
        inner.report_journal(&log);
        if log.state().active_device(inner.state.device).is_none() {
            return Err(MobileError::invalid(
                "the organization has not approved this device",
            ));
        }
        let mut opened = BTreeMap::new();
        for (store, state) in &log.state().stores {
            if state.deleted {
                continue;
            }
            let ring = match inner.client.keys(&log, *store, &identities) {
                Ok(ring) => ring,
                Err(pasejo_client::Error::Unavailable(_)) => continue,
                Err(error) => return Err(identity.take_failure().unwrap_or_else(|| error.into())),
            };
            let snapshot = inner
                .client
                .index(&log, &ring, inner.state.pins.get(store).copied())?;
            if let Some(pin) = snapshot.pin() {
                inner.state.pins.insert(*store, pin);
            }
            opened.insert(*store, Opened { ring, snapshot });
        }
        inner.stores = opened;
        let kept: Vec<Id> = inner.stores.keys().copied().collect();
        inner
            .state
            .offline
            .retain(|record| kept.contains(&record.store));
        Ok(inner.views())
    }

    /// Drops every opened store key.
    pub fn lock(&self) {
        let mut inner = self.lock_inner();
        inner.stores.clear();
        inner.offline = None;
    }

    #[must_use]
    pub fn unlocked(&self) -> bool {
        let inner = self.lock_inner();
        !inner.stores.is_empty() || inner.offline.is_some()
    }

    /// Whether the stores are open from the offline copy because the
    /// server could not be reached.
    #[must_use]
    pub fn offline(&self) -> bool {
        self.lock_inner().offline.is_some()
    }

    /// The opened stores, without contacting the server.
    #[must_use]
    pub fn stores(&self) -> Vec<StoreView> {
        self.lock_inner().views()
    }

    /// Fetches the text of one secret. The server records the read.
    ///
    /// # Errors
    ///
    /// Fails while locked, for unknown names, and when the value does not
    /// verify.
    pub fn reveal(&self, store: String, name: String) -> Result<String, MobileError> {
        let store = parse_id(&store)?;
        let mut inner = self.lock_inner();
        match &inner.read(store, Kind::Secret, &name)? {
            ObjectPlain::Secret { secret } => Ok(secret.clone()),
            ObjectPlain::Otp { .. } => Err(MobileError::verification("the object is not a secret")),
        }
    }

    /// The current TOTP code of one entry. The server records the read.
    ///
    /// # Errors
    ///
    /// Fails while locked, for unknown names and HOTP entries, and when the
    /// value does not verify.
    pub fn totp(&self, store: String, name: String) -> Result<TotpCode, MobileError> {
        let store = parse_id(&store)?;
        let mut inner = self.lock_inner();
        match &inner.read(store, Kind::Otp, &name)? {
            ObjectPlain::Otp { otp } => totp_at(otp, now_seconds()),
            ObjectPlain::Secret { .. } => Err(MobileError::verification(
                "the object is not a one-time password",
            )),
        }
    }

    /// The secrets of every opened store that match `request`, without
    /// secret material. The server records one lookup per store. For an
    /// app, the hosts of secrets naming it through `androidapp://` join the
    /// request when `verifier` accepts them; the result carries the request
    /// a fill passes on.
    ///
    /// # Errors
    ///
    /// Fails while locked and when a lookup object does not verify.
    pub fn query(
        &self,
        request: FillRequest,
        verifier: Option<Arc<dyn DomainVerifier>>,
    ) -> Result<QueryResult, MobileError> {
        let mut inner = self.lock_inner();
        let mut sources = inner.lookups()?;
        let candidates = request.app.as_ref().map_or_else(Vec::new, |app| {
            let mut all: Vec<String> = sources
                .iter()
                .flat_map(|source| fill::app_candidates(&source.texts, &source.settings, app))
                .collect();
            all.sort();
            all.dedup();
            all
        });
        let request = fill::with_candidates(request, candidates, verifier.as_deref());
        let mut matches = Vec::new();
        let mut lookalikes = Vec::new();
        for source in &mut sources {
            let found = fill::matches(
                &source.texts,
                &source.store.to_string(),
                &source.label,
                &source.settings,
                &request,
            );
            if found.is_empty() {
                lookalikes.extend(fill::lookalikes(&source.texts, &source.settings, &request));
            }
            matches.extend(found);
            source.texts.values_mut().for_each(Zeroize::zeroize);
        }
        Ok(query_result(matches, lookalikes, request))
    }

    /// The credential of one secret, released only when the secret still
    /// matches `request`. A TOTP code is attached only when a URL field
    /// names the requested host itself. The server records each read.
    ///
    /// # Errors
    ///
    /// Fails while locked, for unknown names, for a secret that does not
    /// match the request, and when a value does not verify.
    pub fn fill(
        &self,
        request: FillRequest,
        store: String,
        name: String,
    ) -> Result<FillCredential, MobileError> {
        let store = parse_id(&store)?;
        let mut inner = self.lock_inner();
        let ObjectPlain::Secret { secret } = &inner.read(store, Kind::Secret, &name)? else {
            return Err(MobileError::verification("the object is not a secret"));
        };
        let (label, settings, has_otp) = inner.store_meta(store, &name)?;
        let secret = zeroize::Zeroizing::new(secret.clone());
        let totp = || {
            if !has_otp {
                return None;
            }
            match &inner.read(store, Kind::Otp, &name).ok()? {
                ObjectPlain::Otp { otp } => totp_at(otp, now_seconds()).ok().map(|code| code.code),
                ObjectPlain::Secret { .. } => None,
            }
        };
        fill::release(&name, &secret, &label, &settings, &request, totp)
            .ok_or(MobileError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::keys::{generate_secret, software_agreement, software_signer};

    fn device(url: &str) -> Result<Arc<Device>, MobileError> {
        let agreement = software_agreement(generate_secret()).unwrap();
        Device::create(
            url.into(),
            "phone".into(),
            software_signer(generate_secret()).unwrap(),
            agreement.public_key(),
            vec![],
        )
    }

    #[test]
    fn only_https_and_loopback_http_servers_are_accepted() {
        for url in [
            "https://pasejo.example.com",
            "https://pasejo.example.com/",
            "http://127.0.0.1:8080",
            "http://localhost:1",
        ] {
            assert!(device(url).is_ok(), "{url}");
        }
        for url in [
            "http://pasejo.example.com",
            "ftp://pasejo.example.com",
            "pasejo.example.com",
            "https://",
            "https://pasejo.example.com/api",
            "https://pasejo.example.com/?x=1",
            "http://10.0.2.2:8080",
        ] {
            assert!(device(url).is_err(), "{url}");
        }
    }

    #[test]
    fn the_state_round_trips_with_the_same_signer() {
        let seed = generate_secret();
        let agreement = software_agreement(generate_secret()).unwrap();
        let created = Device::create(
            "https://pasejo.example.com".into(),
            "phone".into(),
            software_signer(seed.clone()).unwrap(),
            agreement.public_key(),
            vec![],
        )
        .unwrap();
        let state = created.state();
        let restored =
            Device::restore(state.clone(), software_signer(seed).unwrap(), vec![]).unwrap();
        assert_eq!(restored.device_id(), created.device_id());
        assert_eq!(restored.fingerprint(), created.fingerprint());
        assert_eq!(restored.state(), state);
        assert!(!restored.signed_in());
        assert!(!restored.unlocked());
    }

    #[test]
    fn a_state_restored_with_another_signer_is_refused() {
        let created = device("https://pasejo.example.com").unwrap();
        let error = Device::restore(
            created.state(),
            software_signer(generate_secret()).unwrap(),
            vec![],
        )
        .err()
        .unwrap();
        assert!(matches!(error, MobileError::Key { .. }), "{error}");
    }

    #[test]
    fn malformed_or_tampered_state_is_refused() {
        let signer = software_signer(generate_secret()).unwrap();
        assert!(Device::restore("{}".into(), Arc::clone(&signer), vec![]).is_err());
        let created = device("https://pasejo.example.com").unwrap();
        let downgraded = created
            .state()
            .replace("https://pasejo.example.com", "http://pasejo.example.com");
        assert!(Device::restore(downgraded, signer, vec![]).is_err());
    }

    #[test]
    fn the_state_holds_no_private_key() {
        let seed = generate_secret();
        let created = Device::create(
            "https://pasejo.example.com".into(),
            "phone".into(),
            software_signer(seed.clone()).unwrap(),
            software_agreement(generate_secret()).unwrap().public_key(),
            vec![],
        )
        .unwrap();
        assert!(!created.state().contains(&b64(&seed)));
    }

    #[test]
    fn reads_while_locked_are_refused() {
        let created = device("https://pasejo.example.com").unwrap();
        let request = FillRequest {
            domains: vec!["example.com".into()],
            app: None,
        };
        assert_eq!(
            created.query(request, None).err(),
            Some(MobileError::Locked)
        );
        assert!(created.stores().is_empty());
    }

    #[test]
    fn joining_needs_well_formed_input() {
        let created = device("https://pasejo.example.com").unwrap();
        assert!(matches!(
            created.join("short".into(), b64(&[0; 32])),
            Err(MobileError::Invalid { .. })
        ));
        assert!(matches!(
            created.join(Id::random().to_string(), "not a hash".into()),
            Err(MobileError::Invalid { .. })
        ));
        assert!(matches!(
            created.refresh(),
            Err(MobileError::Invalid { .. })
        ));
    }

    #[test]
    fn finishing_a_sign_in_that_never_started_is_refused() {
        let created = device("https://pasejo.example.com").unwrap();
        assert!(matches!(
            created.finish_sign_in("wtf.metio.pasejo:/auth?code=x&state=y".into()),
            Err(MobileError::Invalid { .. })
        ));
    }
}
