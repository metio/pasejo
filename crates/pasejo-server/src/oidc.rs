// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `OpenID` Connect relying party: authorization code flow with PKCE,
//! `state` and `nonce`.

use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use pasejo_protocol::Millis;
use pasejo_protocol::encoding::{b64, random, sha256};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{OnceCell, RwLock};
use url::Url;
use zeroize::Zeroizing;

use crate::config::{Config, Oidc as OidcConfig};
use crate::jwt::JwksCache;

#[derive(Debug, Clone, Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: Url,
    token_endpoint: Url,
    jwks_uri: Url,
}

/// The identity an ID token asserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub issuer: String,
    pub subject: String,
    pub email: String,
    pub name: String,
}

/// What a sign-in flow keeps between the redirect and the callback.
#[derive(Debug, Clone)]
pub struct Flow {
    pub state: String,
    pub nonce: String,
    pub verifier: String,
    pub url: String,
}

pub struct Oidc {
    issuer: String,
    client_id: String,
    client_secret: Option<Zeroizing<String>>,
    scopes: Vec<String>,
    redirect_uri: String,
    http: reqwest::Client,
    discovery: OnceCell<Discovery>,
    jwks: RwLock<Option<Arc<JwksCache>>>,
}

impl Oidc {
    /// Prepares the relying party; discovery happens on first use.
    ///
    /// # Errors
    ///
    /// Fails when the client secret file cannot be read.
    pub fn new(
        config: &OidcConfig,
        server: &Config,
        http: reqwest::Client,
    ) -> anyhow::Result<Self> {
        let client_secret = config
            .client_secret_file
            .as_ref()
            .map(|path| {
                std::fs::read_to_string(path)
                    .map(|secret| Zeroizing::new(secret.trim().to_owned()))
                    .with_context(|| format!("cannot read {}", path.display()))
            })
            .transpose()?;
        Ok(Self {
            issuer: config.issuer.trim_end_matches('/').to_owned(),
            client_id: config.client_id.clone(),
            client_secret,
            scopes: config.scopes.clone(),
            redirect_uri: server.url("/auth/callback"),
            http,
            discovery: OnceCell::new(),
            jwks: RwLock::new(None),
        })
    }

    async fn discovery(&self) -> anyhow::Result<&Discovery> {
        self.discovery
            .get_or_try_init(|| async {
                let url = format!("{}/.well-known/openid-configuration", self.issuer);
                let discovery: Discovery = self
                    .http
                    .get(&url)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .context("malformed discovery document")?;
                if discovery.issuer.trim_end_matches('/') != self.issuer {
                    bail!("discovery document names issuer {}", discovery.issuer);
                }
                *self.jwks.write().await = Some(Arc::new(JwksCache::new(
                    Some(discovery.jwks_uri.clone()),
                    None,
                    self.http.clone(),
                )));
                Ok(discovery)
            })
            .await
    }

    /// Starts a sign-in: fresh `state`, `nonce` and PKCE verifier, and the
    /// provider URL to redirect to.
    ///
    /// # Errors
    ///
    /// Fails when discovery fails.
    pub async fn start(&self) -> anyhow::Result<Flow> {
        let discovery = self.discovery().await?;
        let state = b64(&random::<24>());
        let nonce = b64(&random::<24>());
        let verifier = b64(&random::<32>());
        let challenge = b64(&sha256(verifier.as_bytes()));
        let mut url = discovery.authorization_endpoint.clone();
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("state", &state)
            .append_pair("nonce", &nonce)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(Flow {
            state,
            nonce,
            verifier,
            url: url.into(),
        })
    }

    /// Exchanges an authorization code and verifies the ID token.
    ///
    /// # Errors
    ///
    /// Fails when the exchange fails or the ID token does not verify.
    pub async fn finish(
        &self,
        code: &str,
        verifier: &str,
        nonce: &str,
        now: Millis,
    ) -> anyhow::Result<Identity> {
        let discovery = self.discovery().await?;
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("client_id", self.client_id.as_str()),
            ("code_verifier", verifier),
        ];
        if let Some(secret) = &self.client_secret {
            form.push(("client_secret", secret.as_str()));
        }
        let response: Value = self
            .http
            .post(discovery.token_endpoint.clone())
            .form(&form)
            .send()
            .await?
            .error_for_status()
            .context("token endpoint refused the code")?
            .json()
            .await?;
        let id_token = response
            .get("id_token")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("token response carries no ID token"))?;
        let jwks = self
            .jwks
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("no key set"))?;
        let claims = jwks
            .verify(id_token, &discovery.issuer, &self.client_id, now)
            .await?;
        if claims.get("nonce").and_then(Value::as_str) != Some(nonce) {
            bail!("ID token nonce does not match");
        }
        identity(&discovery.issuer, &claims)
    }
}

fn identity(issuer: &str, claims: &Value) -> anyhow::Result<Identity> {
    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .filter(|subject| !subject.is_empty())
        .ok_or_else(|| anyhow!("ID token has no subject"))?;
    let email = claims
        .get("email")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let verified = claims
        .get("email_verified")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let name = claims
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| claims.get("preferred_username").and_then(Value::as_str))
        .unwrap_or(email);
    Ok(Identity {
        issuer: issuer.to_owned(),
        subject: subject.to_owned(),
        email: if verified {
            email.to_owned()
        } else {
            String::new()
        },
        name: name.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn identities_drop_unverified_emails() {
        let verified = identity("i", &json!({"sub": "s", "email": "a@x", "name": "A"})).unwrap();
        assert_eq!(verified.email, "a@x");
        assert_eq!(verified.name, "A");
        let unverified = identity(
            "i",
            &json!({"sub": "s", "email": "a@x", "email_verified": false}),
        )
        .unwrap();
        assert_eq!(unverified.email, "");
        assert!(identity("i", &json!({"sub": ""})).is_err());
        let fallback = identity("i", &json!({"sub": "s", "preferred_username": "alice"})).unwrap();
        assert_eq!(fallback.name, "alice");
    }
}
