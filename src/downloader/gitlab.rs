// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::environment_variables;
use crate::downloader::username;
use anyhow::Context;
use std::env;

pub fn download_public_key(username: &str) -> anyhow::Result<String> {
    let username = username::validate(username)?;
    let gitlab_host =
        env::var(environment_variables::GITLAB_HOST).unwrap_or_else(|_| String::from("gitlab.com"));
    let key = ureq::get(format!("https://{gitlab_host}/{username}.keys"))
        .call()
        .context("Downloading public key from gitlab failed")?
        .body_mut()
        .read_to_string()?;
    Ok(String::from(key.trim()))
}
