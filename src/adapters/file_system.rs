use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub fn append_file(path: &Path, content: &String) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    writeln!(file, "{content}")?;
    Ok(())
}

pub fn append_to_path(path: PathBuf, suffix: &str) -> PathBuf {
    let mut p = path.into_os_string();
    p.push(suffix);
    p.into()
}
