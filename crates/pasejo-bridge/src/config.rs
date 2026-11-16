// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The bridge's configuration file.

use std::collections::BTreeSet;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use pasejo_protocol::api::valid_tenant;
use pasejo_protocol::encoding::unb64;
use pasejo_protocol::{Hash, Id};
use serde::Deserialize;

fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 8080))
}

const fn default_timeout() -> u64 {
    10
}

fn default_label() -> String {
    "pasejo-bridge".to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address the API listens on.
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,
    /// The pasejo server's base URL.
    pub server: String,
    /// The organization whose stores the bridge serves.
    pub org: String,
    /// The organization's genesis hash, base64url, as `pasejo server status`
    /// shows it after `pasejo org join`.
    pub genesis: String,
    /// The device file written by `pasejo-bridge keygen`.
    pub device_file: PathBuf,
    /// The workload token the bridge signs in with, read again whenever the
    /// session expires: a projected service-account token with the
    /// audience `pasejo-server`.
    pub workload_token_file: PathBuf,
    /// The device label shown to approvers.
    #[serde(default = "default_label")]
    pub label: String,
    /// Seconds before a request to the server gives up.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// The bearer tokens callers present, each mapped to the tenant label
    /// the server records with its reads.
    #[serde(default)]
    pub credentials: Vec<Credential>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    pub tenant: String,
    /// Lowercase hex SHA-256 of the bearer token, as `pasejo-bridge
    /// credential` prints it.
    pub token_sha256: String,
}

/// A checked configuration.
pub struct Settings {
    pub config: Config,
    pub org: Id,
    pub genesis: Hash,
    /// `(token hash, tenant)` pairs.
    pub credentials: Vec<([u8; 32], String)>,
}

impl Settings {
    /// Reads and checks the configuration at `path`.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or does not check.
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("cannot read configuration {}", path.display()))?;
        Self::parse(&text)
    }

    /// Parses and checks a configuration.
    ///
    /// # Errors
    ///
    /// Fails for malformed TOML, identifiers, hashes, tenant labels or token
    /// hashes, for repeated tenants or tokens, and when no credential is
    /// configured.
    pub fn parse(text: &str) -> Result<Self> {
        let config: Config = toml::from_str(text).context("malformed configuration")?;
        let org = Id::from_str(&config.org).context("malformed organization identifier")?;
        let genesis: [u8; 32] = unb64(&config.genesis)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .context("malformed genesis hash")?;
        if config.credentials.is_empty() {
            anyhow::bail!("the configuration names no credentials, so no caller could read");
        }
        let mut tenants = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        let mut credentials = Vec::with_capacity(config.credentials.len());
        for credential in &config.credentials {
            if !valid_tenant(&credential.tenant) {
                anyhow::bail!("malformed tenant label '{}'", credential.tenant);
            }
            let hash = parse_hex(&credential.token_sha256).with_context(|| {
                format!("malformed token hash for tenant '{}'", credential.tenant)
            })?;
            if !tenants.insert(credential.tenant.clone()) {
                anyhow::bail!("tenant '{}' appears twice", credential.tenant);
            }
            if !hashes.insert(hash) {
                anyhow::bail!("two tenants share one token");
            }
            credentials.push((hash, credential.tenant.clone()));
        }
        Ok(Self {
            config,
            org,
            genesis: Hash(genesis),
            credentials,
        })
    }
}

fn parse_hex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    text.bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        .then_some(out)
}

/// Lowercase hex.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn config(credentials: &str) -> String {
        format!(
            "server = \"https://pasejo.example\"\norg = \"{}\"\ngenesis = \"{}\"\ndevice_file = \"/etc/pasejo/device.json\"\nworkload_token_file = \"/var/run/token\"\n{credentials}",
            Id::random(),
            pasejo_protocol::encoding::b64(&[7; 32]),
        )
    }

    fn credential(tenant: &str, hash: &str) -> String {
        format!("[[credentials]]\ntenant = \"{tenant}\"\ntoken_sha256 = \"{hash}\"\n")
    }

    #[test]
    fn a_complete_configuration_takes_defaults() {
        let hash = hex(&[1; 32]);
        let settings = Settings::parse(&config(&credential("team-a", &hash))).unwrap();
        assert_eq!(settings.config.listen, default_listen());
        assert_eq!(settings.config.timeout_seconds, 10);
        assert_eq!(settings.config.label, "pasejo-bridge");
        assert_eq!(settings.genesis, Hash([7; 32]));
        assert_eq!(settings.credentials, vec![([1; 32], "team-a".to_owned())]);
    }

    #[test]
    fn malformed_configurations_are_refused() {
        let good = hex(&[1; 32]);
        let other = hex(&[2; 32]);
        for (case, text) in [
            ("no credentials", config("")),
            ("bad tenant", config(&credential("team a", &good))),
            ("empty tenant", config(&credential("", &good))),
            ("short hash", config(&credential("a", "abcd"))),
            (
                "upper-case hash",
                config(&credential("a", &good.to_uppercase().replace('1', "A"))),
            ),
            ("non-hex hash", config(&credential("a", &"zz".repeat(32)))),
            (
                "repeated tenant",
                config(&(credential("a", &good) + &credential("a", &other))),
            ),
            (
                "shared token",
                config(&(credential("a", &good) + &credential("b", &good))),
            ),
            (
                "unknown key",
                config(&(credential("a", &good) + "colour = \"red\"\n")),
            ),
            (
                "bad org",
                config(&credential("a", &good)).replace("org = \"", "org = \"x"),
            ),
            (
                "bad genesis",
                config(&credential("a", &good)).replace("genesis = \"", "genesis = \"!"),
            ),
        ] {
            assert!(Settings::parse(&text).is_err(), "{case}");
        }
    }

    #[test]
    fn hex_round_trips() {
        let bytes: [u8; 32] = std::array::from_fn(|index| u8::try_from(index * 7).unwrap());
        assert_eq!(parse_hex(&hex(&bytes)), Some(bytes));
    }
}
