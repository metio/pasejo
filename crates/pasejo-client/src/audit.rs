// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Reading and verifying an organization's audit trail, and co-signing its
//! checkpoints.

use std::collections::BTreeMap;

use pasejo_protocol::Id;
use pasejo_protocol::api::{AuditPage, CheckpointEntry, Checkpoints, Cosign};
use pasejo_protocol::checkpoint::{AuditEvent, CheckpointPayload, verify_chain};
use pasejo_protocol::encoding::{Fixed, Hash};
use pasejo_protocol::envelope::{Context, Signature, sign};
use pasejo_protocol::log::OrgLog;

use crate::error::{Error, Result};
use crate::transport::Client;

/// What a verification covered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// Events whose chain was recomputed.
    pub events: usize,
    /// Checkpoints whose server signature and hash matched.
    pub checkpoints: usize,
    /// Co-signatures by organization devices that verified, over every
    /// checkpoint the server returned.
    pub cosignatures: usize,
    /// The last verified event's sequence number and hash, to start from
    /// next time.
    pub last: Option<(u64, Hash)>,
}

const PAGE: u64 = 1000;

impl Client {
    /// One page of events after `since`.
    ///
    /// # Errors
    ///
    /// Fails for users who are neither admins nor auditors.
    pub fn audit_page(&self, org: Id, since: u64, limit: u64) -> Result<AuditPage> {
        self.get(&format!("/v1/orgs/{org}/audit?since={since}&limit={limit}"))
    }

    /// Every event after `since`.
    ///
    /// # Errors
    ///
    /// Fails like [`Client::audit_page`].
    pub fn audit_events(&self, org: Id, since: u64) -> Result<Vec<AuditEvent>> {
        let mut events = Vec::new();
        let mut cursor = since;
        loop {
            let page = self.audit_page(org, cursor, PAGE)?;
            events.extend(page.events);
            match page.next {
                Some(next) if next > cursor => cursor = next,
                _ => return Ok(events),
            }
        }
    }

