// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Running the pending actions the server computes.

use pasejo_protocol::api::Action;
use pasejo_protocol::log::{Approval, Body, OrgLog, RoleChange, StoreRoleChange};

use crate::error::{Error, Result};
use crate::store::Changes;
use crate::transport::Client;

/// What running one action did.
#[derive(Debug)]
pub enum Outcome {
    Done(Action),
    /// Left for a person, with the reason.
    Skipped(Action, String),
    Failed(Action, Error),
}

impl Client {
    /// Runs every pending action this device can, refreshing `log` as it
    /// goes. Device approvals run only in `automatic` mode and only for
    /// members; in `manual` mode an approver compares fingerprints first.
    ///
    /// # Errors
    ///
    /// Fails when the action list or the log cannot be fetched; a failing
    /// action is reported in its outcome and the rest still run.
    pub fn run_actions(
        &self,
        log: &mut OrgLog,
        identities: &[Box<dyn age::Identity>],
        server_key: &[u8; 32],
    ) -> Result<Vec<Outcome>> {
        self.run_actions_where(log, identities, server_key, |_| true)
    }

    /// [`Client::run_actions`] for the actions `wanted` selects.
    ///
    /// # Errors
    ///
    /// Fails like [`Client::run_actions`].
    pub fn run_actions_where(
        &self,
        log: &mut OrgLog,
        identities: &[Box<dyn age::Identity>],
        server_key: &[u8; 32],
        wanted: impl Fn(&Action) -> bool,
    ) -> Result<Vec<Outcome>> {
        let org = log.state().org;
        *log = self.fetch_log(org, Some(log.clone()), None)?;
        let mut outcomes = Vec::new();
        for action in self
            .actions(org)?
            .into_iter()
            .filter(|action| wanted(action))
        {
            let result = self.run_action(log, &action, identities, server_key);
            outcomes.push(match result {
                Ok(None) => Outcome::Done(action),
                Ok(Some(reason)) => Outcome::Skipped(action, reason),
                Err(error) => Outcome::Failed(action, error),
            });
        }
        Ok(outcomes)
    }

    fn run_action(
        &self,
        log: &mut OrgLog,
        action: &Action,
        identities: &[Box<dyn age::Identity>],
        server_key: &[u8; 32],
    ) -> Result<Option<String>> {
        match action {
            Action::ApproveDevice { enrollment } => {
                let state = log.state();
                if state.settings.device_approval != Approval::Automatic {
                    return Ok(Some(format!(
                        "compare fingerprint {} before approving",
                        enrollment.fingerprint
                    )));
                }
                if !state.members.contains_key(&enrollment.device.user) {
                    return Ok(Some("the device's user is no member".into()));
                }
                self.append_fresh(
                    log,
                    Body::DeviceAdd {
                        device: enrollment.device.clone(),
                    },
                )?;
            }
            Action::JoinMember { user, .. } => {
                self.append_fresh(
                    log,
                    Body::MemberRole {
                        user: *user,
                        role: RoleChange::Member,
                    },
                )?;
            }
            Action::RemoveMember { user } => {
                self.append_fresh(
                    log,
                    Body::MemberRole {
                        user: *user,
                        role: RoleChange::None,
                    },
                )?;
            }
            Action::SyncTeam {
                team,
                name,
                exists,
                add,
                remove,
            } => {
                if !exists {
                    self.append_fresh(
                        log,
                        Body::TeamCreate {
                            team: *team,
                            name: name.clone(),
                        },
                    )?;
                }
                for user in add {
                    self.append_fresh(
                        log,
                        Body::TeamMember {
                            team: *team,
                            user: *user,
                            member: true,
                        },
                    )?;
                }
                for user in remove {
                    self.append_fresh(
                        log,
                        Body::TeamMember {
                            team: *team,
                            user: *user,
                            member: false,
                        },
                    )?;
                }
            }
            Action::RotateEpoch { store, epoch } => self.rotate(log, *store, *epoch, identities)?,
            Action::WrapKeys { store, devices, .. } => {
                let ring = self.keys(log, *store, identities)?;
                self.wrap(log, &ring, devices)?;
            }
            Action::RevokeExpired { store, principal }
            | Action::RevokeGrant {
                store, principal, ..
            } => {
                self.grant(log, *store, *principal, StoreRoleChange::None, None)?;
            }
            Action::CosignCheckpoint { seq } => return self.cosign_due(log, *seq, server_key),
        }
        Ok(None)
    }

    /// Starts the store's next epoch and re-encrypts the index under it,
    /// which hides names added from now on from the removed party.
    fn rotate(
        &self,
        log: &mut OrgLog,
        store: pasejo_protocol::Id,
        epoch: u64,
        identities: &[Box<dyn age::Identity>],
    ) -> Result<()> {
        if epoch == 0 {
            self.start_epoch(log, store, None)?;
            return Ok(());
        }
        let ring = self.keys(log, store, identities)?;
        let snapshot = self.index(log, &ring, None)?;
        let next = self.start_epoch(log, store, Some(&ring))?;
        if snapshot.version > 0 {
            self.commit(log, &next, &snapshot, &Changes::default())?;
        }
        Ok(())
    }

    /// Co-signs checkpoint `seq` when the server signed it and its log head
    /// is part of `log`.
    fn cosign_due(&self, log: &OrgLog, seq: u64, server_key: &[u8; 32]) -> Result<Option<String>> {
        let org = log.state().org;
        let entry = self
            .checkpoints(org, seq.saturating_sub(1))?
            .into_iter()
            .find(|entry| entry.seq == seq)
            .ok_or_else(|| Error::Unavailable(format!("checkpoint {seq} is gone")))?;
        let payload = entry
            .checkpoint
            .verify_server(server_key)
            .map_err(|_| Error::verification("the checkpoint is not signed by the server"))?;
        if payload
            .log_head
            .is_none_or(|head| log.seq_of(&head).is_none())
        {
            return Ok(Some(
                "the checkpoint names a log head this device never saw".into(),
            ));
        }
        self.cosign_checkpoint(org, &entry)?;
        Ok(None)
    }
}
