// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

/// Why a call failed. `Verification` means something the server returned
/// did not verify and must never be retried silently or shown as an outage.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum MobileError {
    #[error("cannot reach the server: {reason}")]
    Network { reason: String },
    #[error("the server refused: {reason}")]
    Refused { reason: String },
    #[error("verification failed: {reason}")]
    Verification { reason: String },
    #[error("not signed in")]
    NotSignedIn,
    #[error("the stores are locked")]
    Locked,
    #[error("not found")]
    NotFound,
    #[error("the device key failed: {reason}")]
    Key { reason: String },
    #[error("{reason}")]
    Invalid { reason: String },
}

impl MobileError {
    pub(crate) fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid {
            reason: reason.into(),
        }
    }

    pub(crate) fn verification(reason: impl Into<String>) -> Self {
        Self::Verification {
            reason: reason.into(),
        }
    }

    pub(crate) fn key(reason: impl Into<String>) -> Self {
        Self::Key {
            reason: reason.into(),
        }
    }
}

impl From<pasejo_client::Error> for MobileError {
    fn from(error: pasejo_client::Error) -> Self {
        use pasejo_client::Error;
        use pasejo_protocol::api::ErrorCode;
        match error {
            Error::Transport(reason) => Self::Network { reason },
            Error::Api {
                code: Some(ErrorCode::Unauthorized),
                ..
            }
            | Error::NoSession => Self::NotSignedIn,
            Error::Api {
                code: Some(ErrorCode::NotFound),
                ..
            } => Self::NotFound,
            Error::Api { .. } | Error::NeedsSecondAdmin | Error::Refused(_) => Self::Refused {
                reason: error.to_string(),
            },
            Error::Protocol(pasejo_protocol::Error::Signing(reason)) => Self::Key { reason },
            Error::Verification(_) | Error::Protocol(_) => Self::verification(error.to_string()),
            Error::Unavailable(reason) => Self::Invalid { reason },
        }
    }
}

impl From<pasejo_protocol::Error> for MobileError {
    fn from(error: pasejo_protocol::Error) -> Self {
        pasejo_client::Error::from(error).into()
    }
}

/// A foreign implementation of a key trait threw something other than a
/// `MobileError`.
impl From<uniffi::UnexpectedUniFFICallbackError> for MobileError {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::key(error.reason)
    }
}
