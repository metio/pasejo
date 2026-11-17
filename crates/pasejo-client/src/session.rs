// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Signing in: the device authorization grant for people, the system-browser
//! sign-in for apps, workload tokens for machines and the emergency window,
//! plus the server's description.

use pasejo_protocol::api::{
    DeviceAuthorization, DeviceAuthorizationRequest, ErrorCode, Me, ServerInfo, TokenRequest,
    TokenResponse, purpose,
};
use pasejo_protocol::encoding::{Fixed, b64, random, sha256};
use pasejo_protocol::envelope::{Context, Envelope, TokenClaim};
use zeroize::Zeroizing;

use crate::error::{Error, Result};
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

/// An app sign-in in progress: the URL the app opens in the system browser,
/// and the PKCE verifier and state the redirect is checked against.
pub struct AppLogin {
    pub url: String,
    pub verifier: Zeroizing<String>,
    pub state: String,
}

/// Percent-encodes `value` for a URL query, leaving RFC 3986's unreserved
/// characters as they are.
fn query_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// The `code` of a redirect that carries `state`.
fn redirect_code(redirect: &str, state: &str) -> Result<String> {
    let query = redirect
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or_else(|| Error::verification("the sign-in redirect carries no code"))?;
    let value = |name: &str| {
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    };
    if value("state") != Some(state) {
        return Err(Error::verification(
            "the sign-in redirect belongs to another sign-in",
        ));
    }
    value("code")
        .filter(|code| !code.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| Error::verification("the sign-in redirect carries no code"))
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
                signing_key: Fixed(self.device.signer.verifying_key()),
                time: self.now(),
            },
            self.device.id,
            &*self.device.signer,
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
                signing_key: Fixed(self.device.signer.verifying_key()),
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
        let request = TokenRequest {
            claim,
            jwt: None,
            verifier: None,
        };
        match self.anonymous::<_, TokenResponse>("POST", "/v1/auth/token", Some(&request)) {
            Ok(response) => Ok(Poll::Done(self.adopt(response))),
            Err(error) if error.is(ErrorCode::AuthorizationPending) => Ok(Poll::Pending),
            Err(error) if error.is(ErrorCode::SlowDown) => Ok(Poll::SlowDown),
            Err(error) => Err(error),
        }
    }

    /// Starts an app sign-in that returns to `redirect_uri`, one of the
    /// server's `[apps] redirect_uris`. The app opens `url` in the system
    /// browser and passes the redirect it receives to
    /// [`Client::finish_app_login`].
    #[must_use]
    pub fn start_app_login(&self, redirect_uri: &str) -> AppLogin {
        let verifier = Zeroizing::new(b64(&random::<32>()));
        let state = b64(&random::<16>());
        let challenge = b64(&sha256(verifier.as_bytes()));
        let url = format!(
            "{}/auth/app/start?challenge={challenge}&redirect_uri={}&state={state}",
            self.base_url(),
            query_component(redirect_uri),
        );
        AppLogin {
            url,
            verifier,
            state,
        }
    }

    /// Exchanges the redirect of an app sign-in for a session bound to this
    /// device's key. On success the client keeps the session.
    ///
    /// # Errors
    ///
    /// Fails when the redirect belongs to another sign-in or the server
    /// refuses the code.
    pub fn finish_app_login(&mut self, login: &AppLogin, redirect: &str) -> Result<Session> {
        let code = redirect_code(redirect, &login.state)?;
        let claim = self.claim(purpose::APP, code)?;
        let request = TokenRequest {
            claim,
            jwt: None,
            verifier: Some(login.verifier.to_string()),
        };
        let response = self.anonymous("POST", "/v1/auth/app", Some(&request))?;
        Ok(self.adopt(response))
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
            verifier: None,
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
        let request = TokenRequest {
            claim,
            jwt: None,
            verifier: None,
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_components_encode_everything_but_unreserved_characters() {
        assert_eq!(
            query_component("wtf.metio.pasejo:/auth"),
            "wtf.metio.pasejo%3A%2Fauth"
        );
        assert_eq!(query_component("a-b_c.d~e"), "a-b_c.d~e");
        assert_eq!(query_component("ü &="), "%C3%BC%20%26%3D");
    }

    #[test]
    fn a_redirect_yields_its_code_only_for_the_matching_state() {
        let redirect = "wtf.metio.pasejo:/auth?code=abc&state=xyz";
        assert_eq!(redirect_code(redirect, "xyz").ok().as_deref(), Some("abc"));
        assert!(redirect_code(redirect, "other").is_err());
        assert!(redirect_code("wtf.metio.pasejo:/auth?state=xyz", "xyz").is_err());
        assert!(redirect_code("wtf.metio.pasejo:/auth?code=&state=xyz", "xyz").is_err());
        assert!(redirect_code("wtf.metio.pasejo:/auth", "xyz").is_err());
        assert!(redirect_code("wtf.metio.pasejo:/auth?code=abc", "xyz").is_err());
    }
}
