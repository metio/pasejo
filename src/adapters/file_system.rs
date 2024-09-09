use std::ffi::OsStr;
use std::fs::DirEntry;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::Result;
use termtree::Tree;

pub fn append_file(path: &Path, content: &String) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    writeln!(file, "{content}")?;
    Ok(())
}

pub fn append_file_extension(path: PathBuf, file_extension: &str) -> PathBuf {
    let mut p = path.into_os_string();
    if !file_extension.starts_with('.') {
        p.push(".");
    }
    p.push(file_extension);
    p.into()
}

pub fn file_tree<P: AsRef<Path>>(
    root_folder_name: String,
    path: P,
    file_extension: &str,
) -> io::Result<Tree<String>> {
    let mut entries: Vec<DirEntry> = fs::read_dir(&path)?.filter_map(Result::ok).collect();
    entries.sort_by_key(DirEntry::path);
    let result = entries
        .iter()
        .fold(Tree::new(root_folder_name), |mut root, entry| {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    if let Ok(subtree) =
                        file_tree(file_name(entry.path()), entry.path(), file_extension)
                    {
                        root.push(subtree);
                    }
                } else if has_file_extension(entry.path(), file_extension) {
                    if let Some(stem) = entry.path().file_stem().and_then(OsStr::to_str) {
                        root.push(Tree::new(String::from(stem)));
                    }
                }
            }
            root
        });
    Ok(result)
}

/// Extracts the file name of a path. The file name is defined as the last
/// segment of a path.
///
/// # Examples
///
/// ```
/// use crate::adapters::file_system;
///
/// assert_eq!("bar.txt", file_system::file_name(Path::new("foo/bar.txt")));
/// ```
pub fn file_name<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .file_name()
        .and_then(OsStr::to_str)
        .map_or_else(String::new, String::from)
}

/// Checks whether a given path has a specific file extension
///
/// # Examples
///
/// ```
/// use crate::adapters::file_system;
///
/// assert_eq!(true, file_system::has_file_extension(Path::new("name.recipients"), "recipients"));
/// assert_eq!(true, file_system::has_file_extension(Path::new("some.age"), "age"));
/// assert_eq!(false, file_system::has_file_extension(PathBuf::from("some.other"), "age"));
/// ```
pub fn has_file_extension<P: AsRef<Path>>(path: P, file_extension: &str) -> bool {
    path.as_ref()
        .extension()
        .map_or(false, |ext| ext.eq_ignore_ascii_case(file_extension))
}
