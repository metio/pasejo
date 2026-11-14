// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use zeroize::Zeroizing;

use crate::exporters::parser;

/// The part of a secret a command reads: all of it, one line, everything
/// after the first lines, or one `key: value` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    All,
    /// 1-indexed line number. Positive counts from the start; negative
    /// from the end. Never 0 (clap rejects 0 at parse time).
    Line(isize),
    /// Skip the first N lines. N > 0 (clap rejects 0).
    SkipFirst(u64),
    /// The value of the field with this key, as [`parser::field`] reads it.
    Field(String),
}

impl Selection {
    /// clap makes the three options mutually exclusive; should several
    /// arrive anyway, `line` wins over `skip_lines` over `field`.
    pub fn from_args(line: Option<isize>, skip_lines: Option<u64>, field: Option<&str>) -> Self {
        match (line, skip_lines, field) {
            (Some(n), _, _) => Self::Line(n),
            (None, Some(n), _) => Self::SkipFirst(n),
            (None, None, Some(key)) => Self::Field(key.to_owned()),
            (None, None, None) => Self::All,
        }
    }

    /// The selected text, or `None` when the secret has no such line or
    /// field. Skipping past the end selects an empty remainder rather than
    /// nothing.
    pub fn extract(&self, secret: &str) -> Option<Zeroizing<String>> {
        let extracted = match self {
            Self::All => Some(secret.to_owned()),
            Self::Line(n) if *n > 0 => {
                let index = usize::try_from(*n).unwrap_or(usize::MAX).saturating_sub(1);
                secret.lines().nth(index).map(str::to_owned)
            }
            Self::Line(n) => {
                // n < 0 here (clap rejects 0); -1 == last line.
                let lines: Vec<&str> = secret.lines().collect();
                let from_end = n.unsigned_abs();
                lines
                    .len()
                    .checked_sub(from_end)
                    .filter(|_| from_end > 0)
                    .and_then(|index| lines.get(index))
                    .map(|line| (*line).to_owned())
            }
            Self::SkipFirst(n) => Some(
                secret
                    .lines()
                    .skip(usize::try_from(*n).unwrap_or(usize::MAX))
                    .collect::<Vec<&str>>()
                    .join("\n"),
            ),
            Self::Field(key) => parser::field(secret, key).map(str::to_owned),
        };
        extracted.map(Zeroizing::new)
    }
}

#[cfg(test)]
mod tests {
    use super::Selection;

    const THREE_LINE_SECRET: &str = "password\nuser: alice\nurl: https://example.com";

    fn extract(secret: &str, selection: &Selection) -> Option<String> {
        selection
            .extract(secret)
            .map(|text| text.as_str().to_owned())
    }

    #[test]
    fn from_args_with_no_arg_is_all() {
        assert_eq!(Selection::from_args(None, None, None), Selection::All);
    }

    #[test]
    fn from_args_with_only_line_is_line() {
        assert_eq!(
            Selection::from_args(Some(2), None, None),
            Selection::Line(2)
        );
    }

    #[test]
    fn from_args_with_only_skip_lines_is_skip_first() {
        assert_eq!(
            Selection::from_args(None, Some(3), None),
            Selection::SkipFirst(3)
        );
    }

    #[test]
    fn from_args_with_only_field_is_field() {
        assert_eq!(
            Selection::from_args(None, None, Some("user")),
            Selection::Field(String::from("user"))
        );
    }

    #[test]
    fn from_args_prefers_line_when_several_are_present() {
        // clap enforces mutual exclusion, so this combination cannot arrive at
        // runtime — but the resolver picks `line` as a defensive fallback.
        assert_eq!(
            Selection::from_args(Some(1), Some(2), Some("user")),
            Selection::Line(1)
        );
    }

    #[test]
    fn all_returns_full_secret() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::All).as_deref(),
            Some(THREE_LINE_SECRET)
        );
    }

    #[test]
    fn line_one_returns_first_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(1)).as_deref(),
            Some("password")
        );
    }

    #[test]
    fn line_in_range_returns_that_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(2)).as_deref(),
            Some("user: alice")
        );
    }

    #[test]
    fn line_at_end_returns_that_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(3)).as_deref(),
            Some("url: https://example.com")
        );
    }

    #[test]
    fn line_past_end_selects_nothing() {
        for n in [4, 99, isize::MAX] {
            assert_eq!(extract(THREE_LINE_SECRET, &Selection::Line(n)), None);
        }
    }

    #[test]
    fn line_minus_one_returns_last_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(-1)).as_deref(),
            Some("url: https://example.com")
        );
    }

    #[test]
    fn line_minus_two_returns_second_to_last_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(-2)).as_deref(),
            Some("user: alice")
        );
    }

    #[test]
    fn line_negative_equal_to_line_count_returns_first_line() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Line(-3)).as_deref(),
            Some("password")
        );
    }

    #[test]
    fn line_negative_past_start_selects_nothing() {
        for n in [-4, isize::MIN] {
            assert_eq!(extract(THREE_LINE_SECRET, &Selection::Line(n)), None);
        }
    }

    #[test]
    fn skip_first_one_returns_everything_after_password() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::SkipFirst(1)).as_deref(),
            Some("user: alice\nurl: https://example.com")
        );
    }

    #[test]
    fn skip_first_two_returns_last_line_only() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::SkipFirst(2)).as_deref(),
            Some("url: https://example.com")
        );
    }

    #[test]
    fn skip_first_at_or_past_end_returns_empty() {
        for n in [3, 99, u64::MAX] {
            assert_eq!(
                extract(THREE_LINE_SECRET, &Selection::SkipFirst(n)).as_deref(),
                Some("")
            );
        }
    }

    #[test]
    fn empty_secret_with_all_or_skip_first_returns_empty() {
        assert_eq!(extract("", &Selection::All).as_deref(), Some(""));
        assert_eq!(extract("", &Selection::SkipFirst(1)).as_deref(), Some(""));
    }

    #[test]
    fn empty_secret_with_any_line_selects_nothing() {
        for n in [1, 5, -1] {
            assert_eq!(extract("", &Selection::Line(n)), None);
        }
    }

    #[test]
    fn single_line_secret_with_line_one_or_minus_one_returns_that_line() {
        for n in [1, -1] {
            assert_eq!(
                extract("only-line", &Selection::Line(n)).as_deref(),
                Some("only-line")
            );
        }
    }

    #[test]
    fn single_line_secret_with_line_two_selects_nothing() {
        assert_eq!(extract("only-line", &Selection::Line(2)), None);
    }

    #[test]
    fn trailing_newline_does_not_count_as_extra_line() {
        assert_eq!(extract("a\nb\n", &Selection::Line(3)), None);
    }

    #[test]
    fn field_returns_the_value_of_that_field() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Field(String::from("user"))).as_deref(),
            Some("alice")
        );
    }

    #[test]
    fn missing_field_selects_nothing() {
        assert_eq!(
            extract(THREE_LINE_SECRET, &Selection::Field(String::from("token"))),
            None
        );
    }
}
