// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Signing in: the device authorization grant for people, workload tokens
//! for machines and the emergency window, plus the server's description.

use pasejo_protocol::api::{
    DeviceAuthorization, DeviceAuthorizationRequest, ErrorCode, Me, ServerInfo, TokenRequest,
    TokenResponse, purpose,
};
use pasejo_protocol::encoding::{Fixed, b64, sha256};
use pasejo_protocol::envelope::{Context, Envelope, TokenClaim};
use zeroize::Zeroizing;

use crate::error::Result;
use crate::transport::{Client, Session};

/// What polling a device grant found.
#[derive(Debug)]
pub enum Poll {
    /// The user has not approved yet.
    Pending,
    /// The client polls too often and should wait longer.
    SlowDown,
    /// The grant was approved.
    Done(Session),
}

impl Client {
    /// The server's version, checkpoint key and sign-in options.
    ///
    /// # Errors
    ///
    /// Fails when the server cannot be reached.
    pub fn server_info(&self) -> Result<ServerInfo> {
        self.anonymous::<(), _>("GET", "/v1/server", None)
    }

    fn claim(&self, purpose: &str, value: String) -> Result<Envelope> {
        Ok(Envelope::seal(
            Context::Token,
            &TokenClaim {
                purpose: purpose.to_owned(),
                value,
                signing_key: Fixed(self.device.signing_key.verifying_key().to_bytes()),
                time: self.now(),
            },
            self.device.id,
            &self.device.signing_key,
        )?)
    }

    /// Starts a device authorization grant; the user approves it in a
    /// browser at the returned URL.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn start_login(&self) -> Result<DeviceAuthorization> {
        self.anonymous(
            "POST",
            "/v1/auth/device",
            Some(&DeviceAuthorizationRequest {
                signing_key: Fixed(self.device.signing_key.verifying_key().to_bytes()),
                label: self.device.label.clone(),
            }),
        )
    }

    /// Polls a device grant once. On success the client keeps the session.
    ///
    /// # Errors
    ///
    /// Fails when the grant was denied or expired.
    pub fn poll_login(&mut self, authorization: &DeviceAuthorization) -> Result<Poll> {
        let claim = self.claim(purpose::DEVICE, authorization.device_code.clone())?;
        let request = TokenRequest { claim, jwt: None };
        match self.anonymous::<_, TokenResponse>("POST", "/v1/auth/token", Some(&request)) {
            Ok(response) => Ok(Poll::Done(self.adopt(response))),
            Err(error) if error.is(ErrorCode::AuthorizationPending) => Ok(Poll::Pending),
            Err(error) if error.is(ErrorCode::SlowDown) => Ok(Poll::SlowDown),
            Err(error) => Err(error),
        }
    }

    /// Signs a machine in with a workload token.
    ///
    /// # Errors
    ///
    /// Fails when the server does not accept the token.
    pub fn workload_login(&mut self, jwt: &str) -> Result<Session> {
        let claim = self.claim(purpose::WORKLOAD, b64(&sha256(jwt.as_bytes())))?;
        let request = TokenRequest {
            claim,
            jwt: Some(jwt.to_owned()),
        };
        let response = self.anonymous("POST", "/v1/auth/workload", Some(&request))?;
        Ok(self.adopt(response))
    }

    /// Signs an enrolled device in while an emergency window is open.
    ///
    /// # Errors
    ///
    /// Fails when no window is open or the device is unknown.
    pub fn emergency_login(&mut self) -> Result<Session> {
        let claim = self.claim(purpose::EMERGENCY, String::new())?;
        let request = TokenRequest { claim, jwt: None };
        let response = self.anonymous("POST", "/v1/auth/emergency", Some(&request))?;
        Ok(self.adopt(response))
    }

    fn adopt(&mut self, response: TokenResponse) -> Session {
        let session = Session {
            token: Zeroizing::new(response.token),
            expires: response.expires,
            user: response.user,
        };
        self.set_session(Some(session.clone()));
        session
    }

    /// Ends the session on the server and forgets it.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses; the session is forgotten anyway.
    pub fn logout(&mut self) -> Result<()> {
        let result = self.call_empty::<()>("POST", "/v1/auth/logout", None);
        self.set_session(None);
        result
    }

    /// The signed-in user and their organizations.
    ///
    /// # Errors
    ///
    /// Fails without a valid session.
    pub fn me(&self) -> Result<Me> {
        self.get("/v1/me")
    }
}

/// Whether a session is still worth using at `now`.
#[must_use]
pub const fn session_valid(session: &Session, now: pasejo_protocol::Millis) -> bool {
    session.expires > now
}
