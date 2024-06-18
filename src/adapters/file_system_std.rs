use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use walkdir::{IntoIter, WalkDir};

use crate::adapters::file_system::FileSystem;

pub struct FileSystemStd {}

impl FileSystem for FileSystemStd {
    fn mkdir_parents(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directories for path '{}'", path.display()))
    }

    fn reverse_walk(&self, path: &Path) -> IntoIter {
        WalkDir::new(path).contents_first(true).into_iter()
    }

    fn read_file(&self, path: &Path) -> Result<String> {
        let root_recipients_data = fs::read_to_string(path)?;
        Ok(root_recipients_data)
    }

    fn write_file(&self, path: &Path, content: String) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }
}
