// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::cli_common::{StdinGuard, read_identities};
use duct::cmd;
use std::cell::LazyCell;
use std::path::PathBuf;

pub fn read(
    identity_files: Vec<PathBuf>,
    ignore_missing_identities: bool,
) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    let existing_identities = if ignore_missing_identities {
        let age_plugin_yubikey_output = LazyCell::new(|| {
            if let Ok(plugin_binary) = which::which("age-plugin-yubikey") {
                return cmd!(plugin_binary, "--identity")
                    .stdout_capture()
                    .stderr_null()
                    .read();
            }
            Ok(String::new())
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
                    log::debug!(
                        "Skipping missing identity file {}: {error}",
                        file.display()
                    );
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
