// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Reading one secret for one tenant. Every read verifies the membership
//! log, the store's keys and index, and the object, as the CLI does, and
//! keeps the plaintext only for the duration of the request.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use pasejo_client::keys::KeyRing;
use pasejo_client::session::session_valid;
use pasejo_client::store::{Kind, Snapshot};
use pasejo_client::transport::{Client, system_clock};
use pasejo_protocol::api::ErrorCode;
use pasejo_protocol::log::OrgLog;
use pasejo_protocol::store::{ObjectPlain, Pin};
use pasejo_protocol::{Hash, Id};
use zeroize::Zeroizing;

use crate::config::Settings;
use crate::device::Device;

/// Which part of a secret a caller asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    All,
    /// A line counted from 1, or from the end when negative.
    Line(isize),
    /// The value of a `key: value` field.
    Field(String),
}

impl Part {
    fn extract(&self, secret: &str) -> Option<Zeroizing<String>> {
        let text = match self {
            Self::All => Some(secret),
            Self::Line(n) => pasejo_protocol::secret::line(secret, *n),
            Self::Field(key) => pasejo_protocol::secret::field(secret, key),
        };
        text.map(|text| Zeroizing::new(text.to_owned()))
    }
}

/// Why a read produced no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The secret is absent from its store's verified index.
    /// external-secrets-operator treats this as a deletion.
    NotFound,
    /// The secret exists but has no such line or field.
    NoSuchPart,
    /// The server, the device's approval, the store, its keys or its index
    /// are not available. external-secrets-operator retries and keeps its
    /// Secrets, so a revoked grant or a mistyped store never deletes them.
    Unavailable(String),
}

impl Failure {
    fn unavailable(error: impl std::fmt::Display) -> Self {
        Self::Unavailable(error.to_string())
    }
}

type Outcome<T> = std::result::Result<T, Failure>;

/// The bridge's connection to its organization.
pub struct Bridge {
    client: Client,
    device: Device,
    org: Id,
    genesis: Hash,
    token_file: PathBuf,
    log: Option<OrgLog>,
    /// Store labels resolved before, checked again on every use.
    labels: HashMap<String, Id>,
    /// The newest index each store showed, so the server cannot roll one
    /// back.
    pins: HashMap<Id, Pin>,
}

impl Bridge {
    #[must_use]
    pub fn new(settings: &Settings, device: Device) -> Self {
        let client = Client::new(
            &settings.config.server,
            device.identity(&settings.config.label),
            system_clock(),
            Duration::from_secs(settings.config.timeout_seconds),
        );
        Self {
            client,
            device,
            org: settings.org,
            genesis: settings.genesis,
            token_file: settings.config.workload_token_file.clone(),
            log: None,
            labels: HashMap::new(),
            pins: HashMap::new(),
        }
    }

