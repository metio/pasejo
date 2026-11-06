// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::ffi::OsString;

use age::secrecy::SecretString;
use pinentry::{ConfirmationDialog, PassphraseInput};

use crate::cli::i18n;

/// Seconds a pinentry dialog stays open before it cancels itself, the same
/// limit age's own prompts use.
const PINENTRY_TIMEOUT_SECONDS: u16 = 30;

/// Answers the requests of age plugins through pinentry alone, for processes
/// whose standard streams belong to somebody else, such as the browser host.
/// A plugin asking for a PIN or a confirmation gets a pinentry dialog when
/// `PINENTRY_PROGRAM` or a `pinentry` on `PATH` names one, and a refusal
/// otherwise, so a request can neither wait on a terminal nobody watches
/// nor read from the host's stdin.
#[derive(Clone, Copy)]
pub struct PinentryCallbacks;

/// The pinentry binary to start: `PINENTRY_PROGRAM` when set, otherwise
/// `pinentry` looked up on `PATH`.
fn pinentry_binary() -> OsString {
    std::env::var_os("PINENTRY_PROGRAM").unwrap_or_else(|| OsString::from("pinentry"))
}

impl age::Callbacks for PinentryCallbacks {
    fn display_message(&self, message: &str) {
        eprintln!("{message}");
    }

    fn confirm(&self, message: &str, yes_string: &str, no_string: Option<&str>) -> Option<bool> {
        let mut dialog = ConfirmationDialog::with_binary(pinentry_binary())?;
        dialog
            .with_ok(yes_string)
            .with_timeout(PINENTRY_TIMEOUT_SECONDS);
        if let Some(no_string) = no_string {
            dialog.with_cancel(no_string);
        }
        dialog.confirm(message).ok()
    }

    fn request_public_string(&self, _description: &str) -> Option<String> {
        None
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        let prompt = i18n::prompt_browser_host_pinentry();
        let mut input = PassphraseInput::with_binary(pinentry_binary())?;
        input
            .with_description(description)
            .with_prompt(&prompt)
            .with_timeout(PINENTRY_TIMEOUT_SECONDS);
        input.interact().ok()
    }
}
