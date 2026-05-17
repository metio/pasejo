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
    const fn name(self) -> &'static str {
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
    // ureq has no default timeout — without one, an unhealthy provider
    // (e.g. codeberg.org returning 502 after several minutes) hangs the
    // whole CLI. The cap is callable-controlled so users can tune it via
    // the `key-download-timeout-seconds` config option.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();
    let key = agent
        .get(format!("https://{host}/{username}.keys"))
        .call()
        .with_context(|| i18n::error_downloading_public_key_failed(provider.name()))?
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_string()?;
    Ok(String::from(key.trim()))
}
