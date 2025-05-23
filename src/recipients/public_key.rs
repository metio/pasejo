// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;

use crate::downloader::{codeberg, github, gitlab};
use crate::models::cli::RecipientKeysArgs;

pub fn get(args: &RecipientKeysArgs) -> anyhow::Result<Vec<(String, String)>> {
    if let Some(public_key) = &args.public_key {
        Ok(vec![split_ssh_key(public_key)?])
    } else if let Some(codeberg_username) = &args.codeberg {
        Ok(vec![split_ssh_key(&codeberg::download_public_key(
            codeberg_username,
        )?)?])
    } else if let Some(github_username) = &args.github {
        Ok(vec![split_ssh_key(&github::download_public_key(
            github_username,
        )?)?])
    } else if let Some(gitlab_username) = &args.gitlab {
        Ok(vec![split_ssh_key(&gitlab::download_public_key(
            gitlab_username,
        )?)?])
    } else if let Some(filename) = &args.file {
        let mut public_keys = vec![];
        let file_content = fs::read_to_string(filename)?;
        let mut comment: String = String::new();
        for line in file_content.lines() {
            if line.starts_with('#') {
                let replacement = if comment.is_empty() { "" } else { " " };
                comment.push_str(line.replace('#', replacement).trim());
            } else {
                let split = split_ssh_key(line)?;

                if comment.is_empty() {
                    public_keys.push(split);
                } else {
                    public_keys.push((split.0, comment.clone()));
                }

                comment.clear();
            }
        }
        if public_keys.is_empty() {
            anyhow::bail!("No public key found in '{filename}'")
        }
        Ok(public_keys)
    } else {
        anyhow::bail!("You must specify at least one source for a public key")
    }
}

fn split_ssh_key(key: &str) -> anyhow::Result<(String, String)> {
    let (key, comment) = if key.starts_with("ssh") {
        let parts: Vec<&str> = key.split_whitespace().collect();
        if parts.len() < 2 {
            anyhow::bail!("Invalid SSH public key format")
        }
        if parts.len() > 2 {
            (parts[0..2].join(" "), parts[2..].join(" "))
        } else {
            (key.to_owned(), String::new())
        }
    } else {
        (key.to_owned(), String::new())
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
        assert_eq!(String::from("public key"), public_key.unwrap()[0].0);
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
        assert_eq!(String::from("public key"), public_key.unwrap()[0].0);
    }
}
