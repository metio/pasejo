// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The Kubernetes bridge: a machine device of one organization that serves
//! single secret values to external-secrets-operator's webhook provider.
//! Callers authenticate with bearer tokens, each mapped to a tenant label
//! the server records with every read.

pub mod bridge;
pub mod config;
pub mod device;
pub mod http;

use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

/// A new bearer token and the hex SHA-256 the configuration stores.
#[must_use]
pub fn credential() -> (Zeroizing<String>, String) {
    let mut bytes = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(bytes.as_mut());
    let token = Zeroizing::new(format!(
        "pasejo-bridge-{}",
        pasejo_protocol::encoding::b64(bytes.as_ref())
    ));
    let hash = config::hex(&Sha256::digest(token.as_bytes()));
    (token, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_random_and_hashed() {
        let (first, first_hash) = credential();
        let (second, second_hash) = credential();
        assert_ne!(first.as_str(), second.as_str());
        assert_ne!(first_hash, second_hash);
        assert!(first.starts_with("pasejo-bridge-"));
        assert_eq!(first_hash, config::hex(&Sha256::digest(first.as_bytes())));
        assert_eq!(first_hash.len(), 64);
    }
}
