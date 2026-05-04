// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::environment_variables;
use crate::downloader::username;
use anyhow::Context;
use std::env;

pub fn download_public_key(username: &str) -> anyhow::Result<String> {
    let username = username::validate(username)?;
    let github_host =
        env::var(environment_variables::GITHUB_HOST).unwrap_or_else(|_| String::from("github.com"));
    let key = ureq::get(format!("https://{github_host}/{username}.keys"))
        .call()
        .context("Downloading public key from github failed")?
        .body_mut()
        .read_to_string()?;
    Ok(String::from(key.trim()))
}
