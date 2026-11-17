// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The device keys. Apps keep them in the platform's key store and
//! implement [`DeviceSigner`] and [`KeyAgreement`] over them, so private keys
//! in secure hardware never enter this library. Keys the platform cannot
//! hold, such as Curve25519 keys on phones whose key store lacks them,
//! are generated here, sealed by the app and handed back through
//! [`software_signer`] and [`software_agreement`].

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use age_core::format::{FILE_KEY_BYTES, FileKey, Stanza};
use age_core::primitives::{aead_decrypt, hkdf};
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use ed25519_dalek::{Signature, Signer as _, SigningKey, VerifyingKey};
use pasejo_protocol::envelope::Signer;
use rand_core::{OsRng, RngCore};
use zeroize::Zeroizing;

use crate::error::MobileError;

const X25519_TAG: &str = "X25519";
const X25519_LABEL: &[u8] = b"age-encryption.org/v1/X25519";

/// The device's Ed25519 signing key.
#[uniffi::export(with_foreign)]
pub trait DeviceSigner: Send + Sync {
    /// The raw 32-byte public key.
    fn public_key(&self) -> Vec<u8>;

    /// Signs `message`, which is at most 64 bytes long, and returns the
    /// 64-byte signature.
    ///
    /// # Errors
    ///
    /// Fails when the key cannot sign now.
    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, MobileError>;
}

/// The device's X25519 key, the age identity store keys are wrapped to.
#[uniffi::export(with_foreign)]
pub trait KeyAgreement: Send + Sync {
    /// The raw 32-byte public key.
    fn public_key(&self) -> Vec<u8>;

    /// The 32-byte X25519 shared secret with `share`.
    ///
    /// # Errors
    ///
    /// Fails when the key cannot agree now, for example because the person
    /// has not authenticated.
    fn agree(&self, share: Vec<u8>) -> Result<Vec<u8>, MobileError>;
}

fn key32(bytes: &[u8], what: &str) -> Result<[u8; 32], MobileError> {
    <[u8; 32]>::try_from(bytes).map_err(|_| MobileError::key(format!("{what} is not 32 bytes")))
}

/// A [`DeviceSigner`] as the signer `pasejo-client` uses. Every signature
/// is checked against the public key before it leaves, so a key store that
/// returns something else fails here instead of at the server.
pub struct SignerAdapter {
    signer: Arc<dyn DeviceSigner>,
    public: VerifyingKey,
}

impl SignerAdapter {
    /// # Errors
    ///
    /// Fails when the public key is not a valid Ed25519 key.
    pub fn new(signer: Arc<dyn DeviceSigner>) -> Result<Self, MobileError> {
        let public = VerifyingKey::from_bytes(&key32(&signer.public_key(), "the public key")?)
            .map_err(|_| MobileError::key("the public key is not an Ed25519 key"))?;
        Ok(Self { signer, public })
    }
}

impl SignerAdapter {
    /// Makes one signature and checks it, so a key store that cannot sign
    /// or returns invalid signatures fails before the device enrolls.
    /// Protocol messages start with `pasejo:`, so the probe can never be one.
    ///
    /// # Errors
    ///
    /// Fails when the key store refuses or the signature does not verify.
    pub fn probe(&self) -> Result<(), MobileError> {
        self.sign_message(b"pasejo-probe")
            .map(|_| ())
            .map_err(|error| MobileError::key(error.to_string()))
    }
}

impl Signer for SignerAdapter {
    fn verifying_key(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    fn sign_message(&self, message: &[u8]) -> Result<[u8; 64], pasejo_protocol::Error> {
        let failed = |reason: String| pasejo_protocol::Error::Signing(reason);
        let bytes = self
            .signer
            .sign(message.to_vec())
            .map_err(|error| failed(error.to_string()))?;
        let signature = <[u8; 64]>::try_from(bytes.as_slice())
            .map_err(|_| failed("the signature is not 64 bytes".into()))?;
        self.public
            .verify_strict(message, &Signature::from_bytes(&signature))
            .map_err(|_| failed("the key store returned an invalid signature".into()))?;
        Ok(signature)
    }
}

/// An age identity whose X25519 agreement happens behind [`KeyAgreement`].
/// It unwraps X25519 stanzas exactly as age's native identity does, with the
/// shared secret coming from the key store.
pub struct AgreementIdentity {
    key: Arc<dyn KeyAgreement>,
    public: [u8; 32],
    failure: Mutex<Option<MobileError>>,
}

impl AgreementIdentity {
    /// # Errors
    ///
    /// Fails when the public key is not 32 bytes.
    pub fn new(key: Arc<dyn KeyAgreement>) -> Result<Self, MobileError> {
        let public = key32(&key.public_key(), "the public key")?;
        Ok(Self {
            key,
            public,
            failure: Mutex::new(None),
        })
    }

