use std::env;

use crate::cli::environment_variables;

pub fn download_public_key(username: &str) -> anyhow::Result<String> {
    let github_host =
        env::var(environment_variables::GITHUB_HOST).unwrap_or_else(|_| String::from("github.com"));
    let key = reqwest::blocking::get(format!("https://{github_host}/{username}.keys"))?.text()?;
    Ok(String::from(key.trim()))
}
