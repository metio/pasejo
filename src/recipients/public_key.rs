// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;
use std::time::Duration;

use crate::downloader::{Provider, download_public_key};
use crate::models::cli::RecipientKeysArgs;

pub fn get(
    args: &RecipientKeysArgs,
    download_timeout: Duration,
) -> anyhow::Result<Vec<(String, String)>> {
    if let Some(public_key) = &args.public_key {
        Ok(vec![split_ssh_key(public_key)?])
    } else if let Some(codeberg_username) = &args.codeberg {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Codeberg,
            codeberg_username,
            download_timeout,
        )?)?])
    } else if let Some(github_username) = &args.github {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Github,
            github_username,
            download_timeout,
        )?)?])
    } else if let Some(gitlab_username) = &args.gitlab {
        Ok(vec![split_ssh_key(&download_public_key(
            Provider::Gitlab,
            gitlab_username,
            download_timeout,
        )?)?])
    } else if let Some(filename) = &args.file {
        let mut public_keys = vec![];
        let file_content = fs::read_to_string(filename)?;
        let mut comment: String = String::new();
        for line in file_content.lines() {
            if line.starts_with('#') {
                append_comment_line(&mut comment, line);
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

fn append_comment_line(buffer: &mut String, line: &str) {
    let stripped = line.trim_start_matches('#').trim();
    if stripped.is_empty() {
        return;
    }
    if !buffer.is_empty() {
        buffer.push(' ');
    }
    buffer.push_str(stripped);
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
        let public_key = get(&args, Duration::from_secs(30));
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
        let public_key = get(&args, Duration::from_secs(30));
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
        assert!(get(&args, Duration::from_secs(30)).is_err());
    }

    #[test]
    fn split_ssh_key_separates_key_from_comment() {
        let (key, comment) = split_ssh_key("ssh-ed25519 AAAA alice@example.com").unwrap();
        assert_eq!(key, "ssh-ed25519 AAAA");
        assert_eq!(comment, "alice@example.com");
    }

    #[test]
    fn split_ssh_key_joins_multi_word_comment() {
        let (key, comment) = split_ssh_key("ssh-ed25519 AAAA alice example user").unwrap();
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

        let result = get(&args, Duration::from_secs(30)).unwrap();
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

        let result = get(&args, Duration::from_secs(30)).unwrap();
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

        assert!(get(&args, Duration::from_secs(30)).is_err());
    }

    #[test]
    fn public_key_from_file_preserves_inner_hash_characters() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("recipients.txt");
        file.write_str("# ticket #1234\nage1abc\n").unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        let result = get(&args, Duration::from_secs(30)).unwrap();
        assert_eq!(result[0].1, "ticket #1234");
    }

    #[test]
    fn public_key_from_file_joins_multi_line_comments_with_space() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("recipients.txt");
        file.write_str("# alice\n# admin\nage1abc\n").unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        let result = get(&args, Duration::from_secs(30)).unwrap();
        assert_eq!(result[0].1, "alice admin");
    }

    #[test]
    fn public_key_from_file_resets_comment_buffer_between_keys() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("recipients.txt");
        file.write_str("# alice\nage1abc\nage1def\n# bob\n# admin\nage1ghi\n")
            .unwrap();
        let args = RecipientKeysArgs {
            public_key: None,
            file: Some(file.path().to_string_lossy().into_owned()),
            codeberg: None,
            github: None,
            gitlab: None,
        };

        let result = get(&args, Duration::from_secs(30)).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (String::from("age1abc"), String::from("alice")));
        assert_eq!(result[1], (String::from("age1def"), String::new()));
        assert_eq!(
            result[2],
            (String::from("age1ghi"), String::from("bob admin"))
        );
    }

    #[test]
    fn append_comment_line_strips_leading_hash_and_whitespace() {
        let mut buf = String::new();
        append_comment_line(&mut buf, "# alice");
        assert_eq!(buf, "alice");
    }

    #[test]
    fn append_comment_line_preserves_inner_hash() {
        let mut buf = String::new();
        append_comment_line(&mut buf, "# ticket #1234");
        assert_eq!(buf, "ticket #1234");
    }

    #[test]
    fn append_comment_line_joins_with_single_space() {
        let mut buf = String::from("alice");
        append_comment_line(&mut buf, "# admin");
        assert_eq!(buf, "alice admin");
    }

    #[test]
    fn append_comment_line_handles_no_space_after_hash() {
        let mut buf = String::new();
        append_comment_line(&mut buf, "#alice");
        assert_eq!(buf, "alice");
    }

    #[test]
    fn append_comment_line_strips_multiple_leading_hashes() {
        let mut buf = String::new();
        append_comment_line(&mut buf, "## double");
        assert_eq!(buf, "double");
    }

    #[test]
    fn append_comment_line_ignores_empty_comment() {
        let mut buf = String::from("alice");
        append_comment_line(&mut buf, "#");
        assert_eq!(buf, "alice", "buffer must not gain a trailing space");
    }

    #[test]
    fn append_comment_line_ignores_whitespace_only_comment() {
        let mut buf = String::from("alice");
        append_comment_line(&mut buf, "#   ");
        assert_eq!(buf, "alice");
    }

    #[test]
    fn append_comment_line_trims_trailing_whitespace() {
        let mut buf = String::new();
        append_comment_line(&mut buf, "# alice   ");
        assert_eq!(buf, "alice");
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

        assert!(get(&args, Duration::from_secs(30)).is_err());
    }
}
