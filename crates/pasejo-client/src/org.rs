// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Organizations: creation, the verified membership log, enrollment,
//! appends, two-admin proposals, members and SCIM tokens.

use pasejo_protocol::Id;
use pasejo_protocol::api::{
    Action, Actions, Append, Appended, Cosign, CreateOrg, Enrollment, EnrollmentRequest,
    EnrollmentStatus, Enrollments, LogPage, OrgSummary, Proposal, Proposals, ScimTokenCreated,
    ScimTokenInfo, ScimTokens, UserSummary, Users,
};
use pasejo_protocol::encoding::{Fixed, Hash};
use pasejo_protocol::envelope::{Context, Signature, try_sign};
use pasejo_protocol::log::{Body, Entry, OrgLog, Settings, genesis, next_entry, seal_entry};

use crate::error::{Error, Result};
use crate::transport::Client;

impl Client {
    /// Creates organization `org` with this device as its founding device.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, for instance because creation is
    /// restricted or the identifier is taken.
    pub fn create_org(&self, org: Id, name: &str, settings: Settings) -> Result<OrgLog> {
        let user = self.user()?;
        let envelope = genesis(
            org,
            name,
            self.device.record(user),
            settings,
            self.now(),
            &*self.device.signer,
        )?;
        let _: OrgSummary = self.call(
            "POST",
            "/v1/orgs",
            &CreateOrg {
                genesis: envelope.clone(),
            },
        )?;
        Ok(OrgLog::genesis(envelope)?)
    }

    /// Fetches and verifies an organization's membership log.
    ///
    /// With `known`, only entries after it are fetched and they must extend
    /// it. Without, the whole log is replayed, and its genesis must hash to
    /// `pinned_genesis` when one is given.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, or when the entries do not replay,
    /// fork from `known` or start from another genesis.
    pub fn fetch_log(
        &self,
        org: Id,
        known: Option<OrgLog>,
        pinned_genesis: Option<Hash>,
    ) -> Result<OrgLog> {
        let path = known.as_ref().map_or_else(
            || format!("/v1/orgs/{org}/log"),
            |log| format!("/v1/orgs/{org}/log?since={}", log.entries().len() - 1),
        );
        let page: LogPage = self.get(&path)?;
        let log = match known {
            Some(mut log) => {
                for envelope in page.entries {
                    log.append(envelope)?;
                }
                log
            }
            None => OrgLog::replay(page.entries).map_err(|(seq, rejection)| {
                Error::verification(format!("membership log entry {seq}: {rejection}"))
            })?,
        };
        if log.state().org != org {
            return Err(Error::verification(
                "the log belongs to another organization",
            ));
        }
        if let Some(pin) = pinned_genesis
            && log.genesis_hash() != pin
        {
            return Err(Error::verification(
                "the organization's genesis differs from the pinned one",
            ));
        }
        Ok(log)
    }

    fn entry(&self, log: &OrgLog, body: Body) -> Entry {
        next_entry(log, self.device.id, self.now(), body)
    }

    /// Signs `body` as the next entry and appends it, extending `log` with
    /// it on success.
    ///
    /// # Errors
    ///
    /// Fails when the log rejects the entry locally, or the server refuses
    /// it, for instance because another entry came first.
    pub fn append(&self, log: &mut OrgLog, body: Body) -> Result<Appended> {
        let envelope = seal_entry(&self.entry(log, body), &*self.device.signer)?;
        let mut trial = log.clone();
        trial.append(envelope.clone()).map_err(Error::refused)?;
        let org = log.state().org;
        let appended: Appended = self.call(
            "POST",
            &format!("/v1/orgs/{org}/log"),
            &Append { entry: envelope },
        )?;
        *log = trial;
        Ok(appended)
    }

    /// Appends `body`, refreshing the log and retrying once when another
    /// entry took the next sequence number first.
    ///
    /// # Errors
    ///
    /// Fails like [`Client::append`].
    pub fn append_fresh(&self, log: &mut OrgLog, body: Body) -> Result<Appended> {
        match self.append(log, body.clone()) {
            Err(error) if error.is(pasejo_protocol::api::ErrorCode::Conflict) => {
                let org = log.state().org;
                *log = self.fetch_log(org, Some(log.clone()), None)?;
                self.append(log, body)
            }
            other => other,
        }
    }

