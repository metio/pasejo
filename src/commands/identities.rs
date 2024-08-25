use crate::adapters::file_system::FileSystem;
use crate::models::configuration::{Configuration, Identity};
use std::path::PathBuf;

pub fn add(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    alias: &Option<String>,
    file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system.absolute_path(file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.add_identity(identity, alias.clone());
    println!("Identity added");
    result
}

pub fn remove(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    alias: &Option<String>,
    file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system.absolute_path(file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.remove_identity(identity, alias.clone());
    println!("Identity removed");
    result
}
