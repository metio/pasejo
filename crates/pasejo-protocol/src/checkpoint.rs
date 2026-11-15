// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The audit log's hash chain and the checkpoints the server signs over it.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::encoding::{Bytes, Fixed, Hash, Id};
use crate::envelope::{Context, verify};
use crate::{Error, Millis};

/// One audit event as the server records and exports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub seq: u64,
    pub time: Millis,
    pub org: Option<Id>,
    pub actor_user: Option<Id>,
    pub actor_device: Option<Id>,
    pub session_kind: Option<String>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub action: String,
    pub store: Option<Id>,
    pub object: Option<Id>,
    pub version: Option<u64>,
    pub outcome: String,
    pub detail: Option<String>,
    pub prev_hash: Hash,
    pub hash: Hash,
}

impl AuditEvent {
    /// The hash this event must carry: SHA-256 over the predecessor's hash
    /// and the JSON serialization of every other field, in declaration
    /// order. Field order is fixed by this struct, so the serialization is
    /// deterministic.
    #[must_use]
    pub fn compute_hash(&self) -> Hash {
        let fields = HashedFields {
            seq: self.seq,
            time: self.time,
            org: self.org,
            actor_user: self.actor_user,
            actor_device: self.actor_device,
            session_kind: self.session_kind.as_deref(),
            ip: self.ip.as_deref(),
            user_agent: self.user_agent.as_deref(),
            action: &self.action,
            store: self.store,
            object: self.object,
            version: self.version,
            outcome: &self.outcome,
            detail: self.detail.as_deref(),
        };
        let mut digest = Sha256::new();
        digest.update(self.prev_hash.0);
        digest.update(serde_json::to_vec(&fields).unwrap_or_default());
        Hash(digest.finalize().into())
    }
}

#[derive(Serialize)]
struct HashedFields<'a> {
    seq: u64,
    time: Millis,
    org: Option<Id>,
    actor_user: Option<Id>,
    actor_device: Option<Id>,
    session_kind: Option<&'a str>,
    ip: Option<&'a str>,
    user_agent: Option<&'a str>,
    action: &'a str,
    store: Option<Id>,
    object: Option<Id>,
    version: Option<u64>,
    outcome: &'a str,
    detail: Option<&'a str>,
}

/// Verifies that `events` form an unbroken chain starting after `start`,
/// the hash of the event before the first one (or of a checkpoint).
///
/// # Errors
///
/// Fails at the first event whose predecessor hash, sequence number or own
/// hash does not match.
pub fn verify_chain(start: Hash, first_seq: u64, events: &[AuditEvent]) -> Result<Hash, Error> {
    let mut previous = start;
    for (offset, event) in events.iter().enumerate() {
        if event.seq != first_seq + offset as u64 {
            return Err(Error::Invalid("audit sequence has a gap"));
        }
        if event.prev_hash != previous {
            return Err(Error::Invalid("audit chain is broken"));
        }
        if event.compute_hash() != event.hash {
            return Err(Error::Invalid("audit event was altered"));
        }
        previous = event.hash;
    }
    Ok(previous)
}

/// The signed statement that the chain of `org` reached `seq` with `hash`
/// while its membership log had head `log_head`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointPayload {
    pub org: Id,
    pub seq: u64,
    pub hash: Hash,
    pub log_head: Option<Hash>,
    pub time: Millis,
}

/// A checkpoint with the server's signature and the organization's
/// co-signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub payload: Bytes,
    pub server_signature: Fixed<64>,
    pub cosignatures: Vec<crate::envelope::Signature>,
}

impl Checkpoint {
    /// Parses the payload after checking the server's signature.
    ///
    /// # Errors
    ///
    /// Fails when the server's signature does not verify.
    pub fn verify_server(&self, server_key: &[u8; 32]) -> Result<CheckpointPayload, Error> {
        verify(
            Context::Checkpoint,
            &self.payload.0,
            server_key,
            &self.server_signature.0,
        )?;
        Ok(serde_json::from_slice(&self.payload.0)?)
    }

