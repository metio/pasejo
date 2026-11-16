// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Finding a registered server or organization, signing its requests with
//! this device's key, and keeping its organization logs verified.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use pasejo_client::session::session_valid;
use pasejo_client::transport::{DEFAULT_TIMEOUT, system_clock};
use pasejo_client::{Client, DeviceIdentity};
use pasejo_protocol::Id;
use pasejo_protocol::device::DeviceKind;
use pasejo_protocol::encoding::{Hash, unb64};
use pasejo_protocol::log::OrgLog;
use zeroize::Zeroizing;

use crate::cli::i18n;
use crate::models::configuration::{Configuration, OrgRegistration, ServerRegistration};
use crate::remote::device::DeviceFiles;
use crate::remote::errors::Localize;

/// A server URL as registrations store it.
pub fn normalize_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_owned()
}

/// The server registered under `url`, or the only one when `url` is
/// `None`.
///
/// # Errors
///
/// Fails for unknown URLs, and without a URL when none or several servers
/// are registered.
pub fn server<'a>(
    configuration: &'a Configuration,
    url: Option<&str>,
) -> Result<&'a ServerRegistration> {
    match url {
        Some(url) => {
            let url = normalize_url(url);
            configuration
                .servers
                .iter()
                .find(|server| server.url == url)
                .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_unknown_server(&url)))
        }
        None => match configuration.servers.as_slice() {
            [] => anyhow::bail!(i18n::error_remote_no_server()),
            [server] => Ok(server),
            _ => anyhow::bail!(i18n::error_remote_server_ambiguous()),
        },
    }
}

/// The organization named or identified by `org`, or the only one when
/// `org` is `None`, with the server it belongs to.
///
/// # Errors
///
/// Fails for unknown organizations, and without `org` when none or several
/// are registered.
pub fn org<'a>(
    configuration: &'a Configuration,
    org: Option<&str>,
) -> Result<(&'a ServerRegistration, &'a OrgRegistration)> {
    let all: Vec<(&ServerRegistration, &OrgRegistration)> = configuration
        .servers
        .iter()
        .flat_map(|server| server.orgs.iter().map(move |org| (server, org)))
        .collect();
    match org {
        Some(wanted) => {
            let matching: Vec<_> = all
                .iter()
                .filter(|(_, org)| org.id == wanted || org.name.eq_ignore_ascii_case(wanted))
                .collect();
            match matching.as_slice() {
                [] => anyhow::bail!(i18n::error_remote_unknown_org(wanted)),
                [found] => Ok(**found),
                _ => anyhow::bail!(i18n::error_remote_org_ambiguous(wanted)),
            }
        }
        None => match all.as_slice() {
            [] => anyhow::bail!(i18n::error_remote_no_org()),
            [found] => Ok(*found),
            _ => anyhow::bail!(i18n::error_remote_org_required()),
        },
    }
}

/// Parses an identifier from the configuration.
///
/// # Errors
///
/// Fails for malformed identifiers.
pub fn parse_id(text: &str) -> Result<Id> {
    text.parse::<Id>()
        .map_err(|_| anyhow::anyhow!(i18n::error_remote_invalid_id(text)))
}

/// Parses a base64url hash from the configuration.
///
/// # Errors
///
/// Fails for malformed hashes.
pub fn parse_hash(text: &str) -> Result<Hash> {
    unb64(text)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .map(Hash)
        .ok_or_else(|| anyhow::anyhow!(i18n::error_remote_invalid_id(text)))
}

/// A client for one registered server, signed in.
pub struct Remote {
    pub server: ServerRegistration,
    pub client: Client,
    pub files: DeviceFiles,
}

impl Remote {
    /// A client for `server` without a session.
    ///
    /// # Errors
    ///
    /// Fails when the device's signing key cannot be read.
    pub fn offline(server: &ServerRegistration) -> Result<Self> {
        let id = parse_id(&server.device)?;
        let files = DeviceFiles::new(id)?;
        let device = DeviceIdentity {
            id,
            signing_key: files.signing_key()?,
            kind: DeviceKind::Age,
            recipient: server.recipient.clone(),
            label: server.label.clone(),
        };
        Ok(Self {
            server: server.clone(),
            client: Client::new(&server.url, device, system_clock(), DEFAULT_TIMEOUT),
            files,
        })
    }

    /// A client for `server` with a valid session, renewed through the
    /// registered workload token when the stored one has expired.
    ///
    /// # Errors
    ///
    /// Fails when no valid session exists and none can be renewed.
    pub fn connect(server: &ServerRegistration) -> Result<Self> {
        let mut remote = Self::offline(server)?;
        let now = remote.client.now();
        match remote.files.session()? {
            Some(session) if session_valid(&session, now) => {
                remote.client.set_session(Some(session));
            }
            _ => {
                let Some(path) = &server.workload_token else {
                    anyhow::bail!(i18n::error_remote_not_logged_in(&server.url));
                };
                remote.workload_login(path)?;
            }
        }
        Ok(remote)
    }

    /// Signs in with the workload token in the file at `path` and stores
    /// the session.
    ///
    /// # Errors
    ///
    /// Fails when the file cannot be read or the server refuses the token.
    pub fn workload_login(&mut self, path: &Path) -> Result<()> {
        let display = path.display().to_string();
        let token = Zeroizing::new(
            fs::read_to_string(path)
                .with_context(|| i18n::error_remote_workload_token_unreadable(&display))?,
        );
        let session = self.client.workload_login(token.trim()).localized()?;
        self.files.save_session(&session)
    }

