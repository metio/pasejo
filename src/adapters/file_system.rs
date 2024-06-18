use std::path::Path;

use anyhow::Result;
use walkdir::IntoIter;

pub trait FileSystem {
    fn mkdir_parents(&self, path: &Path) -> Result<()>;
    fn reverse_walk(&self, path: &Path) -> IntoIter;
    fn read_file(&self, path: &Path) -> Result<String>;
    fn write_file(&self, path: &Path, content: String) -> Result<()>;
}
