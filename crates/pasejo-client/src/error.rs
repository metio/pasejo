// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Errors a client call can end with.

use pasejo_protocol::api::ErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The server refused the request.
    #[error("the server answered {status}: {message}")]
    Api {
        status: u16,
        code: Option<ErrorCode>,
        message: String,
        current_version: Option<u64>,
    },
    /// The request did not reach the server or its answer did not arrive.
    #[error("cannot reach the server: {0}")]
    Transport(String),
    /// Something the server returned failed verification.
    #[error("verification failed: {0}")]
    Verification(String),
    /// A protocol operation failed.
    #[error(transparent)]
    Protocol(#[from] pasejo_protocol::Error),
    /// The call needs a session and the client has none.
    #[error("not signed in")]
    NoSession,
    /// The call needs something this device does not hold.
    #[error("{0}")]
    Unavailable(String),
}

impl Error {
    /// The server's error code, when the server answered with one.
    #[must_use]
    pub const fn code(&self) -> Option<ErrorCode> {
        match self {
            Self::Api { code, .. } => *code,
            _ => None,
        }
    }

    /// Whether the server answered with `code`.
    #[must_use]
    pub fn is(&self, code: ErrorCode) -> bool {
        self.code() == Some(code)
    }

    pub(crate) fn verification(message: impl Into<String>) -> Self {
        Self::Verification(message.into())
    }
}

impl From<pasejo_protocol::Rejection> for Error {
    fn from(rejection: pasejo_protocol::Rejection) -> Self {
        Self::Verification(format!("the membership log is invalid: {rejection}"))
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
