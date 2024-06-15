// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::cli_common::{StdinGuard, read_identities};

pub fn read(identity_files: Vec<String>) -> anyhow::Result<Vec<Box<dyn age::Identity>>> {
    Ok(read_identities(
        identity_files,
        None,
        &mut StdinGuard::new(true),
    )?)
}
