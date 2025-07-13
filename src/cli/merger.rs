// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli;
use anyhow::Result;
use std::collections::BTreeMap;

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
                cli::logs::merge_conflict_values(value_type, key);
            }
        } else if let Some(current_value) = current_value {
            if value != current_value {
                merge_conflict = true;
                cli::logs::merge_conflict_removed_and_modified(value_type, key);
            }
        } else if let Some(other_value) = other_value {
            if value != other_value {
                merge_conflict = true;
                cli::logs::merge_conflict_removed_and_modified(value_type, key);
            }
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
                    cli::logs::merge_conflict_values(value_type, key);
                }
            } else {
                merged_map.insert(key.clone(), value.clone());
            }
        }
    }

    if merge_conflict {
        Err(anyhow::anyhow!(
            "Merge conflict detected in {value_type}s. Please resolve manually."
        ))
    } else {
        Ok(merged_map)
    }
}
