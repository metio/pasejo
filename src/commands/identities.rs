use crate::adapters::file_system;
use crate::cli::printer;
use crate::models::configuration::{Configuration, Identity};
use std::path::PathBuf;

pub fn add(
    mut configuration: Configuration,
    alias: &Option<String>,
    file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system::absolute_path(file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.add_identity(identity, alias.clone());
    printer::identity_added();
    result
}

pub fn remove(
    mut configuration: Configuration,
    alias: &Option<String>,
    file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system::absolute_path(file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.remove_identity(identity, alias.clone());
    println!("Identity removed");
    result
}
