// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use anyhow::Result;

use crate::cli;

pub fn three_way_merge_map<T>(
    common_ancestor: &BTreeMap<String, T>,
    current_version: &BTreeMap<String, T>,
    other_version: &BTreeMap<String, T>,
    value_type: &str,
) -> Result<BTreeMap<String, T>>
where
    T: Clone + PartialEq,
{
    let mut merged_map = BTreeMap::new();
    let mut merge_conflict = false;

    for (key, value) in common_ancestor {
        let current_value = current_version.get(key);
        let other_value = other_version.get(key);

        if let (Some(current_value), Some(other_value)) = (current_value, other_value) {
            if current_value == other_value {
                merged_map.insert(key.clone(), current_value.clone());
            } else if current_value == value {
                merged_map.insert(key.clone(), other_value.clone());
            } else if other_value == value {
                merged_map.insert(key.clone(), current_value.clone());
            } else {
                merge_conflict = true;
                cli::i18n::merge_conflict_values(value_type, key);
            }
        } else if let Some(current_value) = current_value {
            if value != current_value {
                merge_conflict = true;
                cli::i18n::merge_conflict_removed_and_modified(value_type, key);
            }
        } else if let Some(other_value) = other_value
            && value != other_value
        {
            merge_conflict = true;
            cli::i18n::merge_conflict_removed_and_modified(value_type, key);
        }
    }

    for (key, value) in current_version {
        if !common_ancestor.contains_key(key) {
            merged_map.insert(key.clone(), value.clone());
        }
    }

    for (key, value) in other_version {
        if !common_ancestor.contains_key(key) {
            let already_added_value = merged_map.get(key);

            if let Some(already_added_value) = already_added_value {
                if already_added_value != value {
                    merge_conflict = true;
                    cli::i18n::merge_conflict_values(value_type, key);
                }
            } else {
                merged_map.insert(key.clone(), value.clone());
            }
        }
    }

    if merge_conflict {
        Err(anyhow::anyhow!(cli::i18n::error_merge_conflict(value_type)))
    } else {
        Ok(merged_map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, i32)]) -> BTreeMap<String, i32> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), *value))
            .collect()
    }

    /// Run the three-way merge over `i32` maps so the engine is exercised
    /// directly, independent of the secret/OTP/recipient value types that
    /// otherwise re-prove the same matrix three times.
    fn merge(
        ancestor: &[(&str, i32)],
        current: &[(&str, i32)],
        other: &[(&str, i32)],
    ) -> Result<BTreeMap<String, i32>> {
        crate::cli::i18n::init_for_tests();
        three_way_merge_map(&map(ancestor), &map(current), &map(other), "value")
    }

    #[test]
    fn unchanged_on_both_sides_is_kept() {
        let merged = merge(&[("a", 1)], &[("a", 1)], &[("a", 1)]).unwrap();
        assert_eq!(merged, map(&[("a", 1)]));
    }

    #[test]
    fn change_on_one_side_only_takes_that_change() {
        // current edits, other leaves it at the ancestor value → current wins.
        assert_eq!(
            merge(&[("a", 1)], &[("a", 2)], &[("a", 1)]).unwrap(),
            map(&[("a", 2)])
        );
        // symmetric: other edits, current unchanged → other wins.
        assert_eq!(
            merge(&[("a", 1)], &[("a", 1)], &[("a", 3)]).unwrap(),
            map(&[("a", 3)])
        );
    }

    #[test]
    fn same_change_on_both_sides_converges() {
        assert_eq!(
            merge(&[("a", 1)], &[("a", 2)], &[("a", 2)]).unwrap(),
            map(&[("a", 2)])
        );
    }

    #[test]
    fn divergent_change_on_both_sides_is_a_conflict() {
        assert!(merge(&[("a", 1)], &[("a", 2)], &[("a", 3)]).is_err());
    }

    #[test]
    fn removed_on_one_side_and_unchanged_on_the_other_is_removed() {
        assert!(merge(&[("a", 1)], &[], &[("a", 1)]).unwrap().is_empty());
    }

    #[test]
    fn removed_on_one_side_and_modified_on_the_other_is_a_conflict() {
        assert!(merge(&[("a", 1)], &[], &[("a", 2)]).is_err());
    }

    #[test]
    fn added_on_one_side_only_is_included() {
        assert_eq!(merge(&[], &[("new", 9)], &[]).unwrap(), map(&[("new", 9)]));
    }

    #[test]
    fn added_on_both_sides_with_the_same_value_is_included_once() {
        assert_eq!(
            merge(&[], &[("new", 9)], &[("new", 9)]).unwrap(),
            map(&[("new", 9)])
        );
    }

    #[test]
    fn added_on_both_sides_with_different_values_is_a_conflict() {
        assert!(merge(&[], &[("new", 9)], &[("new", 8)]).is_err());
    }

    #[test]
    fn independent_additions_on_each_side_are_unioned() {
        assert_eq!(
            merge(&[], &[("x", 1)], &[("y", 2)]).unwrap(),
            map(&[("x", 1), ("y", 2)])
        );
    }
}
