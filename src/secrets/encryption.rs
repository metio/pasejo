use std::fs::write;
use std::io::Write;
use std::path::Path;

use age::{Encryptor, Recipient};

pub fn encrypt(
    secret: &str,
    absolute_path: &Path,
    recipients: Vec<Box<dyn Recipient + Send>>,
) -> anyhow::Result<()> {
    let Some(encryptor) = Encryptor::with_recipients(recipients) else {
        unreachable!()
    };
    let mut encrypted = vec![];
    let mut writer = encryptor.wrap_output(&mut encrypted)?;
    writer.write_all(secret.as_bytes())?;
    writer.finish()?;
    write(absolute_path, encrypted)?;
    Ok(())
}
