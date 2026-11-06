// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! A local, append-only record of every credential the browser host hands
//! to an extension. It names what was released where, never the secret
//! material itself.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::{data_dir, i18n};

const FILE_NAME: &str = "browser-releases.jsonl";

/// Once the log would grow past this size it is moved aside to `<log>.1`,
/// replacing an older rotation, so the two files together stay bounded.
const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// UTC timestamp in RFC 3339 form.
    pub time: String,
    pub domain: String,
    pub store: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caller: Option<String>,
}

impl Release {
    pub fn now(domain: &str, store: &str, name: &str, caller: Option<&str>) -> Self {
        let seconds = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        Self {
            time: rfc3339_utc(seconds),
            domain: domain.to_owned(),
            store: store.to_owned(),
            name: name.to_owned(),
            caller: caller.map(str::to_owned),
        }
    }
}

/// The entries of the log, oldest first, and how many lines could not be
/// read as an entry.
#[derive(Debug, Default)]
pub struct Entries {
    pub releases: Vec<Release>,
    pub unreadable_lines: usize,
}

#[derive(Debug, Clone)]
pub struct ReleaseLog {
    path: PathBuf,
    max_bytes: u64,
}

impl ReleaseLog {
    pub const fn new(path: PathBuf) -> Self {
        Self {
            path,
            max_bytes: MAX_LOG_BYTES,
        }
    }

    /// The log in pasejo's local data directory.
    pub fn in_data_dir() -> Option<Self> {
        data_dir::local_data_dir().map(|dir| Self::new(dir.join(FILE_NAME)))
    }

