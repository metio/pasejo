// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Signed envelopes: JSON payload bytes plus Ed25519 signatures over them.
//! Verifiers check the exact bytes they received before parsing them, so no
//! canonical JSON form is needed.

use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, VerifyingKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::encoding::{Bytes, Fixed, Hash, Id};
use crate::{Error, Millis};

/// What a signature is for. The context is part of the signed message, so a
/// signature made for one purpose never verifies for another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    LogEntry,
    Index,
    Request,
    Token,
    Checkpoint,
}

impl Context {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::LogEntry => "log-entry/v1",
            Self::Index => "index/v1",
            Self::Request => "request/v1",
            Self::Token => "token/v1",
            Self::Checkpoint => "checkpoint/v1",
        }
    }

    /// The message an Ed25519 signature covers:
    /// `"pasejo:" ‖ context ‖ 0x00 ‖ payload`.
    #[must_use]
    pub fn message(self, payload: &[u8]) -> Vec<u8> {
        let label = self.label().as_bytes();
        let mut message = Vec::with_capacity(8 + label.len() + payload.len());
        message.extend_from_slice(b"pasejo:");
        message.extend_from_slice(label);
        message.push(0);
        message.extend_from_slice(payload);
        message
    }
}

/// One signature of an envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub device: Id,
    pub signature: Fixed<64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub payload: Bytes,
    pub signatures: Vec<Signature>,
}

impl Envelope {
    /// Serializes `payload` and signs it as `device`.
    ///
    /// # Errors
    ///
    /// Fails only if the payload cannot be serialized.
    pub fn seal<T: Serialize>(
        context: Context,
        payload: &T,
        device: Id,
        key: &SigningKey,
    ) -> Result<Self, Error> {
        let bytes = serde_json::to_vec(payload)?;
        let mut envelope = Self {
            payload: Bytes(bytes),
            signatures: Vec::new(),
        };
        envelope.add_signature(context, device, key);
        Ok(envelope)
    }

    /// Adds a signature by `device` over the existing payload bytes.
    pub fn add_signature(&mut self, context: Context, device: Id, key: &SigningKey) {
        let signature = key.sign(&context.message(&self.payload.0));
        self.signatures.push(Signature {
            device,
            signature: Fixed(signature.to_bytes()),
        });
    }

    /// Checks the signature made by `device` against `key`.
    ///
    /// # Errors
    ///
    /// Fails when `device` did not sign or its signature does not verify.
    pub fn verify_by(&self, context: Context, device: Id, key: &[u8; 32]) -> Result<(), Error> {
        let signature = self
            .signatures
            .iter()
            .find(|signature| signature.device == device)
            .ok_or(Error::Signature)?;
        verify(context, &self.payload.0, key, &signature.signature.0)
    }

    /// Parses the payload. Callers verify signatures first.
    ///
    /// # Errors
    ///
    /// Fails when the payload is not the expected JSON.
    pub fn open<T: DeserializeOwned>(&self) -> Result<T, Error> {
        Ok(serde_json::from_slice(&self.payload.0)?)
    }

    /// SHA-256 of the payload bytes, which chains log entries and index
    /// versions.
    #[must_use]
    pub fn hash(&self) -> Hash {
        Hash::of(&self.payload.0)
    }

    /// The devices that signed, in order. The first is the author.
    pub fn signers(&self) -> impl Iterator<Item = Id> + '_ {
        self.signatures.iter().map(|signature| signature.device)
    }
}

/// Verifies one Ed25519 signature. Strict verification rejects small-order
/// keys and non-canonical encodings, so a signature has one valid form.
///
/// # Errors
///
/// Fails when the key is not a valid point or the signature does not verify.
pub fn verify(
    context: Context,
    payload: &[u8],
    key: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), Error> {
    let key = VerifyingKey::from_bytes(key).map_err(|_| Error::Signature)?;
    let signature = DalekSignature::from_bytes(signature);
    key.verify_strict(&context.message(payload), &signature)
        .map_err(|_| Error::Signature)
}

