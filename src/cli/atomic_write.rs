// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Write `contents` to `path` atomically. On Unix the resulting file is
/// created with mode `0600` so secrets and configuration material are
/// readable only by the current user.
///
/// The implementation writes to a sibling temp file in the same directory,
/// fsyncs, then renames over the destination. A crash before the rename
/// leaves the original file untouched. If anything fails after the temp
/// file is created, the temp file is removed best-effort.
pub fn write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("Cannot determine parent directory of {}", path.display()))?;

    if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }

    let tmp = temp_path(path);
    let result = (|| -> Result<()> {
        let mut file = open_temp(&tmp)?;
        file.write_all(contents)
            .with_context(|| format!("Failed to write {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to fsync {}", tmp.display()))?;
        drop(file);
        fs::rename(&tmp, path)
            .with_context(|| format!("Failed to rename {} to {}", tmp.display(), path.display()))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn temp_path(path: &Path) -> PathBuf {
    // Uniqueness comes from three independent sources, layered defensively:
    //   * `process::id()` disambiguates concurrent pasejo invocations
    //     writing to the same directory.
    //   * `SystemTime::now()` (nanos since the epoch) disambiguates
    //     sequential calls in different moments. Falls back to 0 if the
    //     clock is before the epoch — implausible on a real system, but
    //     the counter below still keeps things unique if it happens.
    //   * An atomic counter disambiguates rapid calls within a single
    //     process where the clock hasn't ticked.
    // `OpenOptions::create_new(true)` in `open_temp` is the actual
    // safety net: if the temp path *did* collide, the open fails rather
    // than overwriting. The pieces above just keep that path unlikely.
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);

    let mut name = path
        .file_name()
        .map_or_else(OsString::new, std::ffi::OsStr::to_os_string);
    name.push(format!(".tmp.{pid}.{nanos}.{counter}"));
    let mut tmp = path.to_path_buf();
    tmp.set_file_name(name);
    tmp
}

#[cfg(unix)]
fn open_temp(tmp: &Path) -> Result<fs::File> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(tmp)
        .with_context(|| format!("Failed to create {}", tmp.display()))
}

#[cfg(not(unix))]
fn open_temp(tmp: &Path) -> Result<fs::File> {
    use std::fs::OpenOptions;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp)
        .with_context(|| format!("Failed to create {}", tmp.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    #[test]
    fn write_creates_file_with_contents() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.bin");
        write(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn write_replaces_existing_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.bin");
        fs::write(&path, b"old").unwrap();
        write(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn write_creates_missing_parent_directory() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested/sub/data.bin");
        write(&path, b"hello").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello");
    }

    #[cfg(unix)]
    #[test]
    fn write_sets_file_mode_to_0600_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.bin");
        write(&path, b"secret").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn write_does_not_leave_temp_file_on_success() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("data.bin");
        write(&path, b"hello").unwrap();
        let leftovers: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
    }
}
