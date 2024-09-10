use std::path::{absolute, PathBuf};

use crate::cli::printer;
use crate::models::configuration::{Configuration, Identity};

pub fn add(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
    global: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.add_identity(identity, store_name, global);
    printer::identity_added();
    result
}

pub fn remove(
    mut configuration: Configuration,
    store_name: &Option<String>,
    identity_file: &PathBuf,
    global: bool,
) -> anyhow::Result<()> {
    let absolute_path = absolute(identity_file)?;
    let identity = Identity {
        file: absolute_path.display().to_string(),
    };
    let result = configuration.remove_identity(&identity, store_name, global);
    printer::identity_removed();
    result
}
