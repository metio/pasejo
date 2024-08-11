use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use walkdir::WalkDir;

pub trait FileSystem {
    fn mkdir_parents(&self, path: &Path) -> Result<()>;
    fn reverse_walk(&self, path: &Path) -> walkdir::IntoIter;
    fn read_file(&self, path: &Path) -> Result<String>;
    fn write_file(&self, path: &Path, content: String) -> Result<()>;
    fn append_file(&self, path: &Path, content: &String) -> Result<()>;
    fn file_exists(&self, path: &Path) -> Result<bool>;
    fn directory_exists(&self, path: &Path) -> Result<bool>;
}

pub struct FileSystemDefault {}

impl FileSystem for FileSystemDefault {
    fn mkdir_parents(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directories for path '{}'", path.display()))
    }

    fn reverse_walk(&self, path: &Path) -> walkdir::IntoIter {
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

    fn append_file(&self, path: &Path, content: &String) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)?;
        writeln!(file, "{}", content)?;
        Ok(())
    }

    fn file_exists(&self, path: &Path) -> Result<bool> {
        path.try_exists()?;
        Ok(path.is_file())
    }

    fn directory_exists(&self, path: &Path) -> Result<bool> {
        path.try_exists()?;
        Ok(path.is_dir())
    }
}
