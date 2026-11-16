// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Reading single values out of a secret's text: its fields and lines.
//!
//! The CLI and the Kubernetes bridge select values through
//! these functions, so `pasejo secret show --field` and a bridge request
//! with `field=` agree on every secret.

/// The `key: value` fields of a secret, in order: every non-blank line after
/// the first non-blank one (the password) that contains `": "`. Keys are
/// trimmed, values are verbatim.
pub fn fields(secret: &str) -> impl Iterator<Item = (&str, &str)> {
    secret
        .lines()
        .filter(|line| !line.trim().is_empty())
        .skip(1)
        .filter_map(|line| line.split_once(": "))
        .map(|(key, value)| (key.trim(), value))
}

/// The value of the field named `key`, with surrounding whitespace removed,
/// since stores often align values (`login:    alice`). Keys compare
/// exactly. When a field repeats, the last one wins.
#[must_use]
pub fn field<'a>(secret: &'a str, key: &str) -> Option<&'a str> {
    fields(secret)
        .filter(|(name, _)| *name == key)
        .last()
        .map(|(_, value)| value.trim())
}

/// Line `n` of a secret, counted from 1. A negative `n` counts from the
/// end, so `-1` is the last line. Line 0 does not exist.
#[must_use]
pub fn line(secret: &str, n: isize) -> Option<&str> {
    if n > 0 {
        let index = n.unsigned_abs() - 1;
        return secret.lines().nth(index);
    }
    let from_end = n.unsigned_abs();
    if from_end == 0 {
        return None;
    }
    let lines: Vec<&str> = secret.lines().collect();
    lines
        .len()
        .checked_sub(from_end)
        .and_then(|index| lines.get(index).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str =
        "\n  \nhunter2\nuser:   alice  \nurl: https://example.com\nnote\nuser: bob\n";

    #[test]
    fn fields_skip_blank_lines_and_the_password() {
        assert_eq!(
            fields(SECRET).collect::<Vec<_>>(),
            vec![
                ("user", "  alice  "),
                ("url", "https://example.com"),
                ("user", "bob")
            ]
        );
    }

    #[test]
    fn the_last_repeated_field_wins_and_values_are_trimmed() {
        assert_eq!(field(SECRET, "user"), Some("bob"));
        assert_eq!(field("pw\nuser:   alice  ", "user"), Some("alice"));
        assert_eq!(field(SECRET, "User"), None);
        assert_eq!(field(SECRET, "note"), None);
        assert_eq!(
            field("user: alice", "user"),
            None,
            "the first line is the password"
        );
    }

    #[test]
    fn lines_count_from_the_start_or_the_end() {
        let secret = "one\ntwo\nthree";
        assert_eq!(line(secret, 1), Some("one"));
        assert_eq!(line(secret, 3), Some("three"));
        assert_eq!(line(secret, 4), None);
        assert_eq!(line(secret, -1), Some("three"));
        assert_eq!(line(secret, -3), Some("one"));
        assert_eq!(line(secret, -4), None);
        assert_eq!(line(secret, 0), None);
        assert_eq!(line("", 1), None);
        assert_eq!(line(secret, isize::MIN), None);
        assert_eq!(line(secret, isize::MAX), None);
    }
}
