// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::Deserialize;
use url::Url;

use crate::net::Network;

/// The server's configuration file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,
    pub public_url: Url,
    pub database_url: String,
    pub master_key_file: PathBuf,
    #[serde(default)]
    pub tls: Option<Tls>,
    #[serde(default)]
    pub oidc: Option<Oidc>,
    #[serde(default)]
    pub workload: Vec<WorkloadIssuer>,
    #[serde(default)]
    pub break_glass: BreakGlass,
    #[serde(default)]
    pub audit: Audit,
    #[serde(default)]
    pub orgs: Orgs,
    #[serde(default)]
    pub trusted_proxies: Vec<Network>,
    /// Where `/metrics` is served. Unset, metrics are not served at all;
    /// the main listener never serves them.
    #[serde(default)]
    pub metrics_listen: Option<SocketAddr>,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub apps: Apps,
}

fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 8080))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tls {
    pub certificate: PathBuf,
    pub key: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Oidc {
    pub issuer: String,
    pub client_id: String,
    pub client_secret_file: Option<PathBuf>,
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

fn default_scopes() -> Vec<String> {
    ["openid", "email", "profile"].map(String::from).to_vec()
}

/// A workload token issuer and the rules mapping its subjects to machines.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadIssuer {
    pub issuer: String,
    #[serde(default = "default_audience")]
    pub audience: String,
    pub jwks_url: Option<Url>,
    pub jwks_file: Option<PathBuf>,
    pub rules: Vec<WorkloadRule>,
}

fn default_audience() -> String {
    "pasejo-server".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRule {
    /// An exact subject, or a prefix ending in `*`.
    pub subject: String,
    /// The machine user the subject signs in as.
    pub machine: String,
}

impl WorkloadRule {
    #[must_use]
    pub fn matches(&self, subject: &str) -> bool {
        self.subject
            .strip_suffix('*')
            .map_or(self.subject == subject, |prefix| {
                subject.starts_with(prefix)
            })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BreakGlass {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_break_glass_networks")]
    pub networks: Vec<Network>,
}

impl Default for BreakGlass {
    fn default() -> Self {
        Self {
            enabled: false,
            networks: default_break_glass_networks(),
        }
    }
}

fn default_break_glass_networks() -> Vec<Network> {
    ["127.0.0.0/8", "::1/128"]
        .iter()
        .filter_map(|network| network.parse().ok())
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audit {
    #[serde(default = "default_checkpoint_every")]
    pub checkpoint_every: u64,
    #[serde(default = "default_checkpoint_interval")]
    pub checkpoint_interval_seconds: u64,
    pub retention_days: Option<u64>,
    #[serde(default)]
    pub stdout: bool,
    pub webhook: Option<Webhook>,
}

impl Default for Audit {
    fn default() -> Self {
        Self {
            checkpoint_every: default_checkpoint_every(),
            checkpoint_interval_seconds: default_checkpoint_interval(),
            retention_days: None,
            stdout: false,
            webhook: None,
        }
    }
}

const fn default_checkpoint_every() -> u64 {
    1_000
}

const fn default_checkpoint_interval() -> u64 {
    3_600
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Webhook {
    pub url: Url,
    pub secret_file: PathBuf,
    #[serde(default = "default_batch")]
    pub batch: usize,
}

const fn default_batch() -> usize {
    100
}

/// Who may create organizations.
#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Creation {
    #[default]
    Anyone,
    Nobody,
    Listed,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Orgs {
    #[serde(default)]
    pub creation: Creation,
    /// Email addresses allowed to create organizations under `listed`.
    #[serde(default)]
    pub creators: Vec<String>,
}

/// Native apps signing in through the system browser.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apps {
    /// Redirect URIs an app sign-in may return to; the private-use scheme of
    /// the pasejo apps by default.
    #[serde(default = "default_app_redirect_uris")]
    pub redirect_uris: Vec<String>,
}

impl Default for Apps {
    fn default() -> Self {
        Self {
            redirect_uris: default_app_redirect_uris(),
        }
    }
}

fn default_app_redirect_uris() -> Vec<String> {
    vec!["wtf.metio.pasejo:/auth".into()]
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(default = "default_auth_per_minute")]
    pub auth_per_minute: u32,
    #[serde(default = "default_body")]
    pub body_bytes: usize,
    #[serde(default = "default_commit_body")]
    pub commit_body_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            auth_per_minute: default_auth_per_minute(),
            body_bytes: default_body(),
            commit_body_bytes: default_commit_body(),
        }
    }
}

const fn default_auth_per_minute() -> u32 {
    30
}

const fn default_body() -> usize {
    1 << 20
}

const fn default_commit_body() -> usize {
    32 << 20
}

impl Config {
    /// Reads and checks a configuration file.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read, does not parse or is
    /// inconsistent.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        Self::parse(&text)
    }

    /// Parses and checks configuration text.
    ///
    /// # Errors
    ///
    /// Fails when the text does not parse or is inconsistent.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(text).context("invalid configuration")?;
        config.check()?;
        Ok(config)
    }

    fn check(&self) -> anyhow::Result<()> {
        if !matches!(self.public_url.scheme(), "http" | "https") {
            bail!("public_url must use http or https");
        }
        for issuer in &self.workload {
            if issuer.jwks_url.is_some() == issuer.jwks_file.is_some() {
                bail!(
                    "workload issuer {} needs exactly one of jwks_url and jwks_file",
                    issuer.issuer
                );
            }
            if issuer.rules.is_empty() {
                bail!("workload issuer {} has no rules", issuer.issuer);
            }
        }
        if self.audit.checkpoint_every == 0 {
            bail!("audit.checkpoint_every must be positive");
        }
        Ok(())
    }

    #[must_use]
    pub fn https(&self) -> bool {
        self.public_url.scheme() == "https"
    }

    /// An absolute URL below the public URL.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.public_url.as_str().trim_end_matches('/'), path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
public_url = "https://pasejo.example.com"
database_url = "sqlite://pasejo.db"
master_key_file = "master.key"
"#;

    #[test]
    fn a_minimal_configuration_gets_safe_defaults() {
        let config = Config::parse(MINIMAL).unwrap();
        assert!(!config.break_glass.enabled);
        assert!(config.break_glass.networks[0].contains("127.0.0.1".parse().unwrap()));
        assert_eq!(config.orgs.creation, Creation::Anyone);
        assert_eq!(config.limits.body_bytes, 1 << 20);
        assert_eq!(config.audit.checkpoint_every, 1_000);
        assert!(config.https());
        assert_eq!(
            config.url("/auth/verify"),
            "https://pasejo.example.com/auth/verify"
        );
    }

    #[test]
    fn unknown_fields_and_bad_values_are_refused() {
        assert!(Config::parse(&format!("{MINIMAL}\nbogus = 1")).is_err());
        assert!(
            Config::parse(
                r#"public_url = "ftp://x"
database_url = "sqlite://x"
master_key_file = "k""#
            )
            .is_err()
        );
        let both = format!(
            "{MINIMAL}\n[[workload]]\nissuer = \"i\"\njwks_url = \"https://i/jwks\"\njwks_file = \"f\"\nrules = [{{ subject = \"s\", machine = \"m\" }}]"
        );
        assert!(Config::parse(&both).is_err());
        let no_rules =
            format!("{MINIMAL}\n[[workload]]\nissuer = \"i\"\njwks_file = \"f\"\nrules = []");
        assert!(Config::parse(&no_rules).is_err());
    }

    #[test]
    fn workload_rules_match_exact_subjects_and_prefixes() {
        let exact = WorkloadRule {
            subject: "system:serviceaccount:ns:bridge".into(),
            machine: "bridge".into(),
        };
        assert!(exact.matches("system:serviceaccount:ns:bridge"));
        assert!(!exact.matches("system:serviceaccount:ns:bridge2"));
        let prefix = WorkloadRule {
            subject: "repo:metio/pasejo:*".into(),
            machine: "ci".into(),
        };
        assert!(prefix.matches("repo:metio/pasejo:ref:refs/heads/main"));
        assert!(!prefix.matches("repo:metio/other:ref:refs/heads/main"));
    }
}
