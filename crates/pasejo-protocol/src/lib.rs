// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The protocol between pasejo clients and `pasejo-server`: wire types, the
//! encrypted object formats of remote stores, key wrapping, request proofs
//! and verification of the membership log. Both sides link this crate, so it
//! does no I/O. `design/server.md` at the root of the repository specifies
//! every format implemented here.

pub mod api;
pub mod checkpoint;
pub mod crypto;
pub mod device;
pub mod encoding;
pub mod envelope;
pub mod log;
pub mod padding;
pub mod request;
pub mod secret;
pub mod store;
pub mod wrapping;

pub use crate::encoding::{Bytes, Hash, Id};
pub use crate::log::Rejection;

/// Everything that can go wrong while decoding, verifying or decrypting
/// protocol data.
///
/// The variants are deliberately coarse for cryptographic
/// failures, so an error never tells an attacker which check failed first.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("malformed {0}")]
    Encoding(&'static str),
    #[error("malformed JSON: {0}")]
    Json(String),
    #[error("signature does not verify")]
    Signature,
    #[error("decryption failed")]
    Decryption,
    #[error("unwrapped key does not match the commitment in the membership log")]
    Commitment,
    #[error("membership log rejected the entry: {0}")]
    Log(#[from] Rejection),
    #[error("{0}")]
    Invalid(&'static str),
    #[error("age: {0}")]
    Age(String),
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

/// Milliseconds since the Unix epoch.
pub type Millis = u64;
