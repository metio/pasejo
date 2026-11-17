// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Request proofs: every authenticated request is signed by the device the
//! session is bound to, so a session token alone authorizes nothing.

use ed25519_dalek::SigningKey;

use crate::encoding::{Fixed, b64, random, sha256, unb64};
use crate::envelope::{Context, sign, verify};
use crate::{Error, Millis};

pub const AUTHORIZATION_SCHEME: &str = "Pasejo";
pub const TIMESTAMP_HEADER: &str = "pasejo-timestamp";
pub const NONCE_HEADER: &str = "pasejo-nonce";
pub const PROOF_HEADER: &str = "pasejo-proof";

/// How far a request's timestamp may be from the server's clock.
pub const MAX_SKEW_MILLIS: Millis = 60_000;

/// The message a request proof signs:
/// `METHOD \n path-and-query \n timestamp \n nonce \n b64(SHA-256(body)) \n
/// b64(SHA-256(token))`.
#[must_use]
pub fn message(
    method: &str,
    path_and_query: &str,
    timestamp: Millis,
    nonce: &str,
    body: &[u8],
    token: &str,
) -> Vec<u8> {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.to_ascii_uppercase(),
        path_and_query,
        timestamp,
        nonce,
        b64(&sha256(body)),
        b64(&sha256(token.as_bytes())),
    )
    .into_bytes()
}

/// The headers a client adds to an authenticated request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    pub authorization: String,
    pub timestamp: String,
    pub nonce: String,
    pub proof: String,
}

/// Signs a request.
#[must_use]
pub fn prove(
    method: &str,
    path_and_query: &str,
    timestamp: Millis,
    body: &[u8],
    token: &str,
    key: &SigningKey,
) -> Proof {
    let nonce = b64(&random::<16>());
    let signature = sign(
        Context::Request,
        &message(method, path_and_query, timestamp, &nonce, body, token),
        key,
    );
    proof(token, timestamp, nonce, &signature)
}

/// Signs a request with a key that may refuse.
///
/// # Errors
///
/// Fails when the key cannot sign.
pub fn try_prove<S: crate::envelope::Signer + ?Sized>(
    method: &str,
    path_and_query: &str,
    timestamp: Millis,
    body: &[u8],
    token: &str,
    key: &S,
) -> Result<Proof, Error> {
    let nonce = b64(&random::<16>());
    let signature = crate::envelope::try_sign(
        Context::Request,
        &message(method, path_and_query, timestamp, &nonce, body, token),
        key,
    )?;
    Ok(proof(token, timestamp, nonce, &signature))
}

fn proof(token: &str, timestamp: Millis, nonce: String, signature: &[u8; 64]) -> Proof {
    Proof {
        authorization: format!("{AUTHORIZATION_SCHEME} {token}"),
        timestamp: timestamp.to_string(),
        nonce,
        proof: b64(signature),
    }
}

/// The parts of a received request the server checks.
#[derive(Debug, Clone, Copy)]
pub struct Received<'a> {
    pub method: &'a str,
    pub path_and_query: &'a str,
    pub timestamp: &'a str,
    pub nonce: &'a str,
    pub proof: &'a str,
    pub body: &'a [u8],
    pub token: &'a str,
}

/// Checks a request proof's timestamp window, nonce shape and signature.
/// Replay protection across requests is the caller's: it must refuse a
/// nonce it already accepted within [`MAX_SKEW_MILLIS`].
///
/// # Errors
///
/// Fails for a stale timestamp, a malformed nonce, or a bad signature.
pub fn check(received: &Received<'_>, key: &Fixed<32>, now: Millis) -> Result<Millis, Error> {
    let timestamp: Millis = received
        .timestamp
        .parse()
        .map_err(|_| Error::Encoding("timestamp"))?;
    if timestamp.abs_diff(now) > MAX_SKEW_MILLIS {
        return Err(Error::Invalid(
            "request timestamp outside the allowed window",
        ));
    }
    if unb64(received.nonce).map(|nonce| nonce.len()) != Ok(16) {
        return Err(Error::Encoding("nonce"));
    }
    let signature: [u8; 64] = unb64(received.proof)?
        .try_into()
        .map_err(|_| Error::Encoding("proof"))?;
    verify(
        Context::Request,
        &message(
            received.method,
            received.path_and_query,
            timestamp,
            received.nonce,
            received.body,
            received.token,
        ),
        &key.0,
        &signature,
    )?;
    Ok(timestamp)
}

