// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli::i18n;

/// Validates a username intended for interpolation into a public-key URL
/// (e.g. `https://github.com/{username}.keys`).
///
/// Rejects anything that could change the URL's meaning — slashes, query
/// and fragment characters, whitespace, and anything outside a
/// conservative ASCII alphanumeric + `-` + `_` + `.` set. The first
/// character must be alphanumeric to disallow leading punctuation, and
/// `..` sequences are rejected to prevent path traversal in the URL.
pub fn validate(username: &str) -> anyhow::Result<&str> {
    if username.is_empty() {
        anyhow::bail!(i18n::error_username_empty());
    }
    if username.contains("..") {
        anyhow::bail!(i18n::error_username_contains_dotdot(username));
    }
    for (index, c) in username.chars().enumerate() {
        let allowed = if index == 0 {
            c.is_ascii_alphanumeric()
        } else {
            c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
        };
        if !allowed {
            anyhow::bail!(i18n::error_username_invalid_character(username));
        }
    }
    Ok(username)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_typical_usernames() {
        assert!(validate("octocat").is_ok());
        assert!(validate("user-name").is_ok());
        assert!(validate("User_123").is_ok());
        assert!(validate("first.last").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(validate("").is_err());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate("foo/../bar").is_err());
        assert!(validate("../etc/passwd").is_err());
        assert!(validate("foo..bar").is_err());
        assert!(validate("..").is_err());
    }

    #[test]
    fn rejects_url_metacharacters() {
        assert!(validate("foo?x=1").is_err());
        assert!(validate("foo#frag").is_err());
        assert!(validate("foo bar").is_err());
        assert!(validate("foo:bar").is_err());
        assert!(validate("foo@host").is_err());
    }

    #[test]
    fn rejects_leading_punctuation() {
        assert!(validate("-foo").is_err());
        assert!(validate("_foo").is_err());
        assert!(validate(".foo").is_err());
    }
}
