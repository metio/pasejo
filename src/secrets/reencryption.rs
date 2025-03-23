// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs::write;
use std::io::Write;
use std::path::Path;

use crate::secrets::decrypt;
use age::{Encryptor, Recipient};

pub fn reencrypt(
    absolute_secret_path: &Path,
    identities: &[Box<dyn age::Identity>],
    recipients: &[Box<dyn Recipient + Send>],
) -> anyhow::Result<()> {
    let secret = decrypt(absolute_secret_path, identities)?;
    let encryptor =
        Encryptor::with_recipients(recipients.iter().map(|r| r.as_ref() as &dyn Recipient))?;
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(absolute_secret_path, encrypted)?;
    Ok(())
}
