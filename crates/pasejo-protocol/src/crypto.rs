// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Symmetric cryptography of remote stores: HKDF-SHA256 key derivation,
//! AES-256-GCM with random nonces, epoch keys and their commitments.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::encoding::{Hash, Id, random};
use crate::{Error, padding};

pub const NONCE_LEN: usize = 12;

/// A 256-bit symmetric key that is wiped when dropped.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct Key(pub [u8; 32]);

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Key(…)")
    }
}

impl Key {
    #[must_use]
    pub fn random() -> Self {
        Self(random())
    }
}

/// HKDF-SHA256 with the store identifier as salt.
#[must_use]
pub fn derive(ikm: &Key, store: Id, info: &[u8]) -> Key {
    let hkdf = Hkdf::<Sha256>::new(Some(store.as_bytes()), &ikm.0);
    let mut okm = [0u8; 32];
    hkdf.expand(info, &mut okm)
        .unwrap_or_else(|_| unreachable!("32 bytes are a valid HKDF-SHA256 output length"));
    Key(okm)
}

/// Encrypts with AES-256-GCM under a fresh random nonce, written in front of
/// the ciphertext.
///
/// # Errors
///
/// Fails only for plaintexts beyond the AES-GCM limit.
pub fn seal(key: &Key, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
    seal_with_nonce(key, random(), plaintext, aad)
}

/// [`seal`] with a caller-chosen nonce. Only test vectors use it: reusing a
/// nonce under one key breaks AES-GCM.
///
/// # Errors
///
/// Fails only for plaintexts beyond the AES-GCM limit.
pub fn seal_with_nonce(
    key: &Key,
    nonce: [u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, Error> {
    let cipher = Aes256Gcm::new((&key.0).into());
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::Invalid("plaintext too long"))?;
    let mut sealed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    sealed.extend_from_slice(&nonce);
    sealed.extend_from_slice(&ciphertext);
    Ok(sealed)
}

/// Decrypts what [`seal`] produced.
///
/// # Errors
///
/// Fails when the key, the associated data or the ciphertext do not match.
pub fn open(key: &Key, sealed: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let (nonce, ciphertext) = sealed
        .split_at_checked(NONCE_LEN)
        .ok_or(Error::Decryption)?;
    let cipher = Aes256Gcm::new((&key.0).into());
    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Decryption)
}

/// Pads, then seals.
///
/// # Errors
///
/// See [`seal`].
pub fn seal_padded(key: &Key, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, Error> {
    seal(key, &padding::pad(plaintext)?, aad)
}

/// Opens, then strips the padding.
///
/// # Errors
///
/// See [`open`] and [`padding::unpad`].
pub fn open_padded(key: &Key, sealed: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let padded = open(key, sealed, aad)?;
    Ok(Zeroizing::new(padding::unpad(&padded)?.to_vec()))
}

/// Concatenates the parts of a key-derivation info string or of associated
/// data.
#[must_use]
pub fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// The keys of one store epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochKeys {
    pub names: Key,
    /// Absent for names-only holders.
    pub data: Option<Key>,
}

impl EpochKeys {
    #[must_use]
    pub fn generate() -> Self {
        Self {
            names: Key::random(),
            data: Some(Key::random()),
        }
    }

    /// The data key, or [`Error::Invalid`] for names-only holders.
    ///
    /// # Errors
    ///
    /// Fails for names-only keys.
    pub fn data(&self) -> Result<&Key, Error> {
        self.data.as_ref().ok_or(Error::Invalid("names-only keys"))
    }

    #[must_use]
    pub fn names_only(&self) -> Self {
        Self {
            names: self.names.clone(),
            data: None,
        }
    }
}

/// `SHA-256(label ‖ store ‖ epoch ‖ key)`.
#[must_use]
pub fn commitment(label: &[u8], store: Id, epoch: u64, key: &Key) -> Hash {
    let mut digest = Sha256::new();
    digest.update(label);
    digest.update(store.as_bytes());
    digest.update(epoch.to_be_bytes());
    digest.update(key.0);
    Hash(digest.finalize().into())
}

#[must_use]
pub fn names_commitment(store: Id, epoch: u64, key: &Key) -> Hash {
    commitment(b"pasejo names commit v1", store, epoch, key)
}

#[must_use]
pub fn data_commitment(store: Id, epoch: u64, key: &Key) -> Hash {
    commitment(b"pasejo data commit v1", store, epoch, key)
}

/// Checks unwrapped keys against the commitments of the signed epoch entry.
///
/// # Errors
///
/// Fails with [`Error::Commitment`] when a key does not match.
pub fn check_commitments(
    store: Id,
    epoch: u64,
    keys: &EpochKeys,
    names_commit: &Hash,
    data_commit: &Hash,
) -> Result<(), Error> {
    if names_commitment(store, epoch, &keys.names) != *names_commit {
        return Err(Error::Commitment);
    }
    if let Some(data) = &keys.data
        && data_commitment(store, epoch, data) != *data_commit
    {
        return Err(Error::Commitment);
    }
    Ok(())
}

fn previous_aad(store: Id, epoch: u64) -> Vec<u8> {
    concat(&[
        b"pasejo previous v1",
        store.as_bytes(),
        &epoch.to_be_bytes(),
    ])
}

/// The chain objects of epoch `epoch`: the previous epoch's names key under
/// a key derived from this epoch's names key, and likewise for data keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    pub prev_names: Vec<u8>,
    pub prev_data: Vec<u8>,
}

