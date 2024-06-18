use std::path::PathBuf;

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::cli::configuration::Configuration;
use crate::vcs::api::VCSTypes;

pub(crate) fn store_init(
    path: &PathBuf,
    alias: &String,
    vcs: &VCSTypes,
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
) -> Result<()> {
    file_system.mkdir_parents(path)?;
    vcs.select_implementation().init(path)?;
    configuration.add_store(path.display().to_string(), alias.clone(), vcs.clone())
}