/// Extracts the token from an `Authorization: Pasejo <token>` header.
#[must_use]
pub fn token_from_header(header: &str) -> Option<&str> {
    let (scheme, token) = header.split_once(' ')?;
    (scheme == AUTHORIZATION_SCHEME && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;

    fn setup() -> (SigningKey, Fixed<32>) {
        let key = SigningKey::generate(&mut OsRng);
        let public = Fixed(key.verifying_key().to_bytes());
        (key, public)
    }

    fn received<'a>(proof: &'a Proof, token: &'a str, body: &'a [u8]) -> Received<'a> {
        Received {
            method: "POST",
            path_and_query: "/v1/orgs?x=1",
            timestamp: &proof.timestamp,
            nonce: &proof.nonce,
            proof: &proof.proof,
            body,
            token,
        }
    }

    #[test]
    fn proofs_verify_for_the_signed_request() {
        let (key, public) = setup();
        let proof = prove("post", "/v1/orgs?x=1", 1_000_000, b"{}", "tok", &key);
        assert_eq!(proof.authorization, "Pasejo tok");
        assert_eq!(
            check(&received(&proof, "tok", b"{}"), &public, 1_000_500).unwrap(),
            1_000_000
        );
    }

    #[test]
    fn proofs_fail_for_any_changed_part() {
        let (key, public) = setup();
        let proof = prove("POST", "/v1/orgs?x=1", 1_000_000, b"{}", "tok", &key);
        assert!(check(&received(&proof, "tok", b"{ }"), &public, 1_000_000).is_err());
        assert!(check(&received(&proof, "other", b"{}"), &public, 1_000_000).is_err());
        let mut wrong_path = received(&proof, "tok", b"{}");
        wrong_path.path_and_query = "/v1/orgs?x=2";
        assert!(check(&wrong_path, &public, 1_000_000).is_err());
        let mut wrong_method = received(&proof, "tok", b"{}");
        wrong_method.method = "GET";
        assert!(check(&wrong_method, &public, 1_000_000).is_err());
        let (_, stranger) = setup();
        assert!(check(&received(&proof, "tok", b"{}"), &stranger, 1_000_000).is_err());
    }

    #[test]
    fn stale_or_future_timestamps_are_refused() {
        let (key, public) = setup();
        let proof = prove("GET", "/v1/orgs?x=1", 1_000_000, b"", "tok", &key);
        let mut request = received(&proof, "tok", b"");
        request.method = "GET";
        assert!(check(&request, &public, 1_000_000 + MAX_SKEW_MILLIS).is_ok());
        assert!(check(&request, &public, 1_000_000 + MAX_SKEW_MILLIS + 1).is_err());
        assert!(check(&request, &public, 1_000_000 - MAX_SKEW_MILLIS - 1).is_err());
    }

    #[test]
    fn malformed_headers_are_refused() {
        let (key, public) = setup();
        let proof = prove("GET", "/", 5, b"", "tok", &key);
        let base = Received {
            method: "GET",
            path_and_query: "/",
            timestamp: &proof.timestamp,
            nonce: &proof.nonce,
            proof: &proof.proof,
            body: b"",
            token: "tok",
        };
        assert!(
            check(
                &Received {
                    timestamp: "x",
                    ..base
                },
                &public,
                5
            )
            .is_err()
        );
        assert!(
            check(
                &Received {
                    nonce: "AAAA",
                    ..base
                },
                &public,
                5
            )
            .is_err()
        );
        assert!(
            check(
                &Received {
                    proof: "AAAA",
                    ..base
                },
                &public,
                5
            )
            .is_err()
        );
    }

    #[test]
    fn nonces_differ_between_proofs() {
        let (key, _) = setup();
        let first = prove("GET", "/", 1, b"", "t", &key);
        let second = prove("GET", "/", 1, b"", "t", &key);
        assert_ne!(first.nonce, second.nonce);
    }

    #[test]
    fn tokens_come_out_of_the_pasejo_scheme_only() {
        assert_eq!(token_from_header("Pasejo abc"), Some("abc"));
        assert_eq!(token_from_header("Bearer abc"), None);
        assert_eq!(token_from_header("Pasejo "), None);
        assert_eq!(token_from_header("Pasejo"), None);
    }
}
