// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::time::Duration;

use anyhow::Context;

use crate::cli::{environment_variables, i18n};
use crate::downloader::username;

// `.keys` files hold a handful of public keys (a few KB at most). Cap the
// response so a hostile or compromised provider can't exhaust memory by
// streaming an arbitrarily large body — ureq's default limit is `u64::MAX`.
const MAX_RESPONSE_BYTES: u64 = 1 << 20;

#[derive(Clone, Copy)]
pub enum Provider {
    Codeberg,
    Github,
    Gitlab,
}

impl Provider {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Codeberg => "codeberg",
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }

    const fn host_env(self) -> &'static str {
        match self {
            Self::Codeberg => environment_variables::CODEBERG_HOST,
            Self::Github => environment_variables::GITHUB_HOST,
            Self::Gitlab => environment_variables::GITLAB_HOST,
        }
    }

    const fn default_host(self) -> &'static str {
        match self {
            Self::Codeberg => "codeberg.org",
            Self::Github => "github.com",
            Self::Gitlab => "gitlab.com",
        }
    }
}

pub fn download_public_key(
    provider: Provider,
    username: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let username = username::validate(username)?;
    let host =
        env::var(provider.host_env()).unwrap_or_else(|_| String::from(provider.default_host()));
    // The override may carry its own scheme so a self-hosted instance can
    // be reached over plain HTTP or a non-standard port (and so tests can
    // point at a local fixture). A bare host keeps the secure default.
    let url = if host.contains("://") {
        format!("{host}/{username}.keys")
    } else {
        format!("https://{host}/{username}.keys")
    };
    // ureq has no default timeout — without one, an unhealthy provider
    // (e.g. codeberg.org returning 502 after several minutes) hangs the
    // whole CLI. The cap is callable-controlled so users can tune it via
    // the `key-download-timeout-seconds` config option.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();
    let mut response = match agent.get(url).call() {
        Ok(response) => response,
        Err(error) => return Err(classify_download_error(error, provider, username)),
    };
    let key = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_string()
        .with_context(|| i18n::error_downloading_public_key_failed(provider.name()))?;
    Ok(String::from(key.trim()))
}

/// Turn a transport-level failure into a localized, actionable error.
/// `ureq` is left with its default `http_status_as_error`, so a non-2xx
/// arrives as `StatusCode`: a `404` means the account does not exist, a
/// `429` is rate limiting, any other status is an upstream fault. A
/// `Timeout` is the global cap firing on an unresponsive provider.
/// Everything else (DNS, TLS, connection reset) collapses to the generic
/// "failed" message with the underlying error kept in the chain.
fn classify_download_error(
    error: ureq::Error,
    provider: Provider,
    username: &str,
) -> anyhow::Error {
    let name = provider.name();
    match error {
        ureq::Error::StatusCode(404) => {
            anyhow::anyhow!(i18n::error_download_user_not_found(name, username))
        }
        ureq::Error::StatusCode(429) => {
            anyhow::anyhow!(i18n::error_download_rate_limited(name))
        }
        ureq::Error::StatusCode(status) => {
            anyhow::anyhow!(i18n::error_download_server_error(name, &status.to_string()))
        }
        ureq::Error::Timeout(_) => anyhow::anyhow!(i18n::error_download_timeout(name)),
        // DNS, TLS, connection reset, protocol errors: keep the underlying
        // ureq error in the chain so `{:#}` can surface the detail.
        other => anyhow::Error::new(other).context(i18n::error_downloading_public_key_failed(name)),
    }
}
