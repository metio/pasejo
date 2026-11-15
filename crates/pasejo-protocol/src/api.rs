// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Request and response bodies of the HTTP API under `/v1`.

use serde::{Deserialize, Serialize};

use crate::Millis;
use crate::checkpoint::{AuditEvent, Checkpoint};
use crate::device::{DeviceKind, DeviceRecord};
use crate::encoding::{Bytes, Fixed, Hash, Id};
use crate::envelope::Envelope;
use crate::log::{Principal, StoreRole};
use crate::store::SealedObject;

/// Stable error codes. Clients translate them; the message is English
/// diagnostics for logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    InvalidProof,
    Forbidden,
    PendingDevice,
    NotFound,
    UnknownOrg,
    UnknownStore,
    Conflict,
    Rejected,
    RateLimited,
    Locked,
    TooLarge,
    AuthorizationPending,
    SlowDown,
    ExpiredToken,
    AccessDenied,
    Unavailable,
    Internal,
}

impl ErrorCode {
    #[must_use]
    pub const fn status(self) -> u16 {
        match self {
            Self::BadRequest
            | Self::Rejected
            | Self::AuthorizationPending
            | Self::SlowDown
            | Self::ExpiredToken
            | Self::AccessDenied => 400,
            Self::Unauthorized | Self::InvalidProof => 401,
            Self::Forbidden | Self::PendingDevice => 403,
            Self::NotFound | Self::UnknownOrg | Self::UnknownStore => 404,
            Self::Conflict => 409,
            Self::TooLarge => 413,
            Self::Locked => 423,
            Self::RateLimited => 429,
            Self::Unavailable => 503,
            Self::Internal => 500,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    /// The store's current index version, on `conflict`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    pub checkpoint_key: Fixed<32>,
    pub oidc: bool,
    pub break_glass: bool,
    pub emergency_until: Option<Millis>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthorizationRequest {
    pub signing_key: Fixed<32>,
    pub label: String,
}

/// RFC 8628 device authorization response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// A `token/v1` envelope over a [`crate::envelope::TokenClaim`]. The
/// envelope's device field is unused for claims, which name their key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRequest {
    pub claim: Envelope,
    /// The workload JWT, for `workload` claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires: Millis,
    pub user: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgSummary {
    pub id: Id,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Me {
    pub user: Id,
    pub name: String,
    pub email: String,
    pub machine: bool,
    pub orgs: Vec<OrgSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateOrg {
    pub genesis: Envelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogPage {
    pub entries: Vec<Envelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Append {
    pub entry: Envelope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appended {
    pub seq: u64,
    pub head: Hash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Id,
    pub entry: Envelope,
    pub created: Millis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposals {
    pub proposals: Vec<Proposal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cosign {
    pub signature: crate::envelope::Signature,
}

/// What a device asks to be enrolled as. The server fills in the user from
/// the session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrollmentRequest {
    pub id: Id,
    pub kind: DeviceKind,
    pub recipient: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enrollment {
    pub device: DeviceRecord,
    pub fingerprint: String,
    pub requested: Millis,
    pub user_name: String,
    pub user_email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enrollments {
    pub enrollments: Vec<Enrollment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: Id,
    pub name: String,
    pub email: String,
    pub machine: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Users {
    pub users: Vec<UserSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreSummary {
    pub id: Id,
    pub epoch: Option<u64>,
    pub version: u64,
    pub role: Option<StoreRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stores {
    pub stores: Vec<StoreSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wrapping {
    pub epoch: u64,
    pub device: Id,
    pub full: bool,
    pub ciphertext: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainLink {
    pub epoch: u64,
    pub prev_names: Bytes,
    pub prev_data: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keys {
    pub wrappings: Vec<Wrapping>,
    pub chain: Vec<ChainLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappingUpload {
    pub device: Id,
    /// Whether the wrapping carries the data key.
    pub full: bool,
    pub ciphertext: Bytes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochUpload {
    pub chain: Option<ChainLink>,
    pub wrappings: Vec<WrappingUpload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappingsUpload {
    pub epoch: u64,
    pub wrappings: Vec<WrappingUpload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexResponse {
    pub index: Option<Envelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LookupResponse {
    pub version: u64,
    pub lookup: Option<Bytes>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub id: Id,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectsRequest {
    pub objects: Vec<ObjectRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectsResponse {
    pub objects: Vec<SealedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    pub index: Envelope,
    pub lookup: Option<Bytes>,
    pub objects: Vec<SealedObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committed {
    pub version: u64,
}

/// One pending action; see the design's "Pending actions".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum Action {
    ApproveDevice {
        enrollment: Enrollment,
    },
    JoinMember {
        user: Id,
        name: String,
        email: String,
    },
    RemoveMember {
        user: Id,
    },
    SyncTeam {
        team: Id,
        name: String,
        exists: bool,
        add: Vec<Id>,
        remove: Vec<Id>,
    },
    RotateEpoch {
        store: Id,
        epoch: u64,
    },
    WrapKeys {
        store: Id,
        epoch: u64,
        devices: Vec<Id>,
    },
    RevokeExpired {
        store: Id,
        principal: Principal,
    },
    RevokeGrant {
        store: Id,
        principal: Principal,
        review: Id,
    },
    CosignCheckpoint {
        seq: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actions {
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotationItem {
    pub id: Id,
    pub fetched: bool,
    pub rewritten: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rotation {
    pub user: Id,
    /// When the user's access ended, or `None` while it lasts.
    pub ended: Option<Millis>,
    pub items: Vec<RotationItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPage {
    pub events: Vec<AuditEvent>,
    pub next: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointEntry {
    pub seq: u64,
    pub checkpoint: Checkpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoints {
    pub checkpoints: Vec<CheckpointEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub store: Id,
    pub reviewers: Vec<Id>,
    pub due: Millis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Keep,
    Revoke,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewItem {
    pub principal: Principal,
    pub role: StoreRole,
    pub decision: Option<Decision>,
    pub decided_by: Option<Id>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub id: Id,
    pub store: Id,
    pub reviewers: Vec<Id>,
    pub due: Millis,
    pub created: Millis,
    pub created_by: Id,
    pub items: Vec<ReviewItem>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reviews {
    pub reviews: Vec<Review>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewDecision {
    pub principal: Principal,
    pub decision: Decision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScimTokenCreated {
    pub id: Id,
    pub token: String,
}

/// A live SCIM token, without its secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScimTokenInfo {
    pub id: Id,
    pub created: Millis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScimTokens {
    pub tokens: Vec<ScimTokenInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakGlassLogin {
    pub username: String,
    pub password: String,
    pub totp: String,
    pub signing_key: Fixed<32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergencyWindow {
    /// Milliseconds the window stays open, at most 24 hours; 0 closes it.
    pub duration: Millis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergencyState {
    pub until: Option<Millis>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgList {
    pub orgs: Vec<OrgSummary>,
}

/// Purposes of a [`crate::envelope::TokenClaim`].
pub mod purpose {
    pub const DEVICE: &str = "device";
    pub const WEB: &str = "web";
    pub const WORKLOAD: &str = "workload";
    pub const EMERGENCY: &str = "emergency";
    pub const BREAK_GLASS: &str = "break-glass";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrollmentStatus {
    pub pending: bool,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BreakGlassToken {
    pub token: String,
    pub expires: Millis,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_serialize_snake_case_and_map_to_statuses() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::UnknownStore).unwrap(),
            "\"unknown_store\""
        );
        assert_eq!(ErrorCode::Conflict.status(), 409);
        assert_eq!(ErrorCode::InvalidProof.status(), 401);
        assert_eq!(ErrorCode::PendingDevice.status(), 403);
        assert_eq!(ErrorCode::RateLimited.status(), 429);
    }

    #[test]
    fn actions_are_tagged_in_kebab_case() {
        let action = Action::RotateEpoch {
            store: Id([0; 16]),
            epoch: 2,
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["action"], "rotate-epoch");
        assert_eq!(serde_json::from_value::<Action>(json).unwrap(), action);
    }

    #[test]
    fn conflict_bodies_carry_the_current_version_only_when_set() {
        let body = ErrorBody {
            code: ErrorCode::Conflict,
            message: "x".into(),
            current_version: None,
        };
        assert!(
            !serde_json::to_string(&body)
                .unwrap()
                .contains("current_version")
        );
    }
}