    /// The verified membership log of `org`, extended from the local cache
    /// and stored back.
    ///
    /// # Errors
    ///
    /// Fails when the server refuses, or the log does not verify against
    /// the pinned genesis.
    pub fn log(&self, org: &OrgRegistration) -> Result<OrgLog> {
        let id = parse_id(&org.id)?;
        let pinned = parse_hash(&org.genesis)?;
        let known = self
            .files
            .cached_log(id)
            .ok()
            .flatten()
            .and_then(|entries| OrgLog::replay(entries).ok())
            .filter(|log| log.genesis_hash() == pinned);
        let log = self.client.fetch_log(id, known, Some(pinned)).localized()?;
        self.files.save_log(id, &log)?;
        Ok(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registration(url: &str, orgs: &[(&str, &str)]) -> ServerRegistration {
        ServerRegistration {
            url: url.into(),
            orgs: orgs
                .iter()
                .map(|(id, name)| OrgRegistration {
                    id: (*id).into(),
                    name: (*name).into(),
                    genesis: String::new(),
                })
                .collect(),
            ..ServerRegistration::default()
        }
    }

    fn configuration(servers: Vec<ServerRegistration>) -> Configuration {
        Configuration {
            servers,
            ..Configuration::default()
        }
    }

    #[test]
    fn urls_lose_surrounding_space_and_trailing_slashes() {
        assert_eq!(
            normalize_url(" https://pasejo.example/ "),
            "https://pasejo.example"
        );
        assert_eq!(
            normalize_url("https://pasejo.example//"),
            "https://pasejo.example"
        );
        assert_eq!(
            normalize_url("https://pasejo.example/sub"),
            "https://pasejo.example/sub"
        );
    }

    #[test]
    fn the_only_server_is_the_default_and_urls_select_among_several() {
        i18n::init_for_tests();
        let none = configuration(Vec::new());
        assert!(
            server(&none, None)
                .unwrap_err()
                .to_string()
                .contains("No server is registered")
        );

        let one = configuration(vec![registration("https://a.example", &[])]);
        assert_eq!(server(&one, None).unwrap().url, "https://a.example");
        assert_eq!(
            server(&one, Some("https://a.example/")).unwrap().url,
            "https://a.example"
        );

        let two = configuration(vec![
            registration("https://a.example", &[]),
            registration("https://b.example", &[]),
        ]);
        assert!(
            server(&two, None)
                .unwrap_err()
                .to_string()
                .contains("Several servers")
        );
        assert_eq!(
            server(&two, Some("https://b.example")).unwrap().url,
            "https://b.example"
        );
        let unknown = server(&two, Some("https://c.example"))
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("https://c.example"), "{unknown}");
    }

    #[test]
    fn organizations_are_found_by_identifier_or_name() {
        i18n::init_for_tests();
        let a = "0123456789abcdef0123456789abcdef";
        let b = "fedcba9876543210fedcba9876543210";
        let c = "00112233445566778899aabbccddeeff";
        let config = configuration(vec![
            registration("https://a.example", &[(a, "Acme")]),
            registration("https://b.example", &[(b, "acme"), (c, "Other")]),
        ]);
        let (found_server, found) = org(&config, Some(c)).unwrap();
        assert_eq!(
            (found_server.url.as_str(), found.name.as_str()),
            ("https://b.example", "Other")
        );
        assert_eq!(org(&config, Some("OTHER")).unwrap().1.id, c);
        assert_eq!(org(&config, Some(a)).unwrap().0.url, "https://a.example");
        assert!(
            org(&config, Some("acme"))
                .unwrap_err()
                .to_string()
                .contains("Several organizations")
        );
        assert!(
            org(&config, Some("nope"))
                .unwrap_err()
                .to_string()
                .contains("'nope'")
        );
        assert!(
            org(&config, None)
                .unwrap_err()
                .to_string()
                .contains("--org")
        );

        let single = configuration(vec![registration("https://a.example", &[(a, "Acme")])]);
        assert_eq!(org(&single, None).unwrap().1.id, a);
        let empty = configuration(vec![registration("https://a.example", &[])]);
        assert!(
            org(&empty, None)
                .unwrap_err()
                .to_string()
                .contains("pasejo org create")
        );
    }

    #[test]
    fn identifiers_and_hashes_from_the_configuration_are_checked() {
        i18n::init_for_tests();
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(parse_id(id).unwrap().to_string(), id);
        for bad in [
            "",
            "0123",
            "0123456789ABCDEF0123456789ABCDEFXX",
            "not hex at all, really not hex!",
        ] {
            assert!(
                parse_id(bad)
                    .unwrap_err()
                    .to_string()
                    .contains("not a valid identifier"),
                "{bad}"
            );
        }
        let hash = pasejo_protocol::encoding::b64(&[5; 32]);
        assert_eq!(parse_hash(&hash).unwrap().0, [5; 32]);
        assert!(parse_hash(&pasejo_protocol::encoding::b64(&[5; 31])).is_err());
        assert!(parse_hash("%%%").is_err());
    }
}
