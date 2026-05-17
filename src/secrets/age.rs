// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use age::{Decryptor, Encryptor, Recipient};
use anyhow::Context;

use crate::cli::i18n;

use crate::cli::atomic_write;

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
    atomic_write::write(path, &encrypted)?;
    Ok(())
}

pub fn decrypt(
    path_to_decrypt: &Path,
    identities: &[Box<dyn age::Identity>],
) -> anyhow::Result<String> {
    let path_display = path_to_decrypt.display().to_string();
    let encrypted =
        fs::read(path_to_decrypt).with_context(|| i18n::error_cannot_read_file(&path_display))?;
    let decryptor = Decryptor::new_buffered(&encrypted[..])?;
    let mut reader = decryptor.decrypt(identities.iter().map(std::ops::Deref::deref))?;
    let mut decrypted = vec![];
    reader.read_to_end(&mut decrypted)?;
    let decrypted_text = String::from_utf8(decrypted)?;
    Ok(decrypted_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use age::x25519;
    use assert_fs::TempDir;

    fn fresh_identity_pair() -> (x25519::Identity, x25519::Recipient) {
        let identity = x25519::Identity::generate();
        let recipient = identity.to_public();
        (identity, recipient)
    }

    #[test]
    fn encrypt_decrypt_round_trip_with_x25519_identity() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("store.age");
        let (identity, recipient) = fresh_identity_pair();
        let plaintext = "top secret payload\nwith two lines";

        let recipients: Vec<Box<dyn Recipient + Send>> = vec![Box::new(recipient)];
        encrypt(plaintext, &path, &recipients).unwrap();

        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        let decrypted = decrypt(&path, &identities).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_overwrites_existing_store_file_atomically() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("store.age");
        let (identity, recipient) = fresh_identity_pair();
        let recipients: Vec<Box<dyn Recipient + Send>> = vec![Box::new(recipient)];

        encrypt("first", &path, &recipients).unwrap();
        encrypt("second", &path, &recipients).unwrap();

        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        let decrypted = decrypt(&path, &identities).unwrap();
        assert_eq!(decrypted, "second");
    }

    #[cfg(unix)]
    #[test]
    fn encrypted_store_file_is_0600_on_unix() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("store.age");
        let (_identity, recipient) = fresh_identity_pair();
        let recipients: Vec<Box<dyn Recipient + Send>> = vec![Box::new(recipient)];
        encrypt("payload", &path, &recipients).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
