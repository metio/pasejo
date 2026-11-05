// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use anyhow::Result;

use crate::cli;
use crate::models::password_store::{OneTimePassword, OneTimePasswordType};

/// Three-way merge of the OTP maps. A HOTP entry whose counter both sides
/// advanced past the ancestor's, with every other field unchanged, merges to
/// the higher counter: emitting a lower counter again would replay codes the
/// server has already consumed.
pub fn merge_one_time_passwords(
    common_ancestor_one_time_passwords: &BTreeMap<String, OneTimePassword>,
    current_version_one_time_passwords: &BTreeMap<String, OneTimePassword>,
    other_version_one_time_passwords: &BTreeMap<String, OneTimePassword>,
) -> Result<BTreeMap<String, OneTimePassword>> {
    let mut current = current_version_one_time_passwords.clone();
    let mut other = other_version_one_time_passwords.clone();
    for (key, ancestor) in common_ancestor_one_time_passwords {
        if let (Some(current_value), Some(other_value)) = (current.get_mut(key), other.get_mut(key))
            && both_advanced_only_the_hotp_counter(ancestor, current_value, other_value)
        {
            let counter = current_value.counter.max(other_value.counter);
            current_value.counter = counter;
            other_value.counter = counter;
        }
    }
    cli::merger::three_way_merge_map(
        common_ancestor_one_time_passwords,
        &current,
        &other,
        "one-time password",
    )
}

fn both_advanced_only_the_hotp_counter(
    ancestor: &OneTimePassword,
    current: &OneTimePassword,
    other: &OneTimePassword,
) -> bool {
    let without_counter = |otp: &OneTimePassword| {
        let mut otp = otp.clone();
        otp.counter = 0;
        otp
    };
    ancestor.otp_type == OneTimePasswordType::Hotp
        && current.counter > ancestor.counter
        && other.counter > ancestor.counter
        && without_counter(current) == without_counter(ancestor)
        && without_counter(other) == without_counter(ancestor)
}

#[cfg(test)]
#[allow(clippy::redundant_clone)]
mod tests {
    use super::*;
    use crate::models::password_store::{
        OneTimePassword, OneTimePasswordAlgorithm, OneTimePasswordType,
    };

    fn sample_otp(secret: &str) -> OneTimePassword {
        OneTimePassword {
            secret: secret.to_string(),
            otp_type: OneTimePasswordType::Totp,
            algorithm: OneTimePasswordAlgorithm::Sha1,
            digits: 6,
            period: 30,
            counter: 0,
            skew: 0,
        }
    }

    #[test]
    fn merge_one_time_passwords_with_empty_maps() {
        let common_ancestor = BTreeMap::new();
        let current_version = BTreeMap::new();
        let other_version = BTreeMap::new();
        let result = merge_one_time_passwords(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BTreeMap::new());
    }

    #[test]
    fn merge_without_change() {
        let otp = sample_otp("AAAA");
        let common = BTreeMap::from([("github".to_string(), otp.clone())]);
        let result = merge_one_time_passwords(&common, &common.clone(), &common.clone()).unwrap();
        assert_eq!(result, common);
    }

    #[test]
    fn merge_preserves_change_made_only_in_current() {
        let original = sample_otp("AAAA");
        let updated = sample_otp("BBBB");
        let common = BTreeMap::from([("github".to_string(), original.clone())]);
        let current = BTreeMap::from([("github".to_string(), updated.clone())]);
        let other = BTreeMap::from([("github".to_string(), original)]);

        let result = merge_one_time_passwords(&common, &current, &other).unwrap();
        assert_eq!(result.get("github"), Some(&updated));
    }

    #[test]
    fn merge_preserves_change_made_only_in_other() {
        let original = sample_otp("AAAA");
        let updated = sample_otp("BBBB");
        let common = BTreeMap::from([("github".to_string(), original.clone())]);
        let current = BTreeMap::from([("github".to_string(), original)]);
        let other = BTreeMap::from([("github".to_string(), updated.clone())]);

        let result = merge_one_time_passwords(&common, &current, &other).unwrap();
        assert_eq!(result.get("github"), Some(&updated));
    }

