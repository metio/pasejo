// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::Recipient;
use age::cli_common::StdinGuard;

pub fn read_recipients(
    recipients: &[crate::models::password_store::Recipient],
) -> anyhow::Result<Vec<Box<dyn Recipient + Send>>> {
    Ok(age::cli_common::read_recipients(
        recipients
            .iter()
            .map(|recipient| recipient.public_key.clone())
            .collect(),
        vec![],
        vec![],
        None,
        &mut StdinGuard::new(true),
    )?)
}
