use std::path::PathBuf;

use crate::adapters::file_system;
use crate::cli::printer;
use crate::models::configuration::{Configuration, Identity};

pub fn add(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system::absolute_path(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.add_identity(identity, store_name.clone());
    printer::identity_added();
    result
}

pub fn remove(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
) -> anyhow::Result<()> {
    let absolute_path = file_system::absolute_path(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.remove_identity(&identity, store_name.clone());
    println!("Identity removed");
    result
}
