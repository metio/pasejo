use std::fs;
use std::path;
use std::io::Write;

use anyhow::Context;

pub trait FileSystem {
    fn mkdir_parents(&self, path: &path::Path) -> anyhow::Result<()>;
    fn reverse_walk(&self, path: &path::Path) -> walkdir::IntoIter;
    fn read_file(&self, path: &path::Path) -> anyhow::Result<String>;
    fn write_file(&self, path: &path::Path, content: String) -> anyhow::Result<()>;
    fn append_file(&self, path: &path::Path, content: String) -> anyhow::Result<()>;
}

pub struct FileSystemDefault {}

impl FileSystem for FileSystemDefault {
    fn mkdir_parents(&self, path: &path::Path) -> anyhow::Result<()> {
        fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directories for path '{}'", path.display()))
    }

    fn reverse_walk(&self, path: &path::Path) -> walkdir::IntoIter {
        walkdir::WalkDir::new(path).contents_first(true).into_iter()
    }

    fn read_file(&self, path: &path::Path) -> anyhow::Result<String> {
        let root_recipients_data = fs::read_to_string(path)?;
        Ok(root_recipients_data)
    }

    fn write_file(&self, path: &path::Path, content: String) -> anyhow::Result<()> {
        fs::write(path, content)?;
        Ok(())
    }

    fn append_file(&self, path: &path::Path, content: String) -> anyhow::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(path)?;
        writeln!(file, "{}", content)?;
        Ok(())
    }
}
