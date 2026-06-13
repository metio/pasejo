// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::cell::LazyCell;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use age::cli_common::{StdinGuard, read_identities};

pub fn read(
    identity_files: Vec<PathBuf>,
    ignore_missing_identities: bool,
) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    let existing_identities = if ignore_missing_identities {
        let age_plugin_yubikey_output: LazyCell<io::Result<String>> = LazyCell::new(|| {
            let Some(plugin_binary) = find_in_path("age-plugin-yubikey") else {
                return Ok(String::new());
            };
            let output = Command::new(&plugin_binary)
                .arg("--identity")
                .stderr(Stdio::null())
                .output()?;
            if !output.status.success() {
                return Err(io::Error::other(format!(
                    "age-plugin-yubikey exited with {}",
                    output
                        .status
                        .code()
                        .map_or_else(|| String::from("signal"), |c| c.to_string())
                )));
            }
            String::from_utf8(output.stdout)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
        });

        let mut files = vec![];
        for file in identity_files {
            match std::fs::read_to_string(&file) {
                Ok(content) => {
                    if content.contains("AGE-PLUGIN-YUBIKEY-") {
                        // A YubiKey identity file is only usable if the device
                        // it names is currently plugged in — keep it when the
                        // plugin's `--identity` output echoes one of its
                        // device tags, otherwise skip it so a locked/absent
                        // key doesn't block decryption with the others.
                        let connected = (*age_plugin_yubikey_output)
                            .as_ref()
                            .is_ok_and(|output| file_has_matching_device(&content, output));
                        if connected {
                            files.push(file);
                        } else {
                            log::debug!(
                                "Skipping yubikey identity file with no matching device: {}",
                                file.display()
                            );
                        }
                    } else {
                        files.push(file);
                    }
                }
                Err(error) => {
                    log::debug!("Skipping missing identity file {}: {error}", file.display());
                }
            }
        }
        files
    } else {
        identity_files
    };

    let filenames = existing_identities
        .iter()
        .map(|file| format!("{}", file.display()))
        .collect();

    Ok(read_identities(
        filenames,
        None,
        &mut StdinGuard::new(true),
    )?)
}

/// The device-identifying suffix of an `AGE-PLUGIN-YUBIKEY-…` identity line:
/// its final `-`-separated segment, which the plugin's `--identity` output
/// echoes for a connected device. `None` for any line that is not a yubikey
/// identity line.
fn yubikey_device_tag(line: &str) -> Option<&str> {
    if line.starts_with("AGE-PLUGIN-YUBIKEY-") {
        line.rsplit('-').next()
    } else {
        None
    }
}

/// Whether any `AGE-PLUGIN-YUBIKEY-…` line in `content` names a device tag
/// present in the plugin's `--identity` `output` — i.e. the yubikey backing
/// this identity file is currently connected.
fn file_has_matching_device(content: &str, output: &str) -> bool {
    content
        .lines()
        .filter_map(yubikey_device_tag)
        .any(|tag| output.contains(tag))
}

/// Find an executable by name on `PATH`. Replacement for `which::which`
/// scoped to the one place we need it (the `age-plugin-yubikey` lookup
/// above). Returns the first match, or `None` if `PATH` is unset or no
/// executable file with that name exists in any entry.
fn find_in_path(name: &str) -> Option<PathBuf> {
    find_in_paths(name, &std::env::var_os("PATH")?)
}

/// The pure core of [`find_in_path`], taking the search path explicitly so
/// the lookup can be unit-tested against a constructed directory list
/// without mutating the process-global `PATH`.
fn find_in_paths(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    for dir in std::env::split_paths(path_var) {
        let candidate = dir.join(name);
        if is_executable_file(&candidate) {
            return Some(candidate);
        }
        // On Windows the binary may carry one of the PATHEXT extensions.
        // Try the common ones; we don't parse PATHEXT itself because the
        // binary we look up (`age-plugin-yubikey`) is shipped as `.exe`.
        #[cfg(windows)]
        for ext in ["exe", "cmd", "bat", "com"] {
            let with_ext = dir.join(format!("{name}.{ext}"));
            if with_ext.is_file() {
                return Some(with_ext);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yubikey_device_tag_extracts_the_final_segment() {
        assert_eq!(
            yubikey_device_tag("AGE-PLUGIN-YUBIKEY-ABC123"),
            Some("ABC123")
        );
    }

    #[test]
    fn yubikey_device_tag_ignores_non_yubikey_lines() {
        assert_eq!(yubikey_device_tag("AGE-SECRET-KEY-1XYZ"), None);
        assert_eq!(yubikey_device_tag("# a comment"), None);
        assert_eq!(yubikey_device_tag(""), None);
    }

    #[test]
    fn file_has_matching_device_is_true_when_output_echoes_a_tag() {
        let content = "# created by age-plugin-yubikey\nAGE-PLUGIN-YUBIKEY-ABC123\n";
        assert!(file_has_matching_device(
            content,
            "Serial 12345678 (slot ABC123)"
        ));
    }

    #[test]
    fn file_has_matching_device_is_false_when_no_tag_matches_or_output_is_empty() {
        let content = "AGE-PLUGIN-YUBIKEY-ABC123\n";
        assert!(!file_has_matching_device(content, "Serial DEF456"));
        assert!(!file_has_matching_device(content, ""));
    }

    #[test]
    fn file_has_matching_device_is_false_for_content_without_a_yubikey_line() {
        assert!(!file_has_matching_device("age1abc\n", "ABC123"));
    }

    #[cfg(unix)]
    mod executable_lookup {
        use std::os::unix::fs::PermissionsExt;

        use assert_fs::TempDir;
        use assert_fs::prelude::*;

        use super::super::{find_in_paths, is_executable_file};

        fn write_mode(temp: &TempDir, name: &str, mode: u32) -> std::path::PathBuf {
            let file = temp.child(name);
            file.write_str("#!/bin/sh\n").unwrap();
            std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(mode)).unwrap();
            file.path().to_path_buf()
        }

        #[test]
        fn is_executable_file_requires_an_execute_bit() {
            let temp = TempDir::new().unwrap();
            assert!(is_executable_file(&write_mode(&temp, "runnable", 0o755)));
            assert!(!is_executable_file(&write_mode(&temp, "plain", 0o644)));
        }

        #[test]
        fn is_executable_file_rejects_directories_and_missing_paths() {
            let temp = TempDir::new().unwrap();
            assert!(!is_executable_file(temp.path())); // a directory, not a file
            assert!(!is_executable_file(&temp.path().join("absent")));
        }

        #[test]
        fn find_in_paths_returns_the_first_executable_match() {
            let temp = TempDir::new().unwrap();
            write_mode(&temp, "tool", 0o755);
            let path = std::env::join_paths([temp.path()]).unwrap();
            assert_eq!(find_in_paths("tool", &path), Some(temp.path().join("tool")));
        }

        #[test]
        fn find_in_paths_returns_none_for_non_executable_or_absent_name() {
            let temp = TempDir::new().unwrap();
            write_mode(&temp, "data", 0o644);
            let path = std::env::join_paths([temp.path()]).unwrap();
            assert!(find_in_paths("data", &path).is_none()); // present but not executable
            assert!(find_in_paths("ghost", &path).is_none()); // absent
        }
    }
}
