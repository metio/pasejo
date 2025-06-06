// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::cli_common::{StdinGuard, read_identities};
use duct::cmd;

pub fn read(
    identity_files: Vec<String>,
    ignore_missing_identities: bool,
) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    let existing_identities = if ignore_missing_identities {
        let mut files = vec![];
        for file in identity_files {
            if let Ok(content) = std::fs::read_to_string(&file) {
                if content.contains("AGE-PLUGIN-YUBIKEY-") {
                    content.lines().for_each(|line| {
                        if line.starts_with("AGE-PLUGIN-YUBIKEY-") {
                            let parts = line.split('-').collect::<Vec<&str>>();
                            if let Some(split) = parts.split_last() {
                                let plugin_name = split.1.join("-").to_ascii_lowercase();
                                if let Ok(plugin_binary) = which::which(plugin_name) {
                                    if let Ok(output) = cmd!(plugin_binary, "--identity")
                                        .stdout_capture()
                                        .stderr_null()
                                        .read()
                                    {
                                        if output.contains(split.0) {
                                            files.push(file.clone());
                                        }
                                    }
                                }
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
