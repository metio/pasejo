// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use crate::cli;
use crate::models::password_store::OneTimePassword;
use anyhow::Result;
use std::collections::BTreeMap;

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

    #[test]
    fn merge_one_time_passwords_with_empty_maps() {
        let common_ancestor = BTreeMap::new();
        let current_version = BTreeMap::new();
        let other_version = BTreeMap::new();
        let result = merge_one_time_passwords(&common_ancestor, &current_version, &other_version);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BTreeMap::new());
    }
}
