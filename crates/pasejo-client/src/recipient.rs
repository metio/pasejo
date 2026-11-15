// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Turning a device's recipient string into an age recipient.

use crate::error::{Error, Result};

/// Answers no plugin prompt: wrapping keys never needs one, and a client
/// running unattended must not wait for input.
#[derive(Clone, Copy)]
struct Silent;

impl age::Callbacks for Silent {
    fn display_message(&self, _message: &str) {}

    fn confirm(&self, _message: &str, _yes: &str, _no: Option<&str>) -> Option<bool> {
        None
    }

    fn request_public_string(&self, _description: &str) -> Option<String> {
        None
    }

    fn request_passphrase(&self, _description: &str) -> Option<age::secrecy::SecretString> {
        None
    }
}

/// Parses an X25519, SSH or plugin recipient.
///
/// # Errors
///
/// Fails for anything else, or when a plugin recipient's plugin is missing.
pub fn parse(text: &str) -> Result<Box<dyn age::Recipient>> {
    if let Ok(recipient) = text.parse::<age::x25519::Recipient>() {
        return Ok(Box::new(recipient));
    }
    if let Ok(recipient) = text.parse::<age::ssh::Recipient>() {
        return Ok(Box::new(recipient));
    }
    if let Ok(recipient) = text.parse::<age::plugin::Recipient>() {
        let plugin = age::plugin::RecipientPluginV1::new(
            recipient.plugin(),
            std::slice::from_ref(&recipient),
            &[],
            Silent,
        )
        .map_err(|error| Error::Unavailable(format!("age plugin: {error}")))?;
        return Ok(Box::new(plugin));
    }
    Err(Error::Unavailable(format!("unsupported recipient {text}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x25519_and_ssh_recipients_parse() {
        let identity = age::x25519::Identity::generate();
        assert!(parse(&identity.to_public().to_string()).is_ok());
        assert!(
            parse(
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHsKLqeplhpW+uObz5dvMgjz1OxfM/XXUB+VHtZ6isGN"
            )
            .is_ok()
        );
        assert!(parse("nonsense").is_err());
        assert!(parse("age1nonexistentplugin1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqq").is_err());
    }
}
