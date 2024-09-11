use std::fs;

use crate::downloader::{codeberg, github, gitlab};
use crate::models::cli::RecipientKeysArgs;

pub fn get(args: &RecipientKeysArgs) -> anyhow::Result<String> {
    if let Some(public_key) = &args.public_key {
        Ok(public_key.clone())
    } else if let Some(codeberg_username) = &args.codeberg {
        codeberg::download_public_key(codeberg_username)
    } else if let Some(github_username) = &args.github {
        github::download_public_key(github_username)
    } else if let Some(gitlab_username) = &args.gitlab {
        gitlab::download_public_key(gitlab_username)
    } else if let Some(filename) = &args.file {
        let file_content = fs::read_to_string(filename)?;
        for line in file_content.lines() {
            if !line.starts_with('#') {
                return Ok(String::from(line));
            }
        }
        anyhow::bail!("No public key found in '{filename}'")
    } else {
        anyhow::bail!("You must specify at least source for a public key")
    }
}
