// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::cell::LazyCell;
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
                        let mut matched_any = false;
                        content.lines().for_each(|line| {
                            if line.starts_with("AGE-PLUGIN-YUBIKEY-") {
                                let parts = line.split('-').collect::<Vec<&str>>();
                                if let Some(split) = parts.split_last()
                                    && let Ok(output) = &*age_plugin_yubikey_output
                                    && output.contains(split.0)
                                {
                                    files.push(file.clone());
                                    matched_any = true;
                                }
                            }
                        });
                        if !matched_any {
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

/// Find an executable by name on `PATH`. Replacement for `which::which`
/// scoped to the one place we need it (the `age-plugin-yubikey` lookup
/// above). Returns the first match, or `None` if `PATH` is unset or no
/// executable file with that name exists in any entry.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
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