    /// The error of the last agreement the key store refused, which age's
    /// interface cannot carry.
    pub fn take_failure(&self) -> Option<MobileError> {
        self.failure
            .lock()
            .ok()
            .and_then(|mut failure| failure.take())
    }
}

impl age::Identity for AgreementIdentity {
    fn unwrap_stanza(&self, stanza: &Stanza) -> Option<Result<FileKey, age::DecryptError>> {
        if stanza.tag != X25519_TAG {
            return None;
        }
        let share = match &stanza.args[..] {
            [arg] => match STANDARD_NO_PAD.decode(arg) {
                Ok(bytes) if bytes.len() == 32 && STANDARD_NO_PAD.encode(&bytes) == *arg => bytes,
                _ => return Some(Err(age::DecryptError::InvalidHeader)),
            },
            _ => return Some(Err(age::DecryptError::InvalidHeader)),
        };
        if stanza.body.len() != FILE_KEY_BYTES + 16 {
            return Some(Err(age::DecryptError::InvalidHeader));
        }
        let shared = match self.key.agree(share.clone()) {
            Ok(shared) if shared.len() == 32 => Zeroizing::new(shared),
            Ok(_) => {
                self.record(MobileError::key("the shared secret is not 32 bytes"));
                return Some(Err(age::DecryptError::DecryptionFailed));
            }
            Err(error) => {
                self.record(error);
                return Some(Err(age::DecryptError::DecryptionFailed));
            }
        };
        if shared.iter().all(|byte| *byte == 0) {
            return Some(Err(age::DecryptError::InvalidHeader));
        }
        let mut salt = [0; 64];
        salt[..32].copy_from_slice(&share);
        salt[32..].copy_from_slice(&self.public);
        let key = Zeroizing::new(hkdf(&salt, X25519_LABEL, &shared));
        aead_decrypt(&key, FILE_KEY_BYTES, &stanza.body)
            .ok()
            .map(|plain| {
                let plain = Zeroizing::new(plain);
                Ok(FileKey::init_with_mut(|file_key| {
                    file_key.copy_from_slice(&plain);
                }))
            })
    }
}

impl AgreementIdentity {
    fn record(&self, error: MobileError) {
        if let Ok(mut failure) = self.failure.lock() {
            *failure = Some(error);
        }
    }
}

/// The age recipient (`age1…`) of an X25519 public key.
///
/// # Errors
///
/// Fails when the key is not 32 bytes.
// UniFFI passes arguments from the foreign side as owned values.
#[allow(clippy::needless_pass_by_value)]
#[uniffi::export]
pub fn recipient(public_key: Vec<u8>) -> Result<String, MobileError> {
    let public = key32(&public_key, "the public key")?;
    let hrp = bech32::Hrp::parse("age").map_err(|error| MobileError::key(error.to_string()))?;
    let text = bech32::encode::<bech32::Bech32>(hrp, &public)
        .map_err(|error| MobileError::key(error.to_string()))?;
    age::x25519::Recipient::from_str(&text).map_err(|error| MobileError::key(error.to_string()))?;
    Ok(text)
}

/// 32 random bytes from the operating system, a seed for
/// [`software_signer`] or [`software_agreement`].
#[uniffi::export]
#[must_use]
pub fn generate_secret() -> Vec<u8> {
    let mut secret = vec![0; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

struct SoftwareSigner(SigningKey);

impl DeviceSigner for SoftwareSigner {
    fn public_key(&self) -> Vec<u8> {
        self.0.verifying_key().to_bytes().to_vec()
    }

    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, MobileError> {
        Ok(self.0.sign(&message).to_bytes().to_vec())
    }
}

struct SoftwareAgreement(x25519_dalek::StaticSecret);

impl KeyAgreement for SoftwareAgreement {
    fn public_key(&self) -> Vec<u8> {
        x25519_dalek::PublicKey::from(&self.0).to_bytes().to_vec()
    }

    fn agree(&self, share: Vec<u8>) -> Result<Vec<u8>, MobileError> {
        let share = x25519_dalek::PublicKey::from(key32(&share, "the share")?);
        Ok(self.0.diffie_hellman(&share).as_bytes().to_vec())
    }
}

/// An Ed25519 signer over a 32-byte seed the app unsealed. The seed is
/// copied into the key and the argument wiped.
///
/// # Errors
///
/// Fails when the seed is not 32 bytes.
#[uniffi::export]
pub fn software_signer(seed: Vec<u8>) -> Result<Arc<dyn DeviceSigner>, MobileError> {
    let seed = Zeroizing::new(seed);
    Ok(Arc::new(SoftwareSigner(SigningKey::from_bytes(&key32(
        &seed, "the seed",
    )?))))
}

/// An X25519 key over a 32-byte secret the app unsealed. The secret is
/// copied into the key and the argument wiped.
///
/// # Errors
///
/// Fails when the secret is not 32 bytes.
#[uniffi::export]
pub fn software_agreement(secret: Vec<u8>) -> Result<Arc<dyn KeyAgreement>, MobileError> {
    let secret = Zeroizing::new(secret);
    Ok(Arc::new(SoftwareAgreement(
        x25519_dalek::StaticSecret::from(key32(&secret, "the secret")?),
    )))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::io::{Read, Write};

    use age::secrecy::ExposeSecret;

    use super::*;

    fn encrypt_to(recipient: &str, plain: &[u8]) -> Vec<u8> {
        let recipient = age::x25519::Recipient::from_str(recipient).unwrap();
        let encryptor =
            age::Encryptor::with_recipients(std::iter::once(&recipient as &dyn age::Recipient))
                .unwrap();
        let mut out = Vec::new();
        let mut writer = encryptor.wrap_output(&mut out).unwrap();
        writer.write_all(plain).unwrap();
        writer.finish().unwrap();
        out
    }

    fn decrypt_with(
        identity: &dyn age::Identity,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, age::DecryptError> {
        let decryptor = age::Decryptor::new(ciphertext)?;
        let mut reader = decryptor.decrypt(std::iter::once(identity))?;
        let mut plain = Vec::new();
        reader.read_to_end(&mut plain).unwrap();
        Ok(plain)
    }

    struct Refusing(Vec<u8>);

    impl KeyAgreement for Refusing {
        fn public_key(&self) -> Vec<u8> {
            self.0.clone()
        }

        fn agree(&self, _share: Vec<u8>) -> Result<Vec<u8>, MobileError> {
            Err(MobileError::key("authentication required"))
        }
    }

    struct Lying(Arc<dyn DeviceSigner>);

    impl DeviceSigner for Lying {
        fn public_key(&self) -> Vec<u8> {
            self.0.public_key()
        }

        fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, MobileError> {
            let mut signature = self.0.sign(message)?;
            signature[0] ^= 1;
            Ok(signature)
        }
    }

    #[test]
    fn the_probe_accepts_a_working_signer_and_refuses_a_lying_one() {
        let signer = software_signer(generate_secret()).unwrap();
        assert!(
            SignerAdapter::new(Arc::clone(&signer))
                .unwrap()
                .probe()
                .is_ok()
        );
        let lying = SignerAdapter::new(Arc::new(Lying(signer))).unwrap();
        assert!(matches!(lying.probe(), Err(MobileError::Key { .. })));
    }

    #[test]
    fn the_recipient_matches_the_one_age_derives() {
        let identity = age::x25519::Identity::generate();
        let secret = identity.to_string();
        let native = identity.to_public().to_string();
        // The bech32 secret key decodes to the 32-byte scalar.
        let (_, bytes) = bech32::decode(secret.expose_secret()).unwrap();
        let agreement = software_agreement(bytes).unwrap();
        assert_eq!(recipient(agreement.public_key()).unwrap(), native);
    }

    #[test]
    fn a_foreign_agreement_opens_what_age_encrypted() {
        let agreement = software_agreement(generate_secret()).unwrap();
        let ciphertext = encrypt_to(&recipient(agreement.public_key()).unwrap(), b"epoch keys");
        let identity = AgreementIdentity::new(agreement).unwrap();
        assert_eq!(decrypt_with(&identity, &ciphertext).unwrap(), b"epoch keys");
    }

    #[test]
    fn another_agreement_key_does_not_open_it() {
        let ours = software_agreement(generate_secret()).unwrap();
        let ciphertext = encrypt_to(&recipient(ours.public_key()).unwrap(), b"epoch keys");
        let theirs =
            AgreementIdentity::new(software_agreement(generate_secret()).unwrap()).unwrap();
        assert!(decrypt_with(&theirs, &ciphertext).is_err());
    }

    #[test]
    fn a_refused_agreement_is_reported() {
        let ours = software_agreement(generate_secret()).unwrap();
        let ciphertext = encrypt_to(&recipient(ours.public_key()).unwrap(), b"epoch keys");
        let identity = AgreementIdentity::new(Arc::new(Refusing(ours.public_key()))).unwrap();
        assert!(decrypt_with(&identity, &ciphertext).is_err());
        assert_eq!(
            identity.take_failure(),
            Some(MobileError::key("authentication required"))
        );
        assert_eq!(identity.take_failure(), None);
    }

    #[test]
    fn malformed_stanzas_are_rejected() {
        let identity =
            AgreementIdentity::new(software_agreement(generate_secret()).unwrap()).unwrap();
        let stanza = |args: Vec<String>, body: Vec<u8>| Stanza {
            tag: X25519_TAG.into(),
            args,
            body,
        };
        let share = STANDARD_NO_PAD.encode([9; 32]);
        for bad in [
            stanza(vec![], vec![0; 32]),
            stanza(vec![share.clone(), share.clone()], vec![0; 32]),
            stanza(vec![STANDARD_NO_PAD.encode([9; 31])], vec![0; 32]),
            stanza(vec![format!("{share}=")], vec![0; 32]),
            stanza(vec![share], vec![0; 31]),
        ] {
            assert!(matches!(
                age::Identity::unwrap_stanza(&identity, &bad),
                Some(Err(age::DecryptError::InvalidHeader))
            ));
        }
        let other = Stanza {
            tag: "scrypt".into(),
            args: vec![],
            body: vec![],
        };
        assert!(age::Identity::unwrap_stanza(&identity, &other).is_none());
    }

    #[test]
    fn a_low_order_share_is_rejected() {
        let identity =
            AgreementIdentity::new(software_agreement(generate_secret()).unwrap()).unwrap();
        let stanza = Stanza {
            tag: X25519_TAG.into(),
            args: vec![STANDARD_NO_PAD.encode([0; 32])],
            body: vec![0; 32],
        };
        assert!(matches!(
            age::Identity::unwrap_stanza(&identity, &stanza),
            Some(Err(age::DecryptError::InvalidHeader))
        ));
    }

    #[test]
    fn the_adapter_signs_messages_that_verify() {
        let adapter = SignerAdapter::new(software_signer(generate_secret()).unwrap()).unwrap();
        let signature = adapter.sign_message(b"pasejo:request/v1\0hash").unwrap();
        let public = VerifyingKey::from_bytes(&adapter.verifying_key()).unwrap();
        assert!(
            public
                .verify_strict(
                    b"pasejo:request/v1\0hash",
                    &Signature::from_bytes(&signature)
                )
                .is_ok()
        );
    }

    #[test]
    fn the_adapter_refuses_a_signature_that_does_not_verify() {
        let adapter =
            SignerAdapter::new(Arc::new(Lying(software_signer(generate_secret()).unwrap())))
                .unwrap();
        assert!(matches!(
            adapter.sign_message(b"message"),
            Err(pasejo_protocol::Error::Signing(_))
        ));
    }

    #[test]
    fn seeds_of_the_wrong_length_are_refused() {
        assert!(software_signer(vec![1; 31]).is_err());
        assert!(software_agreement(vec![1; 33]).is_err());
        assert!(recipient(vec![1; 31]).is_err());
    }

    #[test]
    fn secrets_are_random() {
        assert_ne!(generate_secret(), generate_secret());
        assert_eq!(generate_secret().len(), 32);
    }
}
