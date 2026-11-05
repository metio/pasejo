// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use age::{Decryptor, Encryptor, Recipient};
use anyhow::Context;
use zeroize::Zeroizing;

use crate::cli::atomic_write;
use crate::cli::i18n;

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

/// Returns the decrypted store body wrapped in `Zeroizing` so the
/// plaintext — the entire store: every secret, recipient, and OTP seed —
/// is scrubbed from memory when the caller is done with it rather than
/// lingering in a freed heap page. `String::from_utf8` reuses the
/// `decrypted` buffer's allocation in place, so the bytes the caller
/// reads are exactly the bytes the `Zeroizing<String>` owns and clears;
/// on invalid UTF-8 the recovered buffer is scrubbed before returning.
pub fn decrypt(
    path_to_decrypt: &Path,
    identities: &[Box<dyn age::Identity>],
) -> anyhow::Result<Zeroizing<String>> {
    let path_display = path_to_decrypt.display().to_string();
    let encrypted =
        fs::read(path_to_decrypt).with_context(|| i18n::error_cannot_read_file(&path_display))?;
    let decryptor = Decryptor::new_buffered(&encrypted[..])?;
    let reader = decryptor.decrypt(identities.iter().map(std::ops::Deref::deref))?;
    let mut decrypted = read_plaintext(reader, encrypted.len())?;
    match String::from_utf8(std::mem::take(&mut *decrypted)) {
        Ok(text) => Ok(Zeroizing::new(text)),
        Err(error) => {
            let utf8_error = error.utf8_error();
            drop(Zeroizing::new(error.into_bytes()));
            Err(utf8_error.into())
        }
    }
}

/// Reads the whole plaintext into a buffer of `capacity` bytes. Passing the
/// ciphertext length keeps `read_to_end` from ever reallocating, because an
/// age plaintext is always shorter than its ciphertext, so no freed
/// intermediate buffer is left holding a prefix of the store.
fn read_plaintext(mut reader: impl Read, capacity: usize) -> std::io::Result<Zeroizing<Vec<u8>>> {
    let mut plaintext = Zeroizing::new(Vec::with_capacity(capacity));
    reader.read_to_end(&mut plaintext)?;
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use age::x25519;
    use assert_fs::TempDir;

    use super::*;

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
        assert_eq!(decrypted.as_str(), plaintext);
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
        assert_eq!(decrypted.as_str(), "second");
    }

    #[test]
    fn read_plaintext_fills_the_ciphertext_sized_buffer_without_reallocating() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("store.age");
        let (identity, recipient) = fresh_identity_pair();
        let recipients: Vec<Box<dyn Recipient + Send>> = vec![Box::new(recipient)];
        let plaintext = "x".repeat(100 * 1024);
        encrypt(&plaintext, &path, &recipients).unwrap();

        let encrypted = fs::read(&path).unwrap();
        let decryptor = Decryptor::new_buffered(&encrypted[..]).unwrap();
        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        let reader = decryptor
            .decrypt(identities.iter().map(std::ops::Deref::deref))
            .unwrap();
        let decrypted = read_plaintext(reader, encrypted.len()).unwrap();

        assert_eq!(decrypted.as_slice(), plaintext.as_bytes());
        assert_eq!(decrypted.capacity(), encrypted.len());
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
