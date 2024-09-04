use crate::adapters::file_system::FileSystem;
use crate::adapters::vcs::VersionControlSystems;
use crate::cli::printer;
use crate::models::configuration::Configuration;
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn init(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    path: &PathBuf,
    alias: &String,
    vcs: &VersionControlSystems,
    default: &bool,
) -> Result<()> {
    let canonical_path = file_system.absolute_path(path)?;
    file_system.mkdir_parents(canonical_path.as_path())?;
    vcs.select_implementation().init(canonical_path.as_path())?;
    configuration.add_store(
        canonical_path.display().to_string(),
        alias.clone(),
        vcs.clone(),
    )?;
    printer::store_initialized(canonical_path.display().to_string());
    if *default {
        set_default(configuration, alias)?;
    }
    Ok(())
}

pub fn remove(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    alias: &String,
    remove_data: &bool,
) -> Result<()> {
    let path = configuration.remove_store(alias)?;
    if *remove_data {
        file_system.remove_directory(Path::new(&path))?;
    }
    printer::store_removed(alias.clone());
    Ok(())
}

pub fn set_default(mut configuration: Configuration, alias: &String) -> Result<()> {
    configuration.set_default_store(alias.clone())?;
    printer::store_set_default(alias.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_create_paths() {
        // init(
        //     Box::new(file_system),
        //     Configuration { stores: vec![] },
        //     &PathBuf::from("/some/path"),
        //     &String::from("some-alias"),
        //     &VersionControlSystems::None,
        // )?;
        // assert_eq!(file_system.paths_were_created(), true);
    }
}
