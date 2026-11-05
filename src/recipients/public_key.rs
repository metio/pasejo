// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;
use std::time::Duration;

use age::ssh::ParseRecipientKeyError;

use crate::cli::i18n;
use crate::downloader::{Provider, download_public_key};
use crate::models::cli::RecipientKeysArgs;

pub fn get(
    args: &RecipientKeysArgs,
    download_timeout: Duration,
) -> anyhow::Result<Vec<(String, String)>> {
    if let Some(public_key) = &args.public_key {
        Ok(vec![split_ssh_key(public_key)?])
    } else if let Some(codeberg_username) = &args.codeberg {
        download_keys(Provider::Codeberg, codeberg_username, download_timeout)
    } else if let Some(github_username) = &args.github {
        download_keys(Provider::Github, github_username, download_timeout)
    } else if let Some(gitlab_username) = &args.gitlab {
        download_keys(Provider::Gitlab, gitlab_username, download_timeout)
    } else if let Some(filename) = &args.file {
        let public_keys = parse_recipient_file_body(&fs::read_to_string(filename)?)?;
        if public_keys.is_empty() {
            anyhow::bail!(i18n::error_no_public_key_found_in_file(filename))
        }
        Ok(public_keys)
    } else {
        anyhow::bail!(i18n::error_no_public_key_source())
    }
}

/// Parses the body of a `--file` recipient list into `(key, comment)` pairs.
/// A line starting with `#` accumulates as the comment for the *next* key
/// line (multiple comment lines join with a space); a key line with no
/// preceding comment keeps its own inline comment. Pure, so the
/// comment-association logic is testable without a tempfile. Unlike the
/// `.keys` body parser ([`split_keys_body`]), this mirrors the file format
/// exactly and does not skip blank lines.
fn parse_recipient_file_body(content: &str) -> anyhow::Result<Vec<(String, String)>> {
    let mut public_keys = vec![];
    let mut comment = String::new();
    for line in content.lines() {
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
    Ok(public_keys)
}

/// Fetches a provider's `.keys` body and turns EVERY key line into a
/// recipient. A `.keys` endpoint returns one public key per line, so a
/// user with several enrolled keys must contribute all of them —
/// keeping only the first would silently leave that user unable to
/// decrypt from their other devices. (Comment lines / per-key comments
/// don't occur in `.keys` output, so unlike the `--file` path there is
/// no `#`-comment handling.) An account with no published keys yields
/// an empty body; rather than fall through and create a recipient-less
/// (undecryptable) store, that bails with a clear, localized error
/// naming the provider and username.
fn download_keys(
    provider: Provider,
    username: &str,
    download_timeout: Duration,
) -> anyhow::Result<Vec<(String, String)>> {
    let body = download_public_key(provider, username, download_timeout)?;
    let (public_keys, unsupported_key_types) = partition_encryptable(split_keys_body(&body)?);
    for key_type in &unsupported_key_types {
        i18n::recipient_unsupported_key_skipped(provider.name(), key_type, username);
    }
    if public_keys.is_empty() {
        anyhow::bail!(i18n::error_no_public_keys_for_user(
            provider.name(),
            username
        ))
    }
    Ok(public_keys)
}

/// Splits a `.keys` response body (one public key per line) into
/// `(key, comment)` pairs, skipping blank lines. Pure so the
/// multi-key fan-out is testable without a network round-trip.
fn split_keys_body(body: &str) -> anyhow::Result<Vec<(String, String)>> {
    let mut public_keys = vec![];
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        public_keys.push(split_ssh_key(line)?);
    }
    Ok(public_keys)
}

