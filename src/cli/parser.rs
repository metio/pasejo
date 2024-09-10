use std::path::{absolute, Path, PathBuf};

use anyhow::Context;

use crate::models::configuration::Configuration;

pub fn store_name(input: &str) -> anyhow::Result<String> {
    let configuration = Configuration::load().context("load configuration")?;
    let names = configuration.all_store_names();

    if names.contains(&input.to_owned()) {
        Ok(input.to_owned())
    } else {
        anyhow::bail!(format!(
            "Store with name '{input}' does not exist in configuration"
        ))
    }
}

pub fn existing_file(input: &str) -> anyhow::Result<PathBuf> {
    let path = Path::new(input);
    let absolute_path = absolute(path)?;

    if absolute_path.is_file() {
        Ok(path.to_path_buf())
    } else {
        anyhow::bail!(format!("The file '{input}' does not exist"))
    }
}
