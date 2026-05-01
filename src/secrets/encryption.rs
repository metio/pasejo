// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

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
    write_file(path, &encrypted)?;
    Ok(())
}

#[cfg(unix)]
fn write_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_file(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    std::fs::write(path, contents)?;
    Ok(())
}
