use std::fs;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use crate::adapters::file_system::FileSystem;

pub struct FileSystemRustStd {}

impl FileSystem for FileSystemRustStd {
    fn mkdir_parents(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to initialize store at {}", path.display()))
    }
}
