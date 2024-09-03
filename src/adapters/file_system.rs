use anyhow::Context;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::{absolute, Path, PathBuf};

pub trait FileSystem {
    fn absolute_path(&self, path: &PathBuf) -> Result<PathBuf>;
    fn mkdir_parents(&self, path: &Path) -> Result<()>;
    fn read_file(&self, path: &Path) -> Result<String>;
    fn write_file(&self, path: &Path, content: String) -> Result<()>;
    fn append_file(&self, path: &Path, content: &String) -> Result<()>;
    fn file_exists(&self, path: &Path) -> Result<bool>;
    fn directory_exists(&self, path: &Path) -> Result<bool>;
    fn remove_directory(&self, path: &Path) -> Result<()>;
}

pub struct FileSystemDefault {}

impl FileSystemDefault {
    pub(crate) fn new() -> Box<dyn FileSystem> {
        Box::new(FileSystemDefault {})
    }
}

impl FileSystem for FileSystemDefault {
    fn absolute_path(&self, path: &PathBuf) -> Result<PathBuf> {
        let path = absolute(path)?;
        Ok(path)
    }

    fn mkdir_parents(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directories for path '{}'", path.display()))
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

    fn remove_directory(&self, path: &Path) -> Result<()> {
        fs::remove_dir(path)?;
        Ok(())
    }
}
