use anyhow::Context;
use anyhow::Result;
use std::fs;
use std::io::Write;
use std::path::{absolute, Path, PathBuf};

pub fn absolute_path(path: &PathBuf) -> Result<PathBuf> {
    let path = absolute(path)?;
    Ok(path)
}

pub fn mkdir_parents(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directories for path '{}'", path.display()))
}

pub fn read_file(path: &Path) -> Result<String> {
    let root_recipients_data = fs::read_to_string(path)?;
    Ok(root_recipients_data)
}

pub fn write_file(path: &Path, content: String) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn append_file(path: &Path, content: &String) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    writeln!(file, "{}", content)?;
    Ok(())
}

pub fn file_exists(path: &Path) -> Result<bool> {
    path.try_exists()?;
    Ok(path.is_file())
}

pub fn directory_exists(path: &Path) -> Result<bool> {
    path.try_exists()?;
    Ok(path.is_dir())
}

pub fn remove_directory(path: &Path) -> Result<()> {
    fs::remove_dir(path)?;
    Ok(())
}

pub fn remove_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn append_to_path(path: PathBuf, suffix: &str) -> PathBuf {
    let mut p = path.into_os_string();
    p.push(suffix);
    p.into()
}