    #[test]
    fn merge_reports_conflict_when_both_sides_change() {
        let original = sample_otp("AAAA");
        let from_current = sample_otp("BBBB");
        let from_other = sample_otp("CCCC");
        let common = BTreeMap::from([("github".to_string(), original)]);
        let current = BTreeMap::from([("github".to_string(), from_current)]);
        let other = BTreeMap::from([("github".to_string(), from_other)]);

        assert!(merge_one_time_passwords(&common, &current, &other).is_err());
    }

    #[test]
    fn merge_drops_entry_removed_in_one_side_when_unchanged_in_other() {
        let original = sample_otp("AAAA");
        let common = BTreeMap::from([("github".to_string(), original.clone())]);
        let current: BTreeMap<String, OneTimePassword> = BTreeMap::new();
        let other = BTreeMap::from([("github".to_string(), original)]);

        let result = merge_one_time_passwords(&common, &current, &other).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn merge_keeps_addition_made_in_either_side() {
        let added = sample_otp("AAAA");
        let common: BTreeMap<String, OneTimePassword> = BTreeMap::new();
        let current = BTreeMap::from([("github".to_string(), added.clone())]);
        let other: BTreeMap<String, OneTimePassword> = BTreeMap::new();

        let result = merge_one_time_passwords(&common, &current, &other).unwrap();
        assert_eq!(result.get("github"), Some(&added));
    }

    fn hotp_with_counter(counter: u64) -> OneTimePassword {
        let mut otp = sample_otp("AAAA");
        otp.otp_type = OneTimePasswordType::Hotp;
        otp.period = 0;
        otp.counter = counter;
        otp
    }

    #[test]
    fn merge_takes_higher_counter_when_both_sides_advanced_a_hotp_counter() {
        crate::cli::i18n::init_for_tests();
        let common = BTreeMap::from([("bank".to_string(), hotp_with_counter(5))]);
        let current = BTreeMap::from([("bank".to_string(), hotp_with_counter(6))]);
        let other = BTreeMap::from([("bank".to_string(), hotp_with_counter(7))]);

        let result = merge_one_time_passwords(&common, &current, &other).unwrap();
        assert_eq!(result.get("bank"), Some(&hotp_with_counter(7)));

        let result = merge_one_time_passwords(&common, &other, &current).unwrap();
        assert_eq!(result.get("bank"), Some(&hotp_with_counter(7)));
    }

    #[test]
    fn merge_reports_conflict_when_a_hotp_counter_advanced_and_another_field_changed() {
        crate::cli::i18n::init_for_tests();
        let mut changed_secret = hotp_with_counter(7);
        changed_secret.secret = String::from("BBBB");
        let common = BTreeMap::from([("bank".to_string(), hotp_with_counter(5))]);
        let current = BTreeMap::from([("bank".to_string(), hotp_with_counter(6))]);
        let other = BTreeMap::from([("bank".to_string(), changed_secret)]);

        assert!(merge_one_time_passwords(&common, &current, &other).is_err());
    }

    #[test]
    fn merge_reports_conflict_when_one_side_lowered_a_hotp_counter() {
        crate::cli::i18n::init_for_tests();
        let common = BTreeMap::from([("bank".to_string(), hotp_with_counter(5))]);
        let current = BTreeMap::from([("bank".to_string(), hotp_with_counter(2))]);
        let other = BTreeMap::from([("bank".to_string(), hotp_with_counter(7))]);

        assert!(merge_one_time_passwords(&common, &current, &other).is_err());
    }

    #[test]
    fn merge_reports_conflict_when_both_sides_add_diverging_entries() {
        let from_current = sample_otp("AAAA");
        let from_other = sample_otp("BBBB");
        let common: BTreeMap<String, OneTimePassword> = BTreeMap::new();
        let current = BTreeMap::from([("github".to_string(), from_current)]);
        let other = BTreeMap::from([("github".to_string(), from_other)]);

        assert!(merge_one_time_passwords(&common, &current, &other).is_err());
    }
}
