// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Wrappings deliver one epoch's keys to one device: age for CLI and bridge
//! devices, HPKE (RFC 9180) for browser devices.

use hpke::aead::AesGcm256;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem as _, OpModeR, OpModeS, Serializable};
use rand_core::{CryptoRng, OsRng, RngCore};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::Error;
use crate::crypto::{EpochKeys, Key, concat};
use crate::encoding::{Bytes, Id};

/// The plaintext of a wrapping.
#[derive(Serialize, Deserialize)]
struct Plain {
    store: Id,
    epoch: u64,
    names_key: Bytes,
    data_key: Option<Bytes>,
}

impl Plain {
    fn new(store: Id, epoch: u64, keys: &EpochKeys) -> Self {
        Self {
            store,
            epoch,
            names_key: Bytes(keys.names.0.to_vec()),
            data_key: keys.data.as_ref().map(|key| Bytes(key.0.to_vec())),
        }
    }

    fn into_keys(self, store: Id, epoch: u64) -> Result<EpochKeys, Error> {
        if self.store != store || self.epoch != epoch {
            return Err(Error::Decryption);
        }
        let names = to_key(&self.names_key.0)?;
        let data = self
            .data_key
            .as_ref()
            .map(|key| to_key(&key.0))
            .transpose()?;
        Ok(EpochKeys { names, data })
    }
}

impl Drop for Plain {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.names_key.0.zeroize();
        if let Some(data) = &mut self.data_key {
            data.0.zeroize();
        }
    }
}

fn to_key(bytes: &[u8]) -> Result<Key, Error> {
    let array: [u8; 32] = bytes.try_into().map_err(|_| Error::Decryption)?;
    Ok(Key(array))
}

fn plaintext(store: Id, epoch: u64, keys: &EpochKeys) -> Result<Zeroizing<Vec<u8>>, Error> {
    Ok(Zeroizing::new(serde_json::to_vec(&Plain::new(
        store, epoch, keys,
    ))?))
}

fn parse(bytes: &[u8], store: Id, epoch: u64) -> Result<EpochKeys, Error> {
    let plain: Plain = serde_json::from_slice(bytes).map_err(|_| Error::Decryption)?;
    plain.into_keys(store, epoch)
}

/// Encrypts `keys` to an age recipient.
///
/// # Errors
///
/// Fails when age cannot encrypt to the recipient.
#[cfg(feature = "age")]
pub fn wrap_age(
    store: Id,
    epoch: u64,
    keys: &EpochKeys,
    recipient: &dyn age::Recipient,
) -> Result<Vec<u8>, Error> {
    use std::io::Write;

    let plaintext = plaintext(store, epoch, keys)?;
    let encryptor = age::Encryptor::with_recipients(std::iter::once(recipient))
        .map_err(|error| Error::Age(error.to_string()))?;
    let mut wrapped = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut wrapped)
        .map_err(|error| Error::Age(error.to_string()))?;
    writer
        .write_all(&plaintext)
        .and_then(|()| writer.finish().map(|_| ()))
        .map_err(|error| Error::Age(error.to_string()))?;
    Ok(wrapped)
}

/// Decrypts an age wrapping and checks that it is for `store` and `epoch`.
/// Callers still check the keys against the epoch's commitments.
///
/// # Errors
///
/// Fails when no identity decrypts the wrapping or it names another store
/// or epoch.
#[cfg(feature = "age")]
pub fn unwrap_age(
    store: Id,
    epoch: u64,
    wrapped: &[u8],
    identities: &[Box<dyn age::Identity>],
) -> Result<EpochKeys, Error> {
    use std::io::Read;

    let decryptor = age::Decryptor::new_buffered(wrapped).map_err(|_| Error::Decryption)?;
    let mut reader = decryptor
        .decrypt(identities.iter().map(std::ops::Deref::deref))
        .map_err(|error| match error {
            age::DecryptError::NoMatchingKeys => Error::Decryption,
            other => Error::Age(other.to_string()),
        })?;
    let mut plaintext = Zeroizing::new(Vec::with_capacity(wrapped.len()));
    reader
        .read_to_end(&mut plaintext)
        .map_err(|_| Error::Decryption)?;
    parse(&plaintext, store, epoch)
}

const HPKE_INFO: &[u8] = b"pasejo wrapping v1";
const ENCAPPED_LEN: usize = 32;

fn hpke_aad(store: Id, epoch: u64, device: Id) -> Vec<u8> {
    concat(&[store.as_bytes(), &epoch.to_be_bytes(), device.as_bytes()])
}

/// Seals names-only `keys` to a browser device's X25519 public key.
/// Browser devices never receive a data key, so `keys` must be names-only.
///
/// # Errors
///
/// Fails for a data key or an invalid public key.
pub fn wrap_hpke(
    store: Id,
    epoch: u64,
    device: Id,
    public_key: &[u8; 32],
    keys: &EpochKeys,
) -> Result<Vec<u8>, Error> {
    wrap_hpke_with_rng(store, epoch, device, public_key, keys, &mut OsRng)
}