/// Separates SSH keys that age cannot encrypt to (unsupported key types,
/// RSA moduli outside age's bounds) from the rest and returns their key
/// types. Lines age rejects as malformed stay in the first list so they
/// surface as an encryption error instead of being dropped.
fn partition_encryptable(keys: Vec<(String, String)>) -> (Vec<(String, String)>, Vec<String>) {
    let mut encryptable = vec![];
    let mut unsupported_key_types = vec![];
    for (key, comment) in keys {
        match key.parse::<age::ssh::Recipient>() {
            Err(
                ParseRecipientKeyError::Unsupported(_)
                | ParseRecipientKeyError::RsaModulusTooSmall
                | ParseRecipientKeyError::RsaModulusTooLarge
                | ParseRecipientKeyError::Ignore,
            ) => unsupported_key_types
                .push(key.split_whitespace().next().unwrap_or_default().to_owned()),
            _ => encryptable.push((key, comment)),
        }
    }
    (encryptable, unsupported_key_types)
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
            anyhow::bail!(i18n::error_invalid_ssh_public_key_format())
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
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

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
    fn split_keys_body_returns_every_key_in_a_multi_key_response() {
        // Regression for review9 R1: a provider account with several
        // enrolled keys returns one key per line. Keeping only the
        // first (the old `split_ssh_key(&whole_body)` behavior) would
        // silently fold the remaining keys into a comment and leave
        // that user unable to decrypt from their other devices.
        let body = "ssh-ed25519 AAAA alice@laptop\nssh-rsa BBBB alice@desktop\nssh-ed25519 CCCC";
        let keys = split_keys_body(body).unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], ("ssh-ed25519 AAAA".into(), "alice@laptop".into()));
        assert_eq!(keys[1], ("ssh-rsa BBBB".into(), "alice@desktop".into()));
        assert_eq!(keys[2], ("ssh-ed25519 CCCC".into(), String::new()));
    }

    #[test]
    fn split_keys_body_skips_blank_lines() {
        let body = "\nssh-ed25519 AAAA\n\n  \nssh-rsa BBBB\n";
        let keys = split_keys_body(body).unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].0, "ssh-ed25519 AAAA");
        assert_eq!(keys[1].0, "ssh-rsa BBBB");
    }

    #[test]
    fn split_keys_body_is_empty_for_an_account_with_no_keys() {
        // An account that exists but published no SSH keys returns an
        // empty body; `download_keys` turns this empty Vec into a
        // clear "no public keys for user" error rather than creating a
        // recipient-less store (review9 R4).
        assert!(split_keys_body("").unwrap().is_empty());
        assert!(split_keys_body("\n  \n\n").unwrap().is_empty());
    }

    #[test]
    fn split_keys_body_propagates_a_malformed_key_line() {
        // A truncated `ssh-…` line is rejected (mirrors split_ssh_key).
        assert!(split_keys_body("ssh-ed25519 AAAA\nssh-rsa").is_err());
    }

    #[test]
    fn partition_encryptable_separates_key_types_age_cannot_encrypt_to() {
        let ed25519 =
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOycTzXsl1jetsf+Ezi/2FCVU8oydXxxJhb9D7n6MlGf";
        let ecdsa = "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBHFliOyIZs1gxGF3fmDxFykQhE88wy6AKDGFBfn0R6ZuvRmENABZQa9+pj9hMki+LX0qDJbmHTiWDbYv/cmFt/Q=";
        let keys = split_keys_body(&format!("{ecdsa}\n{ed25519}\nssh-ed25519 AAAA")).unwrap();

        let (encryptable, unsupported) = partition_encryptable(keys);

        assert_eq!(
            encryptable,
            vec![
                (ed25519.to_owned(), String::new()),
                ("ssh-ed25519 AAAA".to_owned(), String::new()),
            ]
        );
        assert_eq!(unsupported, vec!["ecdsa-sha2-nistp256".to_owned()]);
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
    fn parse_recipient_file_body_attaches_preceding_comment_to_next_key() {
        let keys = parse_recipient_file_body("# alice\nage1abc\n# bob\nage1def\n").unwrap();
        assert_eq!(keys[0], (String::from("age1abc"), String::from("alice")));
        assert_eq!(keys[1], (String::from("age1def"), String::from("bob")));
    }

    #[test]
    fn parse_recipient_file_body_resets_the_comment_between_keys() {
        let keys =
            parse_recipient_file_body("# alice\nage1abc\nage1def\n# bob\n# admin\nage1ghi\n")
                .unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0], (String::from("age1abc"), String::from("alice")));
        assert_eq!(keys[1], (String::from("age1def"), String::new()));
        assert_eq!(
            keys[2],
            (String::from("age1ghi"), String::from("bob admin"))
        );
    }

    #[test]
    fn parse_recipient_file_body_uses_an_ssh_keys_own_inline_comment() {
        let keys = parse_recipient_file_body("ssh-ed25519 AAAA alice@host\n").unwrap();
        assert_eq!(keys[0], ("ssh-ed25519 AAAA".into(), "alice@host".into()));
    }

    #[test]
    fn parse_recipient_file_body_propagates_a_malformed_key_line() {
        assert!(parse_recipient_file_body("ssh-ed25519\n").is_err());
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
