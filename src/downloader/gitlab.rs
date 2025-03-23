// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;

use crate::cli::environment_variables;

pub fn download_public_key(username: &str) -> anyhow::Result<String> {
    let gitlab_host =
        env::var(environment_variables::GITLAB_HOST).unwrap_or_else(|_| String::from("gitlab.com"));
    let key = reqwest::blocking::get(format!("https://{gitlab_host}/{username}.keys"))?.text()?;
    Ok(String::from(key.trim()))
}