    /// Proposes an entry that needs a second admin.
    ///
    /// # Errors
    ///
    /// Fails when the entry needs no second admin or the server refuses.
    pub fn propose(&self, log: &OrgLog, body: Body) -> Result<Proposal> {
        let envelope = seal_entry(&self.entry(log, body), &*self.device.signer)?;
        let org = log.state().org;
        self.call(
            "POST",
            &format!("/v1/orgs/{org}/log/proposals"),
            &Append { entry: envelope },
        )
    }

    /// Open proposals, each with its entry decoded for review.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses or a proposal is malformed.
    pub fn proposals(&self, org: Id) -> Result<Vec<(Proposal, Entry)>> {
        let proposals: Proposals = self.get(&format!("/v1/orgs/{org}/log/proposals"))?;
        proposals
            .proposals
            .into_iter()
            .map(|proposal| {
                let entry = proposal.entry.open()?;
                Ok((proposal, entry))
            })
            .collect()
    }

    /// Co-signs a proposal; the server appends it once replay accepts both
    /// signatures, and `log` is refreshed to include it.
    ///
    /// # Errors
    ///
    /// Fails when the proposal is stale or the server refuses.
    pub fn cosign_proposal(&self, log: &mut OrgLog, proposal: &Proposal) -> Result<Appended> {
        let org = log.state().org;
        let signature = Signature {
            device: self.device.id,
            signature: Fixed(try_sign(
                Context::LogEntry,
                &proposal.entry.payload.0,
                &*self.device.signer,
            )?),
        };
        let appended: Appended = self.call(
            "POST",
            &format!("/v1/orgs/{org}/log/proposals/{}/cosign", proposal.id),
            &Cosign { signature },
        )?;
        *log = self.fetch_log(org, Some(log.clone()), None)?;
        Ok(appended)
    }

    /// Asks to enroll this device in `org`.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn enroll(&self, org: Id) -> Result<()> {
        let request = EnrollmentRequest {
            id: self.device.id,
            kind: self.device.kind,
            recipient: self.device.recipient.clone(),
            label: self.device.label.clone(),
        };
        let _: serde_json::Value =
            self.call("POST", &format!("/v1/orgs/{org}/enrollments"), &request)?;
        Ok(())
    }

    /// Whether this device is pending or active in `org`.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn enrollment_status(&self, org: Id) -> Result<EnrollmentStatus> {
        self.get(&format!("/v1/orgs/{org}/enrollments/{}", self.device.id))
    }

    /// Devices waiting for approval.
    ///
    /// # Errors
    ///
    /// Fails for devices that may not approve.
    pub fn enrollments(&self, org: Id) -> Result<Vec<Enrollment>> {
        Ok(self
            .get::<Enrollments>(&format!("/v1/orgs/{org}/enrollments"))?
            .enrollments)
    }

    /// Names and emails of the organization's members.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn users(&self, org: Id) -> Result<Vec<UserSummary>> {
        Ok(self.get::<Users>(&format!("/v1/orgs/{org}/users"))?.users)
    }

    /// Pending actions this device's user could resolve.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses.
    pub fn actions(&self, org: Id) -> Result<Vec<Action>> {
        Ok(self
            .get::<Actions>(&format!("/v1/orgs/{org}/actions"))?
            .actions)
    }

    /// Creates a SCIM token; the token is returned only now.
    ///
    /// # Errors
    ///
    /// Fails for non-admins.
    pub fn create_scim_token(&self, org: Id) -> Result<ScimTokenCreated> {
        self.call("POST", &format!("/v1/orgs/{org}/scim-tokens"), &())
    }

    /// Live SCIM tokens.
    ///
    /// # Errors
    ///
    /// Fails for non-admins.
    pub fn scim_tokens(&self, org: Id) -> Result<Vec<ScimTokenInfo>> {
        Ok(self
            .get::<ScimTokens>(&format!("/v1/orgs/{org}/scim-tokens"))?
            .tokens)
    }

    /// Revokes a SCIM token.
    ///
    /// # Errors
    ///
    /// Fails for non-admins or unknown tokens.
    pub fn revoke_scim_token(&self, org: Id, token: Id) -> Result<()> {
        self.call_empty::<()>(
            "DELETE",
            &format!("/v1/orgs/{org}/scim-tokens/{token}"),
            None,
        )
    }
}
