// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

pub struct ParsedSecret {
    pub password: Option<String>,
    pub notes: Vec<String>,
    pub fields: BTreeMap<String, String>,
}

pub fn parse_secret(secret: &str) -> ParsedSecret {
    let mut password: Option<String> = None;
    let mut notes: Vec<String> = vec![];
    let mut fields: BTreeMap<String, String> = BTreeMap::new();

    for line in secret.lines() {
        // Skip lines that contain only whitespace — they are visual padding,
        // not content.
        if line.trim().is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(": ") {
            // Keys are conventional identifiers, so indentation is dropped.
            // Values are stored verbatim so deliberate whitespace survives
            // the round-trip (e.g. a password that intentionally trails a
            // space to satisfy a server's odd validator).
            fields.insert(key.trim().to_owned(), value.to_owned());
        } else if password.is_none() {
            password = Some(line.to_owned());
        } else {
            notes.push(line.to_owned());
        }
    }

    ParsedSecret {
        password,
        notes,
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_empty_parsed_secret() {
        let parsed = parse_secret("");
        assert!(parsed.password.is_none());
        assert!(parsed.notes.is_empty());
        assert!(parsed.fields.is_empty());
    }

    #[test]
    fn first_non_field_line_becomes_password() {
        let parsed = parse_secret("hunter2");
        assert_eq!(parsed.password.as_deref(), Some("hunter2"));
        assert!(parsed.notes.is_empty());
        assert!(parsed.fields.is_empty());
    }

    #[test]
    fn lines_with_colon_separator_become_fields() {
        let parsed = parse_secret("hunter2\nuser: alice\nurl: https://example.com");
        assert_eq!(parsed.password.as_deref(), Some("hunter2"));
        assert_eq!(parsed.fields.get("user").map(String::as_str), Some("alice"));
        assert_eq!(
            parsed.fields.get("url").map(String::as_str),
            Some("https://example.com")
        );
        assert!(parsed.notes.is_empty());
    }

    #[test]
    fn additional_non_field_lines_become_notes() {
        let parsed = parse_secret("hunter2\nfirst note\nsecond note");
        assert_eq!(parsed.password.as_deref(), Some("hunter2"));
        assert_eq!(parsed.notes, vec!["first note", "second note"]);
    }

    #[test]
    fn empty_lines_are_skipped() {
        let parsed = parse_secret("\n\nhunter2\n\nnote\n");
        assert_eq!(parsed.password.as_deref(), Some("hunter2"));
        assert_eq!(parsed.notes, vec!["note"]);
    }

    #[test]
    fn password_preserves_deliberate_whitespace() {
        let parsed = parse_secret("  hunter2  ");
        assert_eq!(parsed.password.as_deref(), Some("  hunter2  "));
    }

    #[test]
    fn field_key_is_trimmed_but_value_is_preserved_verbatim() {
        // Indented keys lose their indentation (keys are conventional
        // identifiers), but value padding survives so users can store
        // whitespace-sensitive content like server tokens that pad to a
        // fixed width.
        let parsed = parse_secret("hunter2\n  user:   alice   ");
        assert_eq!(parsed.password.as_deref(), Some("hunter2"));
        assert_eq!(
            parsed.fields.get("user").map(String::as_str),
            Some("  alice   ")
        );
    }

    #[test]
    fn notes_preserve_deliberate_whitespace() {
        let parsed = parse_secret("hunter2\n  indented note  ");
        assert_eq!(parsed.notes, vec!["  indented note  "]);
    }

    #[test]
    fn whitespace_only_lines_are_skipped() {
        let parsed = parse_secret("\n   \n\thunter2\n \nnote");
        // The first non-blank line is "\thunter2" (with a leading tab); the
        // leading tab is part of the password content.
        assert_eq!(parsed.password.as_deref(), Some("\thunter2"));
        assert_eq!(parsed.notes, vec!["note"]);
    }

    #[test]
    fn line_without_colon_space_is_not_a_field() {
        // The parser specifically looks for ": " (colon + space), so "user:alice"
        // is treated as the password (first non-field line), not a field.
        let parsed = parse_secret("user:alice");
        assert_eq!(parsed.password.as_deref(), Some("user:alice"));
        assert!(parsed.fields.is_empty());
    }
}
