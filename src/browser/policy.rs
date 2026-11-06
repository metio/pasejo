// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Per-secret browser policies, stored as reserved fields inside the
//! encrypted secret so they travel with the store and cannot be changed by
//! anyone without the store's identities.

/// Field that limits which matches release the secret to a browser.
pub const FILL_FIELD: &str = "pasejo-fill";

/// Field that asks the browser extension to confirm before it fills.
pub const CONFIRM_FIELD: &str = "pasejo-confirm";

/// Which matches may hand the secret to a browser. Ordered from least to
/// most restrictive, so combining two values keeps the stricter one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum FillPolicy {
    #[default]
    AnyMatch,
    ExactHost,
    Never,
}

impl FillPolicy {
    /// An unrecognised value maps to `Never`: a misspelt restriction must
    /// not silently widen what the browser receives.
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "exact-host" => Self::ExactHost,
            _ => Self::Never,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Policy {
    pub fill: FillPolicy,
    /// The extension asks the user before filling. The host only reports
    /// the flag; enforcing it is up to the extension.
    pub confirm: bool,
}

impl Policy {
    /// Reads the reserved fields from a raw secret, using the same line
    /// grammar as `exporters::parser::parse_secret`: the first non-blank
    /// line is the password, later `key: value` lines are fields. Keys
    /// compare case-insensitively, and when a field repeats, the strictest
    /// value wins. Any `pasejo-confirm` value requests confirmation, since
    /// `always` is the only one defined.
    pub fn from_secret(secret: &str) -> Self {
        let mut policy = Self::default();
        let fields = secret
            .lines()
            .filter(|line| !line.trim().is_empty())
            .skip(1)
            .filter_map(|line| line.split_once(": "));
        for (key, value) in fields {
            let key = key.trim();
            if key.eq_ignore_ascii_case(FILL_FIELD) {
                policy.fill = policy.fill.max(FillPolicy::parse(value));
            } else if key.eq_ignore_ascii_case(CONFIRM_FIELD) {
                policy.confirm = true;
            }
        }
        policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_without_reserved_fields_allows_any_match_without_confirmation() {
        let policy = Policy::from_secret("hunter2\nurl: https://example.com");
        assert_eq!(policy, Policy::default());
        assert_eq!(policy.fill, FillPolicy::AnyMatch);
        assert!(!policy.confirm);
    }

    #[test]
    fn fill_never_is_recognised() {
        let policy = Policy::from_secret("hunter2\npasejo-fill: never");
        assert_eq!(policy.fill, FillPolicy::Never);
    }

    #[test]
    fn fill_exact_host_is_recognised() {
        let policy = Policy::from_secret("hunter2\npasejo-fill: exact-host");
        assert_eq!(policy.fill, FillPolicy::ExactHost);
    }

    #[test]
    fn keys_and_values_compare_case_insensitively_and_ignore_padding() {
        let policy = Policy::from_secret("hunter2\n  Pasejo-Fill:   EXACT-HOST  ");
        assert_eq!(policy.fill, FillPolicy::ExactHost);
    }

    #[test]
    fn an_unrecognised_fill_value_refuses_the_browser() {
        let policy = Policy::from_secret("hunter2\npasejo-fill: exact_host");
        assert_eq!(policy.fill, FillPolicy::Never);
    }

    #[test]
    fn the_strictest_of_repeated_fill_fields_wins() {
        let policy = Policy::from_secret("hunter2\npasejo-fill: never\npasejo-fill: exact-host");
        assert_eq!(policy.fill, FillPolicy::Never);
    }

    #[test]
    fn confirm_always_requests_confirmation() {
        let policy = Policy::from_secret("hunter2\npasejo-confirm: always");
        assert!(policy.confirm);
        assert_eq!(policy.fill, FillPolicy::AnyMatch);
    }

    #[test]
    fn any_confirm_value_requests_confirmation() {
        let policy = Policy::from_secret("hunter2\npasejo-confirm: sometimes");
        assert!(policy.confirm);
    }

    #[test]
    fn the_password_line_is_never_read_as_a_policy() {
        let policy = Policy::from_secret("pasejo-fill: never\nurl: https://example.com");
        assert_eq!(policy.fill, FillPolicy::AnyMatch);
    }

    #[test]
    fn a_line_without_colon_space_is_not_a_policy() {
        let policy = Policy::from_secret("hunter2\npasejo-fill:never");
        assert_eq!(policy.fill, FillPolicy::AnyMatch);
    }

    #[test]
    fn leading_blank_lines_do_not_shift_the_password_line() {
        let policy = Policy::from_secret("\n  \nhunter2\npasejo-fill: never");
        assert_eq!(policy.fill, FillPolicy::Never);
    }
}
