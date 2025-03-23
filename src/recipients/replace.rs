// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::path::Path;

use crate::adapters::file_system;
use crate::recipients::format;

pub fn recipients(
    absolute_recipients_path: &Path,
    recipients: &Vec<String>,
    force: bool,
) -> anyhow::Result<()> {
    file_system::replace_file_content(
        absolute_recipients_path,
        &format::recipients(recipients),
        force,
        "Replace existing recipients?",
        "Recipients will be taken from --recipient if confirmed",
    )
}
