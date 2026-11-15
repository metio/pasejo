// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use pasejo_protocol::api::{ErrorBody, ErrorCode};

/// An error answered with a JSON body carrying a stable code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub current_version: Option<u64>,
}

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            current_version: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    #[must_use]
    pub fn internal() -> Self {
        Self::new(ErrorCode::Internal, "internal error")
    }

    #[must_use]
    pub fn conflict(current_version: u64) -> Self {
        Self {
            code: ErrorCode::Conflict,
            message: "the store moved on".into(),
            current_version: Some(current_version),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (
            status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
                current_version: self.current_version,
            }),
        )
            .into_response();
        response.extensions_mut().insert(self.code);
        response
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database error");
        Self::internal()
    }
}

impl From<pasejo_protocol::Error> for ApiError {
    fn from(error: pasejo_protocol::Error) -> Self {
        Self::bad_request(error.to_string())
    }
}

impl From<pasejo_protocol::Rejection> for ApiError {
    fn from(rejection: pasejo_protocol::Rejection) -> Self {
        use pasejo_protocol::Rejection;
        // Another entry took the sequence number first: the client refreshes
        // the log and retries, as with a commit to an outdated version.
        let code = match rejection {
            Rejection::WrongSequence | Rejection::WrongPredecessor => ErrorCode::Conflict,
            _ => ErrorCode::Rejected,
        };
        Self::new(code, rejection.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(error: serde_json::Error) -> Self {
        Self::bad_request(format!("invalid JSON: {error}"))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        tracing::error!(error = %format!("{error:#}"), "internal error");
        Self::internal()
    }
}

/// Whether a database error reports a duplicate key.
#[must_use]
pub fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.kind() == sqlx::error::ErrorKind::UniqueViolation)
}
