// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! JSON Web Token verification against a JSON Web Key Set, shared by OIDC
//! sign-in and workload sign-in.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use pasejo_protocol::Millis;
use serde_json::Value;
use tokio::sync::RwLock;
use url::Url;

/// Algorithms accepted for signed tokens. Symmetric algorithms are never
/// accepted, so a public key can never double as an HMAC secret.
const ASYMMETRIC: &[Algorithm] = &[
    Algorithm::RS256,
    Algorithm::RS384,
    Algorithm::RS512,
    Algorithm::PS256,
    Algorithm::PS384,
    Algorithm::PS512,
    Algorithm::ES256,
    Algorithm::ES384,
    Algorithm::EdDSA,
];

/// A key set fetched from a URL or read from a file, refetched at most once
/// a minute when a token names an unknown key.
pub struct JwksCache {
    url: Option<Url>,
    file: Option<PathBuf>,
    http: reqwest::Client,
    cached: RwLock<Option<(JwkSet, std::time::Instant)>>,
}

impl JwksCache {
    #[must_use]
    pub fn new(url: Option<Url>, file: Option<PathBuf>, http: reqwest::Client) -> Self {
        Self {
            url,
            file,
            http,
            cached: RwLock::new(None),
        }
    }

    pub async fn set_url(&mut self, url: Url) {
        self.url = Some(url);
        *self.cached.write().await = None;
    }

    async fn fetch(&self) -> anyhow::Result<JwkSet> {
        if let Some(file) = &self.file {
            let text = tokio::fs::read_to_string(file)
                .await
                .with_context(|| format!("cannot read {}", file.display()))?;
            return Ok(serde_json::from_str(&text)?);
        }
        let url = self
            .url
            .clone()
            .ok_or_else(|| anyhow!("no key set location"))?;
        Ok(self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn keys(&self, refresh: bool) -> anyhow::Result<JwkSet> {
        {
            let cached = self.cached.read().await;
            if let Some((set, fetched)) = cached.as_ref() {
                let fresh = fetched.elapsed() < Duration::from_secs(3_600);
                let recent = fetched.elapsed() < Duration::from_secs(60);
                if (fresh && !refresh) || recent {
                    return Ok(set.clone());
                }
            }
        }
        let set = self.fetch().await?;
        *self.cached.write().await = Some((set.clone(), std::time::Instant::now()));
        Ok(set)
    }

    /// Verifies `token`'s signature, issuer, audience and expiry and returns
    /// its claims.
    ///
    /// # Errors
    ///
    /// Fails for any token that does not verify.
    pub async fn verify(
        &self,
        token: &str,
        issuer: &str,
        audience: &str,
        now: Millis,
    ) -> anyhow::Result<Value> {
        let header = decode_header(token).context("malformed token")?;
        if !ASYMMETRIC.contains(&header.alg) {
            bail!("token algorithm {:?} is not accepted", header.alg);
        }
        let mut set = self.keys(false).await?;
        let mut jwk = find(&set, header.kid.as_deref());
        if jwk.is_none() {
            set = self.keys(true).await?;
            jwk = find(&set, header.kid.as_deref());
        }
        let jwk = jwk.ok_or_else(|| anyhow!("token names an unknown key"))?;
        if let Some(algorithm) = jwk.common.key_algorithm
            && format!("{algorithm:?}") != format!("{:?}", header.alg)
        {
            bail!("token algorithm does not match its key");
        }
        let key = DecodingKey::from_jwk(&jwk).context("unusable key")?;
        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        // The library checks expiry against the system clock; checking our
        // own clock as well keeps tests with a moved clock honest.
        validation.leeway = 60;
        let data = decode::<Value>(token, &key, &validation).context("token does not verify")?;
        let expires = data.claims.get("exp").and_then(Value::as_u64).unwrap_or(0);
        if expires.saturating_mul(1_000) + 60_000 < now {
            bail!("token expired");
        }
        Ok(data.claims)
    }
}

fn find(set: &JwkSet, kid: Option<&str>) -> Option<jsonwebtoken::jwk::Jwk> {
    match kid {
        Some(kid) => set.find(kid).cloned(),
        None if set.keys.len() == 1 => set.keys.first().cloned(),
        None => None,
    }
}
