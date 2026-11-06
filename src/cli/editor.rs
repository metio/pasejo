// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use zeroize::Zeroizing;

use crate::cli::i18n;

const SHARED_MEMORY_DIR: &str = "/dev/shm";

/// Opens `initial` in `$VISUAL` / `$EDITOR` and returns the edited text
/// without trailing line breaks. The text is written to a file that is
/// overwritten with zeros and removed before this returns, whether the edit
/// succeeded or not.
pub fn edit(initial: &str) -> anyhow::Result<Zeroizing<String>> {
    let directory = match select_directory(
        env::var_os("XDG_RUNTIME_DIR"),
        Path::new(SHARED_MEMORY_DIR),
        env::temp_dir(),
    ) {
        EditorDirectory::Private(directory) => directory,
        EditorDirectory::Temporary(directory) => {
            i18n::editor_file_in_temp_dir(&directory);
            directory
        }
    };
    let file = EditorFile::create_in(&directory, initial)?;
    let editor = editor_command(env::var_os("VISUAL"), env::var_os("EDITOR"));
    run_editor(&editor, file.path())?;
    file.read()
}

#[derive(Debug, PartialEq, Eq)]
enum EditorDirectory {
    /// A per-user or RAM-backed directory.
    Private(PathBuf),
    /// The system temp directory, which may be backed by persistent storage.
    Temporary(PathBuf),
}

fn select_directory(
    runtime_dir: Option<OsString>,
    shared_memory: &Path,
    temp_dir: PathBuf,
) -> EditorDirectory {
    runtime_dir
        .filter(|directory| !directory.is_empty())
        .map(PathBuf::from)
        .filter(|directory| directory.is_dir())
        .or_else(|| shared_memory.is_dir().then(|| shared_memory.to_path_buf()))
        .map_or(
            EditorDirectory::Temporary(temp_dir),
            EditorDirectory::Private,
        )
}

/// Resolves the editor like `inquire` does: a non-empty `$VISUAL` wins over a
/// non-empty `$EDITOR`, and the platform default applies when neither is set.
/// The value names the program itself and is not split into arguments.
fn editor_command(visual: Option<OsString>, editor: Option<OsString>) -> OsString {
    visual
        .filter(|command| !command.is_empty())
        .or_else(|| editor.filter(|command| !command.is_empty()))
        .unwrap_or_else(|| OsString::from(if cfg!(windows) { "notepad" } else { "nano" }))
}

fn run_editor(editor: &OsStr, path: &Path) -> anyhow::Result<()> {
    let name = editor.to_string_lossy();
    let status = Command::new(editor)
        .arg(path)
        .status()
        .with_context(|| i18n::error_editor_launch_failed(&name))?;
    if !status.success() {
        anyhow::bail!(i18n::error_editor_exited_unsuccessfully(&name));
    }
    Ok(())
}

/// The file handed to the editor. Dropping it overwrites the file's current
/// content with zeros and removes it.
struct EditorFile {
    path: PathBuf,
}

impl EditorFile {
    fn create_in(directory: &Path, initial: &str) -> anyhow::Result<Self> {
        let path = directory.join(format!("pasejo-{}.txt", uuid::Uuid::now_v7().simple()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .with_context(|| i18n::error_editor_file_create_failed(directory))?;
        let file_guard = Self { path };
        file.write_all(initial.as_bytes())
            .and_then(|()| file.flush())
            .with_context(|| i18n::error_editor_file_create_failed(directory))?;
        Ok(file_guard)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> anyhow::Result<Zeroizing<String>> {
        let mut bytes = Zeroizing::new(Vec::new());
        fs::File::open(&self.path)?.read_to_end(&mut bytes)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| anyhow::anyhow!(i18n::error_editor_file_not_utf8()))?;
        Ok(Zeroizing::new(
            text.trim_end_matches(['\n', '\r']).to_owned(),
        ))
    }
}

impl Drop for EditorFile {
    fn drop(&mut self) {
        let result = scrub(&self.path).and_then(|()| fs::remove_file(&self.path));
        if let Err(error) = result
            && error.kind() != io::ErrorKind::NotFound
        {
            i18n::editor_file_cleanup_failed(&self.path, &error);
        }
    }
}

/// Overwrites the file at `path` with zeros. Editors that save by writing a
/// new file and renaming it over the original leave the previous content in
/// an inode that is no longer reachable by path; only the current one is
/// overwritten.
fn scrub(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(path)?;
    let length = file.metadata()?.len();
    io::copy(&mut io::repeat(0).take(length), &mut file)?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        PathBuf::from("/some/temp")
    }

    #[test]
    fn select_directory_prefers_the_runtime_dir() {
        let runtime = env::temp_dir();
        assert_eq!(
            select_directory(
                Some(runtime.clone().into_os_string()),
                Path::new("/does/not/exist"),
                temp_dir()
            ),
            EditorDirectory::Private(runtime)
        );
    }

    #[test]
    fn select_directory_falls_back_to_shared_memory() {
        let shared_memory = env::temp_dir();
        assert_eq!(
            select_directory(
                Some(OsString::from("/does/not/exist")),
                &shared_memory,
                temp_dir()
            ),
            EditorDirectory::Private(shared_memory.clone())
        );
        assert_eq!(
            select_directory(Some(OsString::new()), &shared_memory, temp_dir()),
            EditorDirectory::Private(shared_memory)
        );
    }

    #[test]
    fn select_directory_falls_back_to_the_temp_dir() {
        assert_eq!(
            select_directory(None, Path::new("/does/not/exist"), temp_dir()),
            EditorDirectory::Temporary(temp_dir())
        );
    }

    #[test]
    fn editor_command_prefers_visual_over_editor() {
        assert_eq!(
            editor_command(Some("vis".into()), Some("ed".into())),
            OsString::from("vis")
        );
        assert_eq!(
            editor_command(Some(OsString::new()), Some("ed".into())),
            OsString::from("ed")
        );
        assert_eq!(
            editor_command(None, Some(OsString::new())),
            OsString::from(if cfg!(windows) { "notepad" } else { "nano" })
        );
    }

    #[cfg(unix)]
    #[test]
    fn editor_file_is_owner_only_and_removed_on_drop() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let directory = env::temp_dir().join(format!("pasejo-editor-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&directory)?;
        let file = EditorFile::create_in(&directory, "secret\n\n")?;
        let path = file.path().to_path_buf();
        assert_eq!(fs::metadata(&path)?.permissions().mode() & 0o777, 0o600);
        assert_eq!(file.read()?.as_str(), "secret");

        let link = directory.join("link");
        fs::hard_link(&path, &link)?;
        drop(file);
        assert!(!path.exists());
        assert_eq!(fs::read(&link)?, vec![0; "secret\n\n".len()]);
        fs::remove_dir_all(&directory)?;
        Ok(())
    }

    #[test]
    fn editor_file_rejects_invalid_utf8() -> anyhow::Result<()> {
        crate::cli::i18n::init_for_tests();
        let directory = env::temp_dir().join(format!("pasejo-editor-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&directory)?;
        let file = EditorFile::create_in(&directory, "")?;
        fs::write(file.path(), [0xff, 0xfe])?;
        assert!(file.read().is_err());
        drop(file);
        fs::remove_dir_all(&directory)?;
        Ok(())
    }
}
