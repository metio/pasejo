// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;

use crate::cli::environment_variables;

pub fn download_public_key(username: &str) -> anyhow::Result<String> {
    let codeberg_host = env::var(environment_variables::CODEBERG_HOST)
        .unwrap_or_else(|_| String::from("codeberg.org"));
    let key = ureq::get(format!("https://{codeberg_host}/{username}.keys"))
        .call()?
        .body_mut()
        .read_to_string()?;
    Ok(String::from(key.trim()))
}
