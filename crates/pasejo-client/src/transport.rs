// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The HTTP transport: one agent with a timeout, request proofs bound to
//! the device key, and JSON in both directions.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use pasejo_protocol::api::ErrorBody;
use pasejo_protocol::device::{DeviceKind, DeviceRecord};
use pasejo_protocol::encoding::Fixed;
use pasejo_protocol::request::{NONCE_HEADER, PROOF_HEADER, TIMESTAMP_HEADER, prove};
use pasejo_protocol::{Id, Millis};
use serde::Serialize;
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::error::{Error, Result};

/// The time in milliseconds since the Unix epoch. Replaceable so tests can
/// share a server's clock.
pub type Clock = Arc<dyn Fn() -> Millis + Send + Sync>;

/// The system clock.
#[must_use]
pub fn system_clock() -> Clock {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
            })
    })
}

/// The keys and description of this installation.
pub struct DeviceIdentity {
    pub id: Id,
    pub signing_key: SigningKey,
    pub kind: DeviceKind,
    /// The age recipient (or HPKE public key) wrappings go to.
    pub recipient: String,
    pub label: String,
}

impl DeviceIdentity {
    /// The device as the membership log records it for `user`.
    #[must_use]
    pub fn record(&self, user: Id) -> DeviceRecord {
        DeviceRecord {
            id: self.id,
            user,
            kind: self.kind,
            recipient: self.recipient.clone(),
            signing_key: Fixed(self.signing_key.verifying_key().to_bytes()),
            label: self.label.clone(),
        }
    }

    /// The fingerprint approvers compare.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        pasejo_protocol::device::fingerprint(
            self.kind,
            &self.recipient,
            &self.signing_key.verifying_key().to_bytes(),
        )
    }
}

/// A session token and what the server said about it.
#[derive(Clone)]
pub struct Session {
    pub token: Zeroizing<String>,
    pub expires: Millis,
    pub user: Option<Id>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Session")
            .field("expires", &self.expires)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

/// A connection to one server as one device.
pub struct Client {
    base: String,
    agent: ureq::Agent,
    clock: Clock,
    pub device: DeviceIdentity,
    session: Option<Session>,
}

/// The default time limit of one request.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

impl Client {
    /// A client for the server at `base_url`.
    #[must_use]
    pub fn new(base_url: &str, device: DeviceIdentity, clock: Clock, timeout: Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .max_redirects(0)
            .build()
            .into();
        Self {
            base: base_url.trim_end_matches('/').to_owned(),
            agent,
            clock,
            device,
            session: None,
        }
    }

    #[must_use]
    pub fn now(&self) -> Millis {
        (self.clock)()
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base
    }

    pub fn set_session(&mut self, session: Option<Session>) {
        self.session = session;
    }

    #[must_use]
    pub const fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// The signed-in user.
    ///
    /// # Errors
    ///
    /// Fails without a session naming a user.
    pub fn user(&self) -> Result<Id> {
        self.session
            .as_ref()
            .and_then(|session| session.user)
            .ok_or(Error::NoSession)
    }

    /// Sends a request with a proof when `authenticated`, and returns the
    /// status and body.
    fn send(
        &self,
        method: &str,
        path: &str,
        body: Option<Vec<u8>>,
        authenticated: bool,
    ) -> Result<(u16, Vec<u8>)> {
        let url = format!("{}{path}", self.base);
        let mut headers: Vec<(&str, String)> = Vec::new();
        if authenticated {
            let session = self.session.as_ref().ok_or(Error::NoSession)?;
            let proof = prove(
                method,
                path,
                self.now(),
                body.as_deref().unwrap_or_default(),
                &session.token,
                &self.device.signing_key,
            );
            headers.push(("authorization", proof.authorization));
            headers.push((TIMESTAMP_HEADER, proof.timestamp));
            headers.push((NONCE_HEADER, proof.nonce));
            headers.push((PROOF_HEADER, proof.proof));
        }
        let response = match (method, body) {
            ("GET", _) => {
                let mut request = self.agent.get(&url);
                for (name, value) in &headers {
                    request = request.header(*name, value);
                }
                request.call()
            }
            ("DELETE", _) => {
                let mut request = self.agent.delete(&url);
                for (name, value) in &headers {
                    request = request.header(*name, value);
                }
                request.call()
            }
            (method, body) => {
                let mut request = match method {
                    "PUT" => self.agent.put(&url),
                    _ => self.agent.post(&url),
                };
                for (name, value) in &headers {
                    request = request.header(*name, value);
                }
                request = request.header("content-type", "application/json");
                request.send(body.unwrap_or_default())
            }
        };
        let mut response = response.map_err(|error| Error::Transport(error.to_string()))?;
        let status = response.status().as_u16();
        let bytes = response
            .body_mut()
            .with_config()
            .limit(64 * 1024 * 1024)
            .read_to_vec()
            .map_err(|error| Error::Transport(error.to_string()))?;
        Ok((status, bytes))
    }

    fn decode<T: DeserializeOwned>(status: u16, bytes: &[u8]) -> Result<T> {
        if (200..300).contains(&status) {
            serde_json::from_slice(bytes)
                .map_err(|error| Error::verification(format!("malformed answer: {error}")))
        } else {
            Err(api_error(status, bytes))
        }
    }

    fn encode<B: Serialize>(body: &B) -> Result<Vec<u8>> {
        serde_json::to_vec(body).map_err(|error| Error::Protocol(error.into()))
    }

    /// An authenticated `GET`.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses or answers something malformed.
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let (status, bytes) = self.send("GET", path, None, true)?;
        Self::decode(status, &bytes)
    }

    /// An authenticated request with a JSON body and a JSON answer.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses or answers something malformed.
    pub fn call<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let (status, bytes) = self.send(method, path, Some(Self::encode(body)?), true)?;
        Self::decode(status, &bytes)
    }

    /// An authenticated request whose answer carries no body.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn call_empty<B: Serialize>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<()> {
        let body = body.map(Self::encode).transpose()?;
        let (status, bytes) = self.send(method, path, body, true)?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(api_error(status, &bytes))
        }
    }

    /// An unauthenticated request.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses or answers something malformed.
    pub fn anonymous<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T> {
        let body = body.map(Self::encode).transpose()?;
        let (status, bytes) = self.send(method, path, body, false)?;
        Self::decode(status, &bytes)
    }
}

fn api_error(status: u16, bytes: &[u8]) -> Error {
    match serde_json::from_slice::<ErrorBody>(bytes) {
        Ok(body) => Error::Api {
            status,
            code: Some(body.code),
            message: body.message,
            current_version: body.current_version,
        },
        Err(_) => Error::Api {
            status,
            code: None,
            message: String::from_utf8_lossy(bytes).chars().take(200).collect(),
            current_version: None,
        },
    }
}