    /// Checkpoints after `since`.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn checkpoints(&self, org: Id, since: u64) -> Result<Vec<CheckpointEntry>> {
        Ok(self
            .get::<Checkpoints>(&format!("/v1/orgs/{org}/audit/checkpoints?since={since}"))?
            .checkpoints)
    }

    /// Recomputes the organization's audit chain and checks every checkpoint
    /// against `server_key`, the key pinned when the client joined.
    ///
    /// Verification starts after `trusted` (a sequence number and hash from
    /// an earlier run), or at the first event, or at the checkpoint just
    /// before the first event the server still keeps after pruning. With a
    /// `log`, every checkpoint's log head must be part of it, and every
    /// co-signature must verify under the key of a device the log names.
    ///
    /// # Errors
    ///
    /// Fails when a signature, a hash or a log head does not match.
    pub fn verify_audit(
        &self,
        org: Id,
        server_key: &[u8; 32],
        log: Option<&OrgLog>,
        trusted: Option<(u64, Hash)>,
    ) -> Result<Verified> {
        let mut checkpoints = BTreeMap::new();
        let mut cosignatures = 0;
        for entry in self.checkpoints(org, 0)? {
            let payload = entry.checkpoint.verify_server(server_key).map_err(|_| {
                Error::verification(format!(
                    "checkpoint {} is not signed by the server",
                    entry.seq
                ))
            })?;
            check_payload(org, entry.seq, &payload, log)?;
            if let Some(log) = log {
                cosignatures += check_cosignatures(&entry, log)?;
            }
            checkpoints.insert(entry.seq, payload);
        }
        let since = trusted.map_or(0, |(seq, _)| seq);
        let events = self.audit_events(org, since)?;
        let Some(first) = events.first() else {
            return Ok(Verified {
                events: 0,
                checkpoints: checkpoints.len(),
                cosignatures,
                last: trusted,
            });
        };
        let start = match trusted {
            Some((seq, hash)) if seq + 1 == first.seq => hash,
            Some(_) => {
                return Err(Error::verification(
                    "events after the trusted point are missing",
                ));
            }
            None if first.seq == 1 => Hash::default(),
            None => checkpoints
                .get(&(first.seq - 1))
                .map(|payload| payload.hash)
                .ok_or_else(|| {
                    Error::verification("no checkpoint vouches for the first kept event")
                })?,
        };
        let last = verify_chain(start, first.seq, &events)?;
        let mut matched = 0;
        for event in &events {
            if let Some(payload) = checkpoints.get(&event.seq) {
                if payload.hash != event.hash {
                    return Err(Error::verification(format!(
                        "checkpoint {} contradicts the events",
                        event.seq
                    )));
                }
                matched += 1;
            }
        }
        let last_seq = events.last().map_or(since, |event| event.seq);
        Ok(Verified {
            events: events.len(),
            checkpoints: matched,
            cosignatures,
            last: Some((last_seq, last)),
        })
    }

    /// Co-signs every checkpoint that the server signed, whose log head is
    /// part of `log`, and that this device has not co-signed yet.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses a co-signature.
    pub fn cosign_checkpoints(&self, log: &OrgLog, server_key: &[u8; 32]) -> Result<usize> {
        let org = log.state().org;
        let mut signed = 0;
        for entry in self.checkpoints(org, 0)? {
            if entry
                .checkpoint
                .cosignatures
                .iter()
                .any(|existing| existing.device == self.device.id)
            {
                continue;
            }
            let Ok(payload) = entry.checkpoint.verify_server(server_key) else {
                continue;
            };
            if check_payload(org, entry.seq, &payload, Some(log)).is_err() {
                continue;
            }
            self.cosign_checkpoint(org, &entry)?;
            signed += 1;
        }
        Ok(signed)
    }

    /// Co-signs one checkpoint.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn cosign_checkpoint(&self, org: Id, entry: &CheckpointEntry) -> Result<()> {
        let signature = Signature {
            device: self.device.id,
            signature: Fixed(sign(
                Context::Checkpoint,
                &entry.checkpoint.payload.0,
                &self.device.signing_key,
            )),
        };
        self.call_empty(
            "POST",
            &format!("/v1/orgs/{org}/audit/checkpoints/{}/cosign", entry.seq),
            Some(&Cosign { signature }),
        )
    }
}

/// Checks every co-signature of `entry` against the signing key the log
/// records for its device, revoked devices included: a device co-signs
/// while it is active, and its later revocation does not undo that.
fn check_cosignatures(entry: &CheckpointEntry, log: &OrgLog) -> Result<usize> {
    for cosignature in &entry.checkpoint.cosignatures {
        let key = log
            .state()
            .devices
            .get(&cosignature.device)
            .map(|state| state.record.signing_key.0)
            .ok_or_else(|| {
                Error::verification(format!(
                    "checkpoint {} carries a co-signature by an unknown device",
                    entry.seq
                ))
            })?;
        entry
            .checkpoint
            .verify_cosignature(cosignature.device, &key)
            .map_err(|_| {
                Error::verification(format!(
                    "a co-signature of checkpoint {} does not verify",
                    entry.seq
                ))
            })?;
    }
    Ok(entry.checkpoint.cosignatures.len())
}

fn check_payload(
    org: Id,
    seq: u64,
    payload: &CheckpointPayload,
    log: Option<&OrgLog>,
) -> Result<()> {
    if payload.org != Some(org) || payload.seq != seq {
        return Err(Error::verification(format!(
            "checkpoint {seq} names another chain or position"
        )));
    }
    if let Some(log) = log {
        let known = payload
            .log_head
            .is_some_and(|head| log.seq_of(&head).is_some());
        if !known {
            return Err(Error::verification(format!(
                "checkpoint {seq} names a membership log head this client never saw"
            )));
        }
    }
    Ok(())
}
