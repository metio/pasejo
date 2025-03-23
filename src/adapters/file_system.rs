// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::ffi::OsStr;
use std::fs::DirEntry;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{fs, io};

use anyhow::{Context, Result};
use inquire::Confirm;
use termtree::Tree;

use crate::cli::logs;

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

pub fn replace_file_content(
    path: &Path,
    content: &String,
    force: bool,
    confirm_message: &str,
    help_message: &str,
) -> Result<()> {
    if path.is_file() {
        if force {
            fs::remove_file(path)?;
            append_file(path, content)?;
            logs::recipients_file_replaced(path);
        } else {
            let replace_recipients = Confirm::new(confirm_message)
                .with_default(false)
                .with_help_message(help_message)
                .prompt()
                .context("Could not get user answer")?;
            if replace_recipients {
                fs::remove_file(path)?;
                append_file(path, content)?;
                logs::recipients_file_replaced(path);
            } else {
                logs::recipients_file_use_existing(path);
            }
        }
    } else {
        append_file(path, content)?;
        logs::recipients_file_created(path);
    }
    Ok(())
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

pub fn file_list<P: AsRef<Path>>(
    parent_name: &str,
    path: P,
    file_extension: &str,
) -> io::Result<Vec<String>> {
    let mut entries: Vec<String> = vec![];

    if path.as_ref().is_dir() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                entries.extend(file_list(
                    &entry_path
                        .file_name()
                        .and_then(OsStr::to_str)
                        .map(std::borrow::ToOwned::to_owned)
                        .map_or_else(
                            || parent_name.to_owned(),
                            |path_to_dir| format!("{parent_name}/{path_to_dir}"),
                        ),
                    &entry_path,
                    file_extension,
                )?);
            } else if has_file_extension(&entry_path, file_extension) {
                if let Some(stem) = entry_path
                    .file_stem()
                    .and_then(OsStr::to_str)
                    .map(std::borrow::ToOwned::to_owned)
                    .map(|path_to_secret| format!("{parent_name}/{path_to_secret}"))
                {
                    entries.push(stem);
                }
            }
        }
    }

    entries.sort();
    Ok(entries)
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
        .is_some_and(|ext| ext.eq_ignore_ascii_case(file_extension))
}