    /// Checks one co-signature by `device`.
    ///
    /// # Errors
    ///
    /// Fails when `device` did not co-sign or its signature does not verify.
    pub fn verify_cosignature(&self, device: Id, key: &[u8; 32]) -> Result<(), Error> {
        let signature = self
            .cosignatures
            .iter()
            .find(|signature| signature.device == device)
            .ok_or(Error::Signature)?;
        verify(
            Context::Checkpoint,
            &self.payload.0,
            key,
            &signature.signature.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;

    use super::*;
    use crate::envelope::sign;

    fn event(seq: u64, prev: Hash) -> AuditEvent {
        let mut event = AuditEvent {
            seq,
            time: 1000 + seq,
            org: Some(Id([1; 16])),
            actor_user: None,
            actor_device: None,
            session_kind: Some("cli".into()),
            ip: Some("127.0.0.1".into()),
            user_agent: None,
            action: "object.read".into(),
            store: Some(Id([2; 16])),
            object: Some(Id([3; 16])),
            version: Some(seq),
            outcome: "ok".into(),
            detail: None,
            prev_hash: prev,
            hash: Hash::default(),
        };
        event.hash = event.compute_hash();
        event
    }

    fn chain(length: u64) -> Vec<AuditEvent> {
        let mut events = Vec::new();
        let mut previous = Hash::default();
        for seq in 1..=length {
            let next = event(seq, previous);
            previous = next.hash;
            events.push(next);
        }
        events
    }

    #[test]
    fn intact_chains_verify() {
        let events = chain(5);
        assert_eq!(
            verify_chain(Hash::default(), 1, &events).unwrap(),
            events[4].hash
        );
        assert_eq!(
            verify_chain(events[1].hash, 3, &events[2..]).unwrap(),
            events[4].hash
        );
    }

    #[test]
    fn altered_reordered_or_missing_events_fail() {
        let mut events = chain(4);
        events[2].outcome = "denied".into();
        assert!(verify_chain(Hash::default(), 1, &events).is_err());

        let mut events = chain(4);
        events.swap(1, 2);
        assert!(verify_chain(Hash::default(), 1, &events).is_err());

        let mut events = chain(4);
        events.remove(1);
        assert!(verify_chain(Hash::default(), 1, &events).is_err());

        let events = chain(3);
        assert!(verify_chain(Hash::of(b"other start"), 1, &events).is_err());
    }

    #[test]
    fn every_field_is_covered_by_the_hash() {
        let base = event(1, Hash::default());
        let mut changed = base.clone();
        changed.ip = Some("10.0.0.1".into());
        assert_ne!(changed.compute_hash(), base.compute_hash());
        let mut changed = base.clone();
        changed.detail = Some("x".into());
        assert_ne!(changed.compute_hash(), base.compute_hash());
        let mut changed = base.clone();
        changed.prev_hash = Hash::of(b"p");
        assert_ne!(changed.compute_hash(), base.compute_hash());
    }

    #[test]
    fn checkpoints_verify_the_server_and_cosigners() {
        let server = SigningKey::generate(&mut OsRng);
        let device = SigningKey::generate(&mut OsRng);
        let device_id = Id::random();
        let payload = CheckpointPayload {
            org: Id::random(),
            seq: 10,
            hash: Hash::of(b"h"),
            log_head: Some(Hash::of(b"l")),
            time: 1,
        };
        let bytes = serde_json::to_vec(&payload).unwrap();
        let checkpoint = Checkpoint {
            server_signature: Fixed(sign(Context::Checkpoint, &bytes, &server)),
            cosignatures: vec![crate::envelope::Signature {
                device: device_id,
                signature: Fixed(sign(Context::Checkpoint, &bytes, &device)),
            }],
            payload: Bytes(bytes),
        };
        assert_eq!(
            checkpoint
                .verify_server(server.verifying_key().as_bytes())
                .unwrap(),
            payload
        );
        checkpoint
            .verify_cosignature(device_id, device.verifying_key().as_bytes())
            .unwrap();
        assert!(
            checkpoint
                .verify_server(device.verifying_key().as_bytes())
                .is_err()
        );
        assert!(
            checkpoint
                .verify_cosignature(Id::random(), device.verifying_key().as_bytes())
                .is_err()
        );
    }
}
