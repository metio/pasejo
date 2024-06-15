// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;

use crate::downloader::{codeberg, github, gitlab};
use crate::models::cli::RecipientKeysArgs;

pub fn get(args: &RecipientKeysArgs) -> anyhow::Result<(String, String)> {
    let public_key = if let Some(public_key) = &args.public_key {
        Ok(public_key.clone())
    } else if let Some(codeberg_username) = &args.codeberg {
        codeberg::download_public_key(codeberg_username)
    } else if let Some(github_username) = &args.github {
        github::download_public_key(github_username)
    } else if let Some(gitlab_username) = &args.gitlab {
        gitlab::download_public_key(gitlab_username)
    } else if let Some(filename) = &args.file {
        let file_content = fs::read_to_string(filename)?;
        let mut public_key: String = String::new();
        for line in file_content.lines() {
            if !line.starts_with('#') {
                public_key = String::from(line);
                break;
            }
        }
        if public_key.is_empty() {
            anyhow::bail!("No public key found in '{filename}'")
        }
        Ok(public_key)
    } else {
        anyhow::bail!("You must specify at least one source for a public key")
    }?;

    let (key, comment) = if public_key.starts_with("ssh") {
        let parts: Vec<&str> = public_key.split_whitespace().collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid public key format")
        }
        if parts.len() > 2 {
            (parts[0..2].join(" "), parts[2..].join(" "))
        } else {
            (public_key.clone(), String::new())
        }
    } else {
        (public_key, String::new())
    };

    Ok((key, comment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_key_from_args() {
        let args = RecipientKeysArgs {
            public_key: Some(String::from("public key")),
            file: None,
            codeberg: None,
            github: None,
            gitlab: None,
        };
        let public_key = get(&args);
        assert_eq!(String::from("public key"), public_key.unwrap().0);
    }

    #[test]
    fn public_key_from_args_is_most_important() {
        let args = RecipientKeysArgs {
            public_key: Some(String::from("public key")),
            file: Some(String::from("some-file")),
            codeberg: Some(String::from("codeberg")),
            github: Some(String::from("github")),
            gitlab: Some(String::from("gitlab")),
        };
        let public_key = get(&args);
        assert_eq!(String::from("public key"), public_key.unwrap().0);
    }
}
