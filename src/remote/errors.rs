// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Localized messages for what the client reports. The server's own text
//! and the client's diagnostics stay in the error chain below them.

use pasejo_client::Error;
use pasejo_protocol::api::ErrorCode;

use crate::cli::i18n;

/// The localized message for `error`.
pub fn message(error: &Error) -> String {
    match error {
        Error::Api {
            code: Some(code), ..
        } => match code {
            ErrorCode::Unauthorized | ErrorCode::InvalidProof | ErrorCode::ExpiredToken => {
                i18n::error_remote_session_rejected()
            }
            ErrorCode::Forbidden => i18n::error_remote_forbidden(),
            ErrorCode::PendingDevice => i18n::error_remote_pending_device(),
            ErrorCode::NotFound | ErrorCode::UnknownOrg | ErrorCode::UnknownStore => {
                i18n::error_remote_not_found()
            }
            ErrorCode::Conflict => i18n::error_remote_conflict(),
            ErrorCode::Rejected => i18n::error_remote_rejected(),
            ErrorCode::RateLimited | ErrorCode::Locked => i18n::error_remote_rate_limited(),
            ErrorCode::TooLarge => i18n::error_remote_too_large(),
            ErrorCode::AccessDenied => i18n::error_remote_login_denied(),
            ErrorCode::BadRequest | ErrorCode::AuthorizationPending | ErrorCode::SlowDown => {
                i18n::error_remote_bad_request()
            }
            ErrorCode::Unavailable | ErrorCode::Internal => i18n::error_remote_server_failed(),
        },
        Error::Api { code: None, .. } => i18n::error_remote_server_failed(),
        Error::Transport(_) => i18n::error_remote_unreachable(),
        Error::Verification(_) | Error::Protocol(_) => i18n::error_remote_verification_failed(),
        Error::NoSession => i18n::error_remote_no_session(),
        Error::Unavailable(_) => i18n::error_remote_unavailable(),
        Error::NeedsSecondAdmin => i18n::error_remote_needs_second_admin(),
        Error::Refused(_) => i18n::error_remote_rejected(),
    }
}

/// Turns client results into `anyhow` results carrying a localized
/// message above the client's error.
pub trait Localize<T> {
    /// # Errors
    ///
    /// Passes on the client's error with a localized context.
    fn localized(self) -> anyhow::Result<T>;
}

impl<T> Localize<T> for pasejo_client::Result<T> {
    fn localized(self) -> anyhow::Result<T> {
        self.map_err(|error| {
            let message = message(&error);
            anyhow::Error::new(error).context(message)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_maps_to_a_localized_message() {
        i18n::init_for_tests();
        let api = |code| Error::Api {
            status: 400,
            code,
            message: "detail".into(),
            current_version: None,
        };
        let errors = [
            api(Some(ErrorCode::Forbidden)),
            api(Some(ErrorCode::Conflict)),
            api(None),
            Error::Transport("down".into()),
            Error::Verification("bad".into()),
            Error::NoSession,
            Error::Unavailable("x".into()),
            Error::NeedsSecondAdmin,
            Error::Refused(pasejo_protocol::Rejection::SecondAdminRequired),
        ];
        for error in errors {
            let text = message(&error);
            assert!(!text.contains("No localization"), "{text}");
        }
        let chained = Err::<(), _>(Error::Verification("index hash".into()))
            .localized()
            .unwrap_err();
        assert!(format!("{chained:#}").contains("index hash"));
    }
}
