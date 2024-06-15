// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs::write;
use std::io::Write;
use std::path::Path;

use age::{Encryptor, Recipient};

pub fn encrypt(
    secret: &str,
    path: &Path,
    recipients: &[Box<dyn Recipient + Send>],
) -> anyhow::Result<()> {
    let encryptor =
        Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as &dyn Recipient))?;
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(path, encrypted)?;
    Ok(())
}
