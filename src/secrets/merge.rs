// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli;
use anyhow::Result;
use std::collections::BTreeMap;

pub fn merge_secrets(
    common_ancestor_secrets: &BTreeMap<String, String>,
    current_version_secrets: &BTreeMap<String, String>,
    other_version_secrets: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    cli::merger::three_way_merge_map(
        common_ancestor_secrets,
        current_version_secrets,
        other_version_secrets,
        "secret",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_secrets_with_empty_maps() {
        let common_ancestor = BTreeMap::new();
        let current_version = BTreeMap::new();
        let other_version = BTreeMap::new();
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BTreeMap::new());
    }

    #[test]
    fn merge_secrets_without_change() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let other_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([("key1".to_string(), "value1".to_string())])
        );
    }

    #[test]
    fn merge_secrets_removed_in_current() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([]);
        let other_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BTreeMap::from([]));
    }

    #[test]
    fn merge_secrets_removed_in_other() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let other_version = BTreeMap::from([]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BTreeMap::from([]));
    }

    #[test]
    fn merge_secrets_changed_in_current() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value2".to_string())]);
        let other_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([("key1".to_string(), "value2".to_string())])
        );
    }

    #[test]
    fn merge_secrets_changed_in_other() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let other_version = BTreeMap::from([("key1".to_string(), "value2".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([("key1".to_string(), "value2".to_string())])
        );
    }

    #[test]
    fn merge_secrets_changed_in_both() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value2".to_string())]);
        let other_version = BTreeMap::from([("key1".to_string(), "value3".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_secrets_changed_in_current_removed_in_other() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value2".to_string())]);
        let other_version = BTreeMap::from([]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_secrets_changed_in_other_removed_in_current() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([]);
        let other_version = BTreeMap::from([("key1".to_string(), "value2".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }

    #[test]
    fn merge_secrets_added_in_current() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        let other_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string())
            ])
        );
    }

    #[test]
    fn merge_secrets_added_in_other() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let other_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string())
            ])
        );
    }

    #[test]
    fn merge_secrets_added_in_both() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        let other_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            BTreeMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string())
            ])
        );
    }

    #[test]
    fn merge_secrets_added_in_both_with_different_values() {
        let common_ancestor = BTreeMap::from([("key1".to_string(), "value1".to_string())]);
        let current_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2a".to_string()),
        ]);
        let other_version = BTreeMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2b".to_string()),
        ]);
        let result = merge_secrets(&common_ancestor, &current_version, &other_version);
        assert!(result.is_err());
    }
}
