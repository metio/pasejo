use std::path::PathBuf;

use anyhow::Result;

use crate::adapters::file_system;
use crate::cli::configuration;
use crate::vcs::vcs;

pub(crate) fn init(
    file_system: Box<dyn file_system::FileSystem>,
    mut configuration: configuration::Configuration,
    path: &PathBuf,
    alias: &String,
    vcs: &vcs::VersionControlSystems,
) -> Result<()> {
    file_system.mkdir_parents(path)?;
    vcs.select_implementation().init(path)?;
    configuration.add_store(path.display().to_string(), alias.clone(), vcs.clone())
}
