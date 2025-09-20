// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::cli_common::{read_identities, StdinGuard};
use duct::cmd;
use std::cell::LazyCell;

pub fn read(
    identity_files: Vec<String>,
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
            if let Ok(content) = std::fs::read_to_string(&file) {
                if content.contains("AGE-PLUGIN-YUBIKEY-") {
                    content.lines().for_each(|line| {
                        if line.starts_with("AGE-PLUGIN-YUBIKEY-") {
                            let parts = line.split('-').collect::<Vec<&str>>();
                            if let Some(split) = parts.split_last()
                                && let Ok(output) = &*age_plugin_yubikey_output
                                && output.contains(split.0)
                            {
                                files.push(file.clone());
                            }
                        }
                    });
                } else {
                    files.push(file.clone());
                }
            }
        }
        files
    } else {
        identity_files
    };

    Ok(read_identities(
        existing_identities,
        None,
        &mut StdinGuard::new(true),
    )?)
}
