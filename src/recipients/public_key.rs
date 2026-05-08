// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;

use crate::downloader::{Provider, download_public_key};
use crate::models::cli::RecipientKeysArgs;

pub fn get(args: &RecipientKeysArgs) -> anyhow::Result<Vec<(String, String)>> {
    if let Some(public_key) = &args.public_key {
        Ok(vec![split_ssh_key(public_key)?])
    } else if let Some(codeberg_username) = &args.codeberg {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Codeberg,
            codeberg_username,
        )?)?])
    } else if let Some(github_username) = &args.github {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Github,
            github_username,
        )?)?])
    } else if let Some(gitlab_username) = &args.gitlab {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Gitlab,
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
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

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

    #[test]
    fn no_source_returns_error() {
        let args = RecipientKeysArgs {
            public_key: None,
            file: None,
            codeberg: None,
            github: None,
            gitlab: None,
        };
        assert!(get(&args).is_err());
    }

    #[test]
    fn split_ssh_key_separates_key_from_comment() {
        let (key, comment) =
            split_ssh_key("ssh-ed25519 AAAA alice@example.com").unwrap();
        assert_eq!(key, "ssh-ed25519 AAAA");
        assert_eq!(comment, "alice@example.com");
    }

    #[test]
    fn split_ssh_key_joins_multi_word_comment() {
        let (key, comment) =
            split_ssh_key("ssh-ed25519 AAAA alice example user").unwrap();
        assert_eq!(key, "ssh-ed25519 AAAA");
        assert_eq!(comment, "alice example user");
    }

    #[test]
    fn split_ssh_key_without_comment_yields_empty_comment() {
        let (key, comment) = split_ssh_key("ssh-ed25519 AAAA").unwrap();
        assert_eq!(key, "ssh-ed25519 AAAA");
        assert_eq!(comment, "");
    }

    #[test]
    fn split_ssh_key_rejects_truncated_ssh_line() {
        assert!(split_ssh_key("ssh-ed25519").is_err());
    }

    #[test]
    fn split_ssh_key_passes_age_keys_through_unchanged() {
        let (key, comment) = split_ssh_key("age1xyz").unwrap();
        assert_eq!(key, "age1xyz");
        assert_eq!(comment, "");
    }

    #[test]
    fn public_key_from_file_with_age_key() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("recipients.txt");
        file.write_str("age1abc\nage1def\n").unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        let result = get(&args).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "age1abc");
        assert_eq!(result[0].1, "");
        assert_eq!(result[1].0, "age1def");
    }

    #[test]
    fn public_key_from_file_attaches_preceding_comments() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("recipients.txt");
        file.write_str("# alice\nage1abc\n# bob\nage1def\n")
            .unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        let result = get(&args).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], (String::from("age1abc"), String::from("alice")));
        assert_eq!(result[1], (String::from("age1def"), String::from("bob")));
    }

    #[test]
    fn public_key_from_empty_file_returns_error() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("empty.txt");
        file.write_str("").unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        assert!(get(&args).is_err());
    }

    #[test]
    fn public_key_from_missing_file_returns_error() {
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(String::from("/nonexistent/path/to/file")),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        assert!(get(&args).is_err());
    }
}