/// Seals `previous` under `current` for epoch `epoch` (> 0).
///
/// # Errors
///
/// Fails when `current` or `previous` lacks a data key.
pub fn seal_chain(
    store: Id,
    epoch: u64,
    current: &EpochKeys,
    previous: &EpochKeys,
) -> Result<Chain, Error> {
    let aad = previous_aad(store, epoch);
    let names_key = derive(&current.names, store, b"pasejo previous names v1");
    let data_key = derive(current.data()?, store, b"pasejo previous data v1");
    Ok(Chain {
        prev_names: seal(&names_key, &previous.names.0, &aad)?,
        prev_data: seal(&data_key, &previous.data()?.0, &aad)?,
    })
}

/// Recovers epoch `epoch − 1`'s keys from epoch `epoch`'s keys and chain.
/// Names-only holders recover the previous names key only.
///
/// # Errors
///
/// Fails when the chain does not decrypt.
pub fn open_chain(
    store: Id,
    epoch: u64,
    current: &EpochKeys,
    chain: &Chain,
) -> Result<EpochKeys, Error> {
    let aad = previous_aad(store, epoch);
    let names_key = derive(&current.names, store, b"pasejo previous names v1");
    let names = key_from(&open(&names_key, &chain.prev_names, &aad)?)?;
    let data = match &current.data {
        Some(data) => {
            let data_key = derive(data, store, b"pasejo previous data v1");
            Some(key_from(&open(&data_key, &chain.prev_data, &aad)?)?)
        }
        None => None,
    };
    Ok(EpochKeys { names, data })
}

fn key_from(bytes: &[u8]) -> Result<Key, Error> {
    let array: [u8; 32] = bytes.try_into().map_err(|_| Error::Decryption)?;
    Ok(Key(array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_data_opens_only_with_the_same_key_and_associated_data() {
        let key = Key::random();
        let sealed = seal(&key, b"secret", b"aad").unwrap();
        assert_eq!(open(&key, &sealed, b"aad").unwrap().as_slice(), b"secret");
        assert_eq!(open(&key, &sealed, b"other"), Err(Error::Decryption));
        assert_eq!(
            open(&Key::random(), &sealed, b"aad"),
            Err(Error::Decryption)
        );
        assert_eq!(open(&key, &sealed[..5], b"aad"), Err(Error::Decryption));
        let mut flipped = sealed;
        flipped[NONCE_LEN] ^= 1;
        assert_eq!(open(&key, &flipped, b"aad"), Err(Error::Decryption));
    }

    #[test]
    fn nonces_are_fresh_for_every_seal() {
        let key = Key::random();
        let first = seal(&key, b"x", b"").unwrap();
        let second = seal(&key, b"x", b"").unwrap();
        assert_ne!(first[..NONCE_LEN], second[..NONCE_LEN]);
    }

    #[test]
    fn padded_sealing_hides_the_exact_length() {
        let key = Key::random();
        let short = seal_padded(&key, b"01234567890", b"").unwrap();
        let longer = seal_padded(&key, b"012345678901", b"").unwrap();
        assert_eq!(short.len(), longer.len());
        assert_eq!(
            open_padded(&key, &longer, b"").unwrap().as_slice(),
            b"012345678901"
        );
    }

    #[test]
    fn derivation_depends_on_key_store_and_info() {
        let key = Key::random();
        let store = Id::random();
        let base = derive(&key, store, b"a");
        assert_eq!(base, derive(&key, store, b"a"));
        assert_ne!(base, derive(&key, store, b"b"));
        assert_ne!(base, derive(&key, Id::random(), b"a"));
        assert_ne!(base, derive(&Key::random(), store, b"a"));
    }

    #[test]
    fn commitments_bind_store_epoch_and_key() {
        let store = Id::random();
        let keys = EpochKeys::generate();
        let names = names_commitment(store, 3, &keys.names);
        let data = data_commitment(store, 3, keys.data().unwrap());
        check_commitments(store, 3, &keys, &names, &data).unwrap();
        check_commitments(store, 3, &keys.names_only(), &names, &Hash::default()).unwrap();
        assert_eq!(
            check_commitments(store, 4, &keys, &names, &data),
            Err(Error::Commitment)
        );
        assert_eq!(
            check_commitments(Id::random(), 3, &keys, &names, &data),
            Err(Error::Commitment)
        );
        let forged = EpochKeys::generate();
        assert_eq!(
            check_commitments(store, 3, &forged, &names, &data),
            Err(Error::Commitment)
        );
        assert_ne!(names, data_commitment(store, 3, &keys.names));
    }

    #[test]
    fn chains_recover_the_previous_epoch() {
        let store = Id::random();
        let previous = EpochKeys::generate();
        let current = EpochKeys::generate();
        let chain = seal_chain(store, 5, &current, &previous).unwrap();
        assert_eq!(open_chain(store, 5, &current, &chain).unwrap(), previous);
        let names_only = open_chain(store, 5, &current.names_only(), &chain).unwrap();
        assert_eq!(names_only, previous.names_only());
        assert!(open_chain(store, 6, &current, &chain).is_err());
        assert!(open_chain(store, 5, &previous, &chain).is_err());
    }

    #[test]
    fn keys_do_not_print_their_bytes() {
        assert_eq!(format!("{:?}", Key([9; 32])), "Key(…)");
    }
}
