// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

pub fn format_recipient(public_key: &str, name: &str) -> String {
    if name.is_empty() {
        public_key.to_owned()
    } else {
        format!("# {name}\n{public_key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_name_returns_public_key_alone() {
        assert_eq!(format_recipient("age1xyz", ""), "age1xyz");
    }

    #[test]
    fn with_name_prepends_comment_line() {
        assert_eq!(format_recipient("age1xyz", "alice"), "# alice\nage1xyz");
    }

    #[test]
    fn name_with_whitespace_is_preserved_verbatim() {
        assert_eq!(
            format_recipient("age1xyz", "Alice Example"),
            "# Alice Example\nage1xyz"
        );
    }

    #[test]
    fn empty_public_key_with_name_still_formats() {
        assert_eq!(format_recipient("", "alice"), "# alice\n");
    }
}
