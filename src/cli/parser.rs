// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::{absolute, Path, PathBuf};

use anyhow::Context;
use anyhow::Result;

use crate::models::configuration::Configuration;

pub fn store_name(input: &str) -> Result<String> {
    let configuration =
        Configuration::load_configuration().context("Could not load configuration")?;
    let names = configuration.all_store_names();

    if names.contains(&input.to_owned()) {
        Ok(input.to_owned())
    } else {
        anyhow::bail!("Store with name '{input}' does not exist in configuration")
    }
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
