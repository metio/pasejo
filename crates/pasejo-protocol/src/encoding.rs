// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Identifiers, binary values and hashes as they appear on the wire.

use std::fmt;
use std::str::FromStr;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::Error;

/// Encodes bytes as base64url without padding.
#[must_use]
pub fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes base64url without padding.
///
/// # Errors
///
/// Fails on characters outside the URL-safe alphabet and on padding.
pub fn unb64(text: &str) -> Result<Vec<u8>, Error> {
    URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| Error::Encoding("base64url"))
}

/// SHA-256 of `bytes`.
#[must_use]
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Fills a fresh array with bytes from the operating system's generator.
#[must_use]
pub fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

/// A 16-byte random identifier of an organization, user, device, team,
/// store or secret, written as 32 lowercase hexadecimal characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Id(pub [u8; 16]);

impl Id {
    #[must_use]
    pub fn random() -> Self {
        Self(random())
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id({self})")
    }
}

impl FromStr for Id {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let bytes = text.as_bytes();
        if bytes.len() != 32 {
            return Err(Error::Encoding("identifier"));
        }
        let mut id = [0u8; 16];
        for (index, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
            id[index] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
        }
        Ok(Self(id))
    }
}

/// Only lowercase digits are accepted, so every identifier has exactly one
/// spelling and string comparisons agree with byte comparisons.
const fn nibble(character: u8) -> Result<u8, Error> {
    match character {
        b'0'..=b'9' => Ok(character - b'0'),
        b'a'..=b'f' => Ok(character - b'a' + 10),
        _ => Err(Error::Encoding("identifier")),
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Binary data written as base64url without padding.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Bytes(pub Vec<u8>);

impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes({} bytes)", self.0.len())
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&b64(&self.0))
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        unb64(&text).map(Self).map_err(serde::de::Error::custom)
    }
}

/// A SHA-256 hash written as base64url.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(sha256(bytes))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", b64(&self.0))
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&b64(&self.0))
    }
}

impl FromStr for Hash {
    type Err = Error;

    fn from_str(text: &str) -> Result<Self, Error> {
        let bytes: [u8; 32] = unb64(text)?
            .try_into()
            .map_err(|_| Error::Encoding("hash"))?;
        Ok(Self(bytes))
    }
}

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

/// Fixed-length keys and signatures written as base64url.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fixed<const N: usize>(pub [u8; N]);

impl<const N: usize> fmt::Debug for Fixed<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fixed({})", b64(&self.0))
    }
}

impl<const N: usize> Serialize for Fixed<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&b64(&self.0))
    }
}

impl<'de, const N: usize> Deserialize<'de> for Fixed<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let bytes: [u8; N] = unb64(&text)
            .map_err(serde::de::Error::custom)?
            .try_into()
            .map_err(|_| serde::de::Error::custom("wrong length"))?;
        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_round_trip_through_lowercase_hex() {
        let id = Id([
            0x00, 0x01, 0x0a, 0xff, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
            0xb0, 0xc0,
        ]);
        assert_eq!(id.to_string(), "00010aff102030405060708090a0b0c0");
        assert_eq!(id.to_string().parse::<Id>().unwrap(), id);
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            "\"00010aff102030405060708090a0b0c0\""
        );
    }

    #[test]
    fn identifiers_reject_uppercase_wrong_length_and_other_characters() {
        assert!("00010AFF102030405060708090A0B0C0".parse::<Id>().is_err());
        assert!("00".parse::<Id>().is_err());
        assert!("zz010aff102030405060708090a0b0c0".parse::<Id>().is_err());
        assert!("00010aff102030405060708090a0b0c0ff".parse::<Id>().is_err());
        assert!(serde_json::from_str::<Id>("\"xyz\"").is_err());
    }

    #[test]
    fn random_identifiers_differ() {
        assert_ne!(Id::random(), Id::random());
    }

    #[test]
    fn bytes_use_base64url_without_padding() {
        let bytes = Bytes(vec![0xfb, 0xff, 0x00]);
        assert_eq!(serde_json::to_string(&bytes).unwrap(), "\"-_8A\"");
        assert_eq!(serde_json::from_str::<Bytes>("\"-_8A\"").unwrap(), bytes);
        assert!(serde_json::from_str::<Bytes>("\"-_8A==\"").is_err());
        assert!(serde_json::from_str::<Bytes>("\"+/8A\"").is_err());
    }

    #[test]
    fn hashes_are_sha256_and_round_trip() {
        let hash = Hash::of(b"abc");
        assert_eq!(
            hash.to_string(),
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
        );
        assert_eq!(hash.to_string().parse::<Hash>().unwrap(), hash);
        assert!("AAAA".parse::<Hash>().is_err());
    }

    #[test]
    fn fixed_values_require_their_length() {
        let key = Fixed([7u8; 32]);
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(serde_json::from_str::<Fixed<32>>(&json).unwrap(), key);
        assert!(serde_json::from_str::<Fixed<64>>(&json).is_err());
    }
}