/// [`wrap_hpke`] with a caller-chosen generator, for test vectors.
///
/// # Errors
///
/// See [`wrap_hpke`].
pub fn wrap_hpke_with_rng<R: RngCore + CryptoRng>(
    store: Id,
    epoch: u64,
    device: Id,
    public_key: &[u8; 32],
    keys: &EpochKeys,
    rng: &mut R,
) -> Result<Vec<u8>, Error> {
    if keys.data.is_some() {
        return Err(Error::Invalid(
            "browser devices receive names-only wrappings",
        ));
    }
    let public = <X25519HkdfSha256 as hpke::Kem>::PublicKey::from_bytes(public_key)
        .map_err(|_| Error::Invalid("hpke recipient"))?;
    let plaintext = plaintext(store, epoch, keys)?;
    let (encapped, ciphertext) =
        hpke::single_shot_seal::<AesGcm256, HkdfSha256, X25519HkdfSha256, _>(
            &OpModeS::Base,
            &public,
            HPKE_INFO,
            &plaintext,
            &hpke_aad(store, epoch, device),
            rng,
        )
        .map_err(|_| Error::Invalid("hpke seal"))?;
    let mut wrapped = encapped.to_bytes().to_vec();
    wrapped.extend_from_slice(&ciphertext);
    Ok(wrapped)
}

/// Opens an HPKE wrapping with the device's X25519 private key. Browsers do
/// this with `WebCrypto`; this implementation serves tests and vectors.
///
/// # Errors
///
/// Fails when the wrapping does not open, carries a data key, or names
/// another store or epoch.
pub fn unwrap_hpke(
    store: Id,
    epoch: u64,
    device: Id,
    private_key: &[u8; 32],
    wrapped: &[u8],
) -> Result<EpochKeys, Error> {
    let (encapped, ciphertext) = wrapped
        .split_at_checked(ENCAPPED_LEN)
        .ok_or(Error::Decryption)?;
    let private = <X25519HkdfSha256 as hpke::Kem>::PrivateKey::from_bytes(private_key)
        .map_err(|_| Error::Decryption)?;
    let encapped = <X25519HkdfSha256 as hpke::Kem>::EncappedKey::from_bytes(encapped)
        .map_err(|_| Error::Decryption)?;
    let plaintext = Zeroizing::new(
        hpke::single_shot_open::<AesGcm256, HkdfSha256, X25519HkdfSha256>(
            &OpModeR::Base,
            &private,
            &encapped,
            HPKE_INFO,
            ciphertext,
            &hpke_aad(store, epoch, device),
        )
        .map_err(|_| Error::Decryption)?,
    );
    let keys = parse(&plaintext, store, epoch)?;
    if keys.data.is_some() {
        return Err(Error::Decryption);
    }
    Ok(keys)
}

/// Generates an X25519 key pair for HPKE: (private, public).
#[must_use]
pub fn hpke_keypair() -> ([u8; 32], [u8; 32]) {
    let (private, public) = X25519HkdfSha256::gen_keypair(&mut OsRng);
    let mut private_bytes = [0u8; 32];
    private_bytes.copy_from_slice(&private.to_bytes());
    let mut public_bytes = [0u8; 32];
    public_bytes.copy_from_slice(&public.to_bytes());
    (private_bytes, public_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "age")]
    #[test]
    fn age_wrappings_round_trip_and_check_store_and_epoch() {
        let identity = age::x25519::Identity::generate();
        let recipient = identity.to_public();
        let store = Id::random();
        let keys = EpochKeys::generate();
        let wrapped = wrap_age(store, 2, &keys, &recipient).unwrap();
        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        assert_eq!(unwrap_age(store, 2, &wrapped, &identities).unwrap(), keys);
        assert_eq!(
            unwrap_age(store, 3, &wrapped, &identities),
            Err(Error::Decryption)
        );
        assert_eq!(
            unwrap_age(Id::random(), 2, &wrapped, &identities),
            Err(Error::Decryption)
        );
        let stranger: Vec<Box<dyn age::Identity>> =
            vec![Box::new(age::x25519::Identity::generate())];
        assert_eq!(
            unwrap_age(store, 2, &wrapped, &stranger),
            Err(Error::Decryption)
        );
    }

    #[cfg(feature = "age")]
    #[test]
    fn age_wrappings_carry_names_only_keys() {
        let identity = age::x25519::Identity::generate();
        let store = Id::random();
        let keys = EpochKeys::generate().names_only();
        let wrapped = wrap_age(store, 0, &keys, &identity.to_public()).unwrap();
        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        assert_eq!(unwrap_age(store, 0, &wrapped, &identities).unwrap(), keys);
    }

    #[test]
    fn hpke_wrappings_round_trip_for_the_named_device() {
        let (private, public) = hpke_keypair();
        let (store, device) = (Id::random(), Id::random());
        let keys = EpochKeys::generate().names_only();
        let wrapped = wrap_hpke(store, 1, device, &public, &keys).unwrap();
        assert_eq!(
            unwrap_hpke(store, 1, device, &private, &wrapped).unwrap(),
            keys
        );
        assert!(unwrap_hpke(store, 1, Id::random(), &private, &wrapped).is_err());
        assert!(unwrap_hpke(store, 2, device, &private, &wrapped).is_err());
        let (other, _) = hpke_keypair();
        assert!(unwrap_hpke(store, 1, device, &other, &wrapped).is_err());
        assert!(unwrap_hpke(store, 1, device, &private, &wrapped[..20]).is_err());
    }

    #[test]
    fn browser_devices_never_receive_a_data_key() {
        let (_, public) = hpke_keypair();
        assert!(
            wrap_hpke(
                Id::random(),
                0,
                Id::random(),
                &public,
                &EpochKeys::generate()
            )
            .is_err()
        );
    }
}
