use crate::cli::constants;
use crate::recipients;
use std::fs;
use std::path::{Path, PathBuf};

pub fn secrets_without_recipients_overwrite(directory: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut entries = vec![];
    if directory.is_dir() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                if !entry_path.join(constants::SECRET_FILE_EXTENSION).is_file() {
                    entries.extend(secrets_without_recipients_overwrite(&entry_path)?);
                }
            } else if !recipients::recipient_file_for_secret(&entry_path).is_file() {
                entries.push(entry_path);
            }
        }
    }
    Ok(entries)
}