    /// The device's fingerprint, which an approver compares.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        self.client.device.fingerprint()
    }

    /// Signs in with the workload token unless the session is still valid.
    fn session(&mut self) -> Outcome<()> {
        let now = self.client.now();
        if self
            .client
            .session()
            .is_some_and(|session| session_valid(session, now))
        {
            return Ok(());
        }
        let token = Zeroizing::new(fs::read_to_string(&self.token_file).map_err(|error| {
            Failure::Unavailable(format!(
                "cannot read the workload token {}: {error}",
                self.token_file.display()
            ))
        })?);
        self.client
            .workload_login(token.trim())
            .map_err(Failure::unavailable)?;
        Ok(())
    }

    /// Asks to join the organization; an existing enrollment counts.
    fn enroll(&self) -> Outcome<()> {
        match self.client.enroll(self.org) {
            Ok(()) => Ok(()),
            Err(error) if error.is(ErrorCode::Conflict) => Ok(()),
            Err(error) => Err(Failure::unavailable(error)),
        }
    }

    /// Signs in, verifies the membership log against the pinned genesis
    /// and checks that the device is active, enrolling it when it is not.
    ///
    /// # Errors
    ///
    /// `Unavailable` while the device waits for approval or the server
    /// cannot be reached.
    pub fn ready(&mut self) -> Outcome<&OrgLog> {
        self.session()?;
        let fetched = match self
            .client
            .fetch_log(self.org, self.log.take(), Some(self.genesis))
        {
            Err(error) if error.is(ErrorCode::Forbidden) => {
                // Only members and enrolling devices read the log.
                self.enroll()?;
                self.client.fetch_log(self.org, None, Some(self.genesis))
            }
            other => other,
        };
        let log = fetched.map_err(Failure::unavailable)?;
        if log.state().active_device(self.device.id).is_none() {
            let pending = self
                .client
                .enrollment_status(self.org)
                .is_ok_and(|status| status.pending);
            if !pending {
                self.enroll()?;
            }
            self.log = Some(log);
            return Err(Failure::Unavailable(format!(
                "device {} waits for approval (fingerprint {})",
                self.device.id,
                self.fingerprint()
            )));
        }
        Ok(self.log.insert(log))
    }

    /// Opens `store`'s keys and verified index.
    fn open(&mut self, log: &OrgLog, store: Id) -> Outcome<(KeyRing, Snapshot)> {
        let ring = self
            .client
            .keys(log, store, &self.device.identities())
            .map_err(Failure::unavailable)?;
        let snapshot = self
            .client
            .index(log, &ring, self.pins.get(&store).copied())
            .map_err(Failure::unavailable)?;
        if let Some(pin) = snapshot.pin() {
            self.pins.insert(store, pin);
        }
        Ok((ring, snapshot))
    }

    /// The store named by identifier or label. A label resolved before is
    /// checked against its store's current index; otherwise every store the
    /// device sees is opened.
    fn resolve(&mut self, log: &OrgLog, store: &str) -> Outcome<(KeyRing, Snapshot)> {
        let visible = self.client.stores(self.org).map_err(Failure::unavailable)?;
        if let Ok(id) = Id::from_str(store)
            && visible.iter().any(|summary| summary.id == id)
        {
            return self.open(log, id);
        }
        if let Some(id) = self.labels.get(store).copied()
            && visible.iter().any(|summary| summary.id == id)
        {
            let opened = self.open(log, id)?;
            if opened.1.plain.label == store {
                return Ok(opened);
            }
        }
        self.labels.remove(store);
        let mut found = None;
        let mut failure = None;
        for summary in visible {
            match self.open(log, summary.id) {
                Ok(opened) => {
                    self.labels.insert(opened.1.plain.label.clone(), summary.id);
                    if opened.1.plain.label == store && found.is_none() {
                        found = Some(opened);
                    }
                }
                Err(error) => failure = Some(error),
            }
        }
        match (found, failure) {
            (Some(opened), _) => Ok(opened),
            (None, Some(failure)) => Err(failure),
            (None, None) => Err(Failure::Unavailable(format!(
                "no store this device holds keys for is called '{store}'"
            ))),
        }
    }

    /// Reads `part` of secret `name` in `store` on behalf of `tenant`.
    ///
    /// # Errors
    ///
    /// See [`Failure`].
    pub fn read(
        &mut self,
        store: &str,
        name: &str,
        part: &Part,
        tenant: &str,
    ) -> Outcome<Zeroizing<String>> {
        let log = self.ready()?.clone();
        let (ring, snapshot) = self.resolve(&log, store)?;
        if !snapshot.plain.secrets.contains_key(name) {
            return Err(Failure::NotFound);
        }
        let mut objects = self
            .client
            .objects_for(
                &log,
                &ring,
                &snapshot,
                &[(Kind::Secret, name.to_owned())],
                Some(tenant),
            )
            .map_err(Failure::unavailable)?;
        let object = objects.remove(&(Kind::Secret, name.to_owned()));
        let Some(ObjectPlain::Secret { secret }) = object.as_ref() else {
            return Err(Failure::Unavailable(
                "the server returned no secret".to_owned(),
            ));
        };
        part.extract(secret).ok_or(Failure::NoSuchPart)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_select_the_whole_secret_a_line_or_a_field() {
        let secret = "hunter2\nuser: alice\nurl: https://example.com";
        let extract = |part: Part| part.extract(secret).map(|text| text.to_string());
        assert_eq!(extract(Part::All).as_deref(), Some(secret));
        assert_eq!(extract(Part::Line(1)).as_deref(), Some("hunter2"));
        assert_eq!(
            extract(Part::Line(-1)).as_deref(),
            Some("url: https://example.com")
        );
        assert_eq!(extract(Part::Line(4)), None);
        assert_eq!(
            extract(Part::Field("user".into())).as_deref(),
            Some("alice")
        );
        assert_eq!(extract(Part::Field("password".into())), None);
    }
}
