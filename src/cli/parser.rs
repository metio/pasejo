// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf, absolute};

use anyhow::Context;
use anyhow::Result;

use crate::cli::i18n;
use crate::models::configuration::Configuration;

pub fn store_name(input: &str) -> Result<String> {
    let configuration =
        Configuration::cached().context(i18n::error_could_not_load_configuration())?;
    configuration
        .canonical_store_name(input)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_store_does_not_exist(input)))
}

pub fn nonzero_isize(input: &str) -> Result<isize> {
    let value: isize = input
        .parse()
        .with_context(|| i18n::error_invalid_line_number(input))?;
    if value == 0 {
        anyhow::bail!(i18n::error_line_number_must_not_be_zero())
    }
    Ok(value)
}

pub fn positive_u64(input: &str) -> Result<u64> {
    let value: u64 = input
        .parse()
        .with_context(|| i18n::error_invalid_count(input))?;
    if value == 0 {
        anyhow::bail!(i18n::error_count_must_not_be_zero())
    }
    Ok(value)
}

pub fn existing_file(input: &str) -> Result<PathBuf> {
    let path = Path::new(input);
    let absolute_path = absolute(path)?;

    if absolute_path.is_file() {
        Ok(path.to_path_buf())
    } else {
        anyhow::bail!(i18n::error_file_does_not_exist(input))
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

    use super::*;

    #[test]
    fn existing_file_accepts_real_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("present.txt");
        file.write_str("contents").unwrap();
        let path_str = file.path().to_str().unwrap();

        let result = existing_file(path_str).unwrap();
        assert_eq!(result, std::path::PathBuf::from(path_str));
    }

    #[test]
    fn existing_file_rejects_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.child("does-not-exist.txt");
        let path_str = missing.path().to_str().unwrap();

        assert!(existing_file(path_str).is_err());
    }

    #[test]
    fn nonzero_isize_accepts_positive_and_negative_values() {
        assert_eq!(nonzero_isize("1").unwrap(), 1);
        assert_eq!(nonzero_isize("42").unwrap(), 42);
        assert_eq!(nonzero_isize("-1").unwrap(), -1);
        assert_eq!(nonzero_isize("-100").unwrap(), -100);
    }

    #[test]
    fn nonzero_isize_rejects_zero() {
        i18n::init_for_tests();
        let err = nonzero_isize("0").unwrap_err().to_string();
        assert!(err.contains("must not be 0"));
    }

    #[test]
    fn nonzero_isize_rejects_non_numeric() {
        assert!(nonzero_isize("abc").is_err());
        assert!(nonzero_isize("").is_err());
        assert!(nonzero_isize("1.5").is_err());
    }

    #[test]
    fn existing_file_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let path_str = temp.path().to_str().unwrap();

        assert!(existing_file(path_str).is_err());
    }
}
