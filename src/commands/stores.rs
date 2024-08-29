use std::path::PathBuf;

use anyhow::Result;

use crate::adapters::file_system::FileSystem;
use crate::adapters::vcs::VersionControlSystems;
use crate::cli::printer;
use crate::models::configuration::Configuration;

pub fn init(
    file_system: Box<dyn FileSystem>,
    mut configuration: Configuration,
    path: &PathBuf,
    alias: &String,
    vcs: &VersionControlSystems,
) -> Result<()> {
    let canonical_path = file_system.absolute_path(path)?;
    file_system.mkdir_parents(canonical_path.as_path())?;
    vcs.select_implementation().init(canonical_path.as_path())?;
    let result = configuration.add_store(
        canonical_path.display().to_string(),
        alias.clone(),
        vcs.clone(),
    );
    printer::store_initialized(canonical_path.display().to_string());
    result
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
