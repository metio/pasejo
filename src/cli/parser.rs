// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{Path, PathBuf, absolute};

use anyhow::Context;
use anyhow::Result;

use crate::models::configuration::Configuration;

pub fn store_name(input: &str) -> Result<String> {
    let configuration = Configuration::cached().context("Could not load configuration")?;
    configuration
        .canonical_store_name(input)
        .ok_or_else(|| anyhow::anyhow!("Store with name '{input}' does not exist in configuration"))
}

pub fn nonzero_isize(input: &str) -> Result<isize> {
    let value: isize = input
        .parse()
        .with_context(|| format!("'{input}' is not a valid line number"))?;
    if value == 0 {
        anyhow::bail!("Line number must not be 0. Use 1 for the first line, -1 for the last")
    }
    Ok(value)
}

pub fn positive_u64(input: &str) -> Result<u64> {
    let value: u64 = input
        .parse()
        .with_context(|| format!("'{input}' is not a valid count"))?;
    if value == 0 {
        anyhow::bail!("Count must not be 0. Use 1 to skip the first line")
    }
    Ok(value)
}

pub fn existing_file(input: &str) -> Result<PathBuf> {
    let path = Path::new(input);
    let absolute_path = absolute(path)?;

    if absolute_path.is_file() {
        Ok(path.to_path_buf())
    } else {
        anyhow::bail!("The file '{input}' does not exist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;

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
