use std::fs;
use std::path::{absolute, Path, PathBuf};

use anyhow::Result;

use crate::adapters::vcs::VersionControlSystems;
use crate::cli::logs;
use crate::models::configuration::Configuration;

pub fn init(
    mut configuration: Configuration,
    store_root_path: &PathBuf,
    store_name: &str,
    vcs: &VersionControlSystems,
    default: bool,
) -> Result<()> {
    let canonical_path = absolute(store_root_path)?;
    fs::create_dir_all(canonical_path.as_path())?;
    vcs.select_implementation().init(canonical_path.as_path())?;
    configuration.add_store(
        canonical_path.display().to_string(),
        store_name,
        vcs.clone(),
    )?;
    logs::store_initialized(&canonical_path.display().to_string());
    if default {
        set_default(configuration, store_name)?;
    }
    Ok(())
}

pub fn remove(mut configuration: Configuration, store_name: &str, remove_data: bool) -> Result<()> {
    let path = configuration.remove_store(store_name)?;
    if remove_data {
        fs::remove_dir(Path::new(&path))?;
    }
    logs::store_removed(store_name);
    Ok(())
}

pub fn set_default(mut configuration: Configuration, store_name: &str) -> Result<()> {
    configuration.set_default_store(store_name)?;
    logs::store_set_default(store_name);
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
        //     &String::from("some-name"),
        //     &VersionControlSystems::None,
        // )?;
        // assert_eq!(file_system.paths_were_created(), true);
    }
}