/// Signs `payload` for `context` and returns the raw signature.
#[must_use]
pub fn sign(context: Context, payload: &[u8], key: &SigningKey) -> [u8; 64] {
    key.sign(&context.message(payload)).to_bytes()
}

/// A session exchange: a device proves possession of its signing key when
/// it claims a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenClaim {
    /// What the claim is for: the device code of a device grant, `web` for a
    /// console session, `workload` or `emergency`.
    pub purpose: String,
    /// The value being claimed, such as the device code; empty when the
    /// purpose carries none.
    pub value: String,
    pub signing_key: Fixed<32>,
    pub time: Millis,
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;
    use serde_json::json;

    use super::*;

    fn key() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn sealed_envelopes_verify_and_open() {
        let signing = key();
        let device = Id::random();
        let envelope =
            Envelope::seal(Context::LogEntry, &json!({"a": 1}), device, &signing).expect("seal");
        envelope
            .verify_by(
                Context::LogEntry,
                device,
                signing.verifying_key().as_bytes(),
            )
            .expect("verify");
        let value: serde_json::Value = envelope.open().expect("open");
        assert_eq!(value, json!({"a": 1}));
    }

    #[test]
    fn a_signature_does_not_verify_under_another_context() {
        let signing = key();
        let device = Id::random();
        let envelope = Envelope::seal(Context::Index, &json!({}), device, &signing).unwrap();
        assert_eq!(
            envelope.verify_by(
                Context::LogEntry,
                device,
                signing.verifying_key().as_bytes()
            ),
            Err(Error::Signature)
        );
    }

    #[test]
    fn tampered_payloads_and_foreign_keys_fail() {
        let signing = key();
        let device = Id::random();
        let mut envelope =
            Envelope::seal(Context::Index, &json!({"v": 1}), device, &signing).expect("seal");
        let other = key();
        assert!(
            envelope
                .verify_by(Context::Index, device, other.verifying_key().as_bytes())
                .is_err()
        );
        envelope.payload.0[1] ^= 1;
        assert!(
            envelope
                .verify_by(Context::Index, device, signing.verifying_key().as_bytes())
                .is_err()
        );
    }

    #[test]
    fn a_missing_signer_fails() {
        let signing = key();
        let envelope = Envelope::seal(Context::Index, &json!({}), Id::random(), &signing).unwrap();
        assert!(
            envelope
                .verify_by(
                    Context::Index,
                    Id::random(),
                    signing.verifying_key().as_bytes()
                )
                .is_err()
        );
    }

    #[test]
    fn co_signatures_cover_the_same_payload() {
        let first = key();
        let second = key();
        let (a, b) = (Id::random(), Id::random());
        let mut envelope = Envelope::seal(Context::LogEntry, &json!({}), a, &first).unwrap();
        envelope.add_signature(Context::LogEntry, b, &second);
        assert_eq!(envelope.signers().collect::<Vec<_>>(), vec![a, b]);
        envelope
            .verify_by(Context::LogEntry, b, second.verifying_key().as_bytes())
            .unwrap();
    }

    #[test]
    fn the_hash_covers_the_payload_bytes() {
        let signing = key();
        let envelope =
            Envelope::seal(Context::Index, &json!({"x": "y"}), Id::random(), &signing).unwrap();
        assert_eq!(envelope.hash(), Hash::of(br#"{"x":"y"}"#));
    }

    #[test]
    fn envelopes_round_trip_through_json() {
        let signing = key();
        let envelope =
            Envelope::seal(Context::Token, &json!({"p": 1}), Id::random(), &signing).unwrap();
        let text = serde_json::to_string(&envelope).unwrap();
        assert_eq!(serde_json::from_str::<Envelope>(&text).unwrap(), envelope);
    }

    #[test]
    fn the_signed_message_names_the_context() {
        assert_eq!(
            Context::Request.message(b"x"),
            b"pasejo:request/v1\0x".to_vec()
        );
    }
}
