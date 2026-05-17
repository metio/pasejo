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
    fn existing_file_rejects_directory() {
        let temp = TempDir::new().unwrap();
        let path_str = temp.path().to_str().unwrap();

        assert!(existing_file(path_str).is_err());
    }
}