    #[cfg(test)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn rotated_path(&self) -> PathBuf {
        suffixed(&self.path, ".1")
    }

    fn lock_path(&self) -> PathBuf {
        suffixed(&self.path, ".lock")
    }

    /// Appends one entry. Concurrent writers, whether worker threads of one
    /// host or several hosts started by different browsers, serialise on an
    /// advisory lock of a sibling file, so rotation and append cannot
    /// interleave.
    pub fn append(&self, release: &Release) -> Result<()> {
        let display = self.path.display().to_string();
        if let Some(parent) = self.path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| {
                i18n::error_failed_to_create_directory(&parent.display().to_string())
            })?;
        }
        let lock = open_private(&self.lock_path(), false)
            .with_context(|| i18n::error_browser_release_log_lock(&display))?;
        lock.lock()
            .with_context(|| i18n::error_browser_release_log_lock(&display))?;

        let mut line = serde_json::to_vec(release)?;
        line.push(b'\n');
        let current = match fs::metadata(&self.path) {
            Ok(metadata) => metadata.len(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(error) => {
                return Err(error).with_context(|| i18n::error_failed_to_write_file(&display));
            }
        };
        let line_len = u64::try_from(line.len()).unwrap_or(u64::MAX);
        if current > 0 && current.saturating_add(line_len) > self.max_bytes {
            fs::rename(&self.path, self.rotated_path())
                .with_context(|| i18n::error_failed_to_write_file(&display))?;
        }
        let mut file = open_private(&self.path, true)
            .with_context(|| i18n::error_failed_to_write_file(&display))?;
        file.write_all(&line)
            .and_then(|()| file.sync_all())
            .with_context(|| i18n::error_failed_to_write_file(&display))?;
        drop(lock);
        Ok(())
    }

    /// Reads the rotated file, then the current one. Missing files count as
    /// empty.
    pub fn read(&self) -> Result<Entries> {
        let mut entries = Entries::default();
        for path in [self.rotated_path(), self.path.clone()] {
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        i18n::error_cannot_read_file(&path.display().to_string())
                    });
                }
            };
            for line in content.lines().filter(|line| !line.trim().is_empty()) {
                match serde_json::from_str::<Release>(line) {
                    Ok(release) => entries.releases.push(release),
                    Err(_) => entries.unreadable_lines = entries.unreadable_lines.saturating_add(1),
                }
            }
        }
        Ok(entries)
    }
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Opens `path` for writing, creating it readable by the owner only on Unix.
fn open_private(path: &Path, append: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.create(true);
    if append {
        options.append(true);
    } else {
        options.write(true).truncate(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Formats seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`, using the
/// days-to-civil-date conversion of the proleptic Gregorian calendar.
fn rfc3339_utc(seconds: u64) -> String {
    let days = seconds / 86_400;
    let of_day = seconds % 86_400;
    let (hour, minute, second) = (of_day / 3_600, of_day % 3_600 / 60, of_day % 60);

    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(name: &str) -> Release {
        Release {
            time: "2025-11-06T12:00:00Z".into(),
            domain: "example.com".into(),
            store: "personal".into(),
            name: name.into(),
            caller: Some("chrome-extension://abcdefgh/".into()),
        }
    }

    #[test]
    fn rfc3339_utc_formats_known_instants() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339_utc(1_709_210_096), "2024-02-29T12:34:56Z");
        assert_eq!(rfc3339_utc(1_762_430_400), "2025-11-06T12:00:00Z");
        assert_eq!(rfc3339_utc(4_102_444_799), "2099-12-31T23:59:59Z");
    }

    #[test]
    fn appended_releases_read_back_in_order() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let log = ReleaseLog::new(temp.path().join("nested").join(FILE_NAME));
        log.append(&release("alice")).unwrap();
        log.append(&release("bob")).unwrap();
        let entries = log.read().unwrap();
        assert_eq!(entries.releases, vec![release("alice"), release("bob")]);
        assert_eq!(entries.unreadable_lines, 0);
    }

    #[test]
    fn each_release_is_one_json_line_without_an_unknown_caller() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let log = ReleaseLog::new(temp.path().join(FILE_NAME));
        let mut anonymous = release("alice");
        anonymous.caller = None;
        log.append(&anonymous).unwrap();
        let content = fs::read_to_string(log.path()).unwrap();
        assert_eq!(
            content,
            "{\"time\":\"2025-11-06T12:00:00Z\",\"domain\":\"example.com\",\"store\":\"personal\",\"name\":\"alice\"}\n"
        );
    }

    #[test]
    fn a_missing_log_reads_as_empty() {
        let temp = assert_fs::TempDir::new().unwrap();
        let entries = ReleaseLog::new(temp.path().join(FILE_NAME)).read().unwrap();
        assert!(entries.releases.is_empty());
        assert_eq!(entries.unreadable_lines, 0);
    }

    #[test]
    fn unreadable_lines_are_counted_and_skipped() {
        let temp = assert_fs::TempDir::new().unwrap();
        let log = ReleaseLog::new(temp.path().join(FILE_NAME));
        let valid = serde_json::to_string(&release("alice")).unwrap();
        fs::write(log.path(), format!("{{\"time\":\n{valid}\nnot json\n")).unwrap();
        let entries = log.read().unwrap();
        assert_eq!(entries.releases, vec![release("alice")]);
        assert_eq!(entries.unreadable_lines, 2);
    }

    #[test]
    fn a_full_log_is_rotated_and_both_files_are_read() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let line_len = serde_json::to_vec(&release("alice")).unwrap().len() as u64 + 1;
        let log = ReleaseLog {
            path: temp.path().join(FILE_NAME),
            max_bytes: line_len * 2,
        };
        for name in ["alice", "bob", "carol"] {
            log.append(&release(name)).unwrap();
        }
        let rotated = fs::read_to_string(log.rotated_path()).unwrap();
        assert_eq!(rotated.lines().count(), 2);
        let current = fs::read_to_string(log.path()).unwrap();
        assert_eq!(current.lines().count(), 1);
        let names: Vec<String> = log
            .read()
            .unwrap()
            .releases
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, ["alice", "bob", "carol"]);
    }

    #[test]
    fn a_second_rotation_keeps_the_size_bounded() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let line_len = serde_json::to_vec(&release("alice")).unwrap().len() as u64 + 1;
        let log = ReleaseLog {
            path: temp.path().join(FILE_NAME),
            max_bytes: line_len,
        };
        for name in ["alice", "bob", "carol"] {
            log.append(&release(name)).unwrap();
        }
        let names: Vec<String> = log
            .read()
            .unwrap()
            .releases
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, ["bob", "carol"]);
    }

    #[cfg(unix)]
    #[test]
    fn the_log_is_readable_by_its_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let log = ReleaseLog::new(temp.path().join(FILE_NAME));
        log.append(&release("alice")).unwrap();
        let mode = fs::metadata(log.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn concurrent_appends_keep_every_line_whole() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let log = ReleaseLog::new(temp.path().join(FILE_NAME));
        std::thread::scope(|scope| {
            for worker in 0..8 {
                let log = &log;
                scope.spawn(move || {
                    for index in 0..20 {
                        log.append(&release(&format!("w{worker}-{index}"))).unwrap();
                    }
                });
            }
        });
        let entries = log.read().unwrap();
        assert_eq!(entries.releases.len(), 160);
        assert_eq!(entries.unreadable_lines, 0);
    }

    #[test]
    fn appending_fails_when_the_log_path_is_a_directory() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let path = temp.path().join(FILE_NAME);
        fs::create_dir(&path).unwrap();
        assert!(ReleaseLog::new(path).append(&release("alice")).is_err());
    }
}
