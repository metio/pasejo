use std::path::PathBuf;

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::adapters::vcs::VersionControlSystems;
use crate::models::configuration::Configuration;

pub fn init(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    path: &PathBuf,
    alias: &String,
    vcs: &VersionControlSystems,
) -> Result<()> {
    file_system.mkdir_parents(path)?;
    vcs.select_implementation().init(path)?;
    configuration.add_store(path.display().to_string(), alias.clone(), vcs.clone())
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
