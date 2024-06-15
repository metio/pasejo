// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use age::Decryptor;
use std::fs;
use std::io::Read;
use std::path::Path;

pub fn decrypt(
    absolute_secret_path: &Path,
    identities: &[Box<dyn age::Identity>],
) -> anyhow::Result<String> {
    let encrypted = fs::read(absolute_secret_path)?;
    let decryptor = Decryptor::new_buffered(&encrypted[..])?;
    let mut reader = decryptor.decrypt(identities.iter().map(std::ops::Deref::deref))?;
    let mut decrypted = vec![];
    reader.read_to_end(&mut decrypted)?;
    let decrypted_text = String::from_utf8(decrypted)?;
    Ok(decrypted_text)
}
