// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use anyhow::Result;

use crate::cli;
use crate::models::password_store::OneTimePassword;

pub fn merge_one_time_passwords(
    common_ancestor_one_time_passwords: &BTreeMap<String, OneTimePassword>,
    current_version_one_time_passwords: &BTreeMap<String, OneTimePassword>,
    other_version_one_time_passwords: &BTreeMap<String, OneTimePassword>,
) -> Result<BTreeMap<String, OneTimePassword>> {
    cli::merger::three_way_merge_map(
        common_ancestor_one_time_passwords,
        current_version_one_time_passwords,
        other_version_one_time_passwords,
        "one-time password",
    )
}

#[cfg(test)]
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
