// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Autofill matching over `pasejo-match`, with the two phases of the browser
//! host: a query over the lookup object returns metadata only, and a fill
//! releases one credential only when its secret still matches the request.

use std::collections::BTreeMap;

use pasejo_match::matcher::{self, Credential, Match};
use pasejo_match::parser::fields;
use pasejo_protocol::store::StoreSettings;

use crate::links::normalize_fingerprint;

/// The field holding the SHA-256 fingerprint of the certificate an app
/// named by an `androidapp://` URL must be signed with.
pub const ANDROID_CERT_FIELD: &str = "android-cert";
const ANDROID_APP_SCHEME: &str = "androidapp://";

/// The app that asks for credentials, as the platform reported it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AppRequest {
    pub package: String,
    /// SHA-256 fingerprints of every certificate the app was signed with.
    pub certificates: Vec<String>,
}

/// What a fill asks for. `domains` must already be trusted: the web domain a
/// listed browser reports, or domains whose statement file names the app.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FillRequest {
    pub domains: Vec<String>,
    pub app: Option<AppRequest>,
}

/// A match without secret material.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct FillMatch {
    pub store: String,
    pub store_label: String,
    pub name: String,
    pub username: String,
    pub url: Option<String>,
    /// Whether a URL field names the requested host itself.
    pub exact: bool,
    /// Whether the secret asks to be confirmed before it is filled.
    pub confirm: bool,
    pub score: f64,
}

/// The matches of a query, and stored domains that resemble a requested one
/// when nothing matched.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct QueryResult {
    pub matches: Vec<FillMatch>,
    pub lookalikes: Vec<String>,
    /// The request with the domains verified during the query added, which
    /// a fill of one of the matches passes on.
    pub request: FillRequest,
}

/// Checks whether a domain lets the requesting app receive its
/// credentials, as its statement file says; implemented by the app over a
/// cache of [`crate::verify_app_domain`].
#[uniffi::export(with_foreign)]
pub trait DomainVerifier: Send + Sync {
    fn verified(&self, domain: String) -> bool;
}

/// How many candidate domains a query verifies at most, so a store cannot
/// make one fill fetch many statement files.
const MAX_CANDIDATES: usize = 8;

/// The hosts of the other URL fields of every secret that names `app`
/// through an `androidapp://` URL: domains whose statement file may name
/// the app, so the app can match their entries without a pinned
/// certificate.
pub fn app_candidates(
    texts: &BTreeMap<String, String>,
    settings: &StoreSettings,
    app: &AppRequest,
) -> Vec<String> {
    let url = format!("{ANDROID_APP_SCHEME}{}", app.package);
    let mut found = Vec::new();
    for text in texts.values() {
        let secret = format!("*\n{text}");
        let urls: Vec<&str> = fields(&secret)
            .filter(|(key, _)| settings.uri_keys.iter().any(|uri| uri == key))
            .map(|(_, value)| value.trim())
            .collect();
        if !urls.contains(&url.as_str()) {
            continue;
        }
        found.extend(
            urls.iter()
                .filter(|value| !value.starts_with(ANDROID_APP_SCHEME))
                .filter_map(|value| matcher::extract_host(value)),
        );
    }
    found.sort();
    found.dedup();
    found
}

/// `request` with the candidate domains `verifier` accepts added, at most
/// [`MAX_CANDIDATES`] of them checked.
pub fn with_candidates(
    mut request: FillRequest,
    candidates: Vec<String>,
    verifier: Option<&dyn DomainVerifier>,
) -> FillRequest {
    let Some(verifier) = verifier else {
        return request;
    };
    if request.app.is_none() {
        return request;
    }
    let fresh: Vec<String> = candidates
        .into_iter()
        .filter(|domain| !request.domains.contains(domain))
        .take(MAX_CANDIDATES)
        .collect();
    for domain in fresh {
        if verifier.verified(domain.clone()) {
            request.domains.push(domain);
        }
    }
    request
}

/// The credential a fill releases.
#[derive(Clone, PartialEq, Eq, uniffi::Record)]
pub struct FillCredential {
    pub username: String,
    pub password: String,
    pub totp: Option<String>,
}

impl std::fmt::Debug for FillCredential {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FillCredential")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// The registrable domain spelled by an app's package name, read
/// backwards: `com.example.mail` gives `example.com`.
// UniFFI passes arguments from the foreign side as owned values.
#[allow(clippy::needless_pass_by_value)]
#[uniffi::export]
#[must_use]
pub fn package_domains(package: String) -> Vec<String> {
    let host: Vec<&str> = package.split('.').rev().collect();
    if host.len() < 2 || host.iter().any(|label| label.is_empty()) {
        return vec![];
    }
    matcher::registrable_domain(&host.join(".").to_ascii_lowercase())
        .filter(|domain| domain.contains('.'))
        .into_iter()
        .collect()
}

/// Whether `secret` names `app` through an `androidapp://` URL field and
/// carries the fingerprint of one of its certificates.
fn names_app(secret: &str, app: &AppRequest, settings: &StoreSettings) -> bool {
    let url = format!("{ANDROID_APP_SCHEME}{}", app.package);
    let mut named = false;
    let mut signed = false;
    for (key, value) in fields(secret) {
        let value = value.trim();
        if settings.uri_keys.iter().any(|uri| uri == key) && value == url {
            named = true;
        }
        if key == ANDROID_CERT_FIELD {
            let pinned = normalize_fingerprint(value);
            signed |= pinned.len() == 64
                && app
                    .certificates
                    .iter()
                    .any(|certificate| normalize_fingerprint(certificate) == pinned);
        }
    }
    named && signed
}

/// The matches of `request` in one store, given each secret's lookup text
/// (or full text; both parse the same way below a password line).
pub fn matches(
    texts: &BTreeMap<String, String>,
    store: &str,
    label: &str,
    settings: &StoreSettings,
    request: &FillRequest,
) -> Vec<FillMatch> {
    let secrets: BTreeMap<String, String> = texts
        .iter()
        .map(|(name, text)| (name.clone(), format!("*\n{text}")))
        .collect();
    let mut found: BTreeMap<String, Match> = BTreeMap::new();
    let mut keep = |candidate: Match| {
        let better = found
            .get(&candidate.name)
            .is_none_or(|current| candidate.score > current.score);
        if better {
            found.insert(candidate.name.clone(), candidate);
        }
    };
    for domain in &request.domains {
        for candidate in matcher::find_matches(
            &secrets,
            label,
            domain,
            &settings.uri_keys,
            &settings.username_keys,
        ) {
            keep(candidate);
        }
    }
    if let Some(app) = &request.app {
        for candidate in matcher::find_matches(
            &secrets,
            label,
            &app.package,
            &settings.uri_keys,
            &settings.username_keys,
        ) {
            if names_app(&secrets[&candidate.name], app, settings) {
                keep(candidate);
            }
        }
    }
    let mut matches: Vec<FillMatch> = found
        .into_values()
        .map(|candidate| FillMatch {
            store: store.to_owned(),
            store_label: label.to_owned(),
            name: candidate.name,
            username: candidate.username,
            url: candidate.url,
            exact: candidate.exact_host,
            confirm: candidate.confirm,
            score: candidate.score,
        })
        .collect();
    matches.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
    });
    matches
}

/// Stored domains resembling one of the requested domains.
pub fn lookalikes(
    texts: &BTreeMap<String, String>,
    settings: &StoreSettings,
    request: &FillRequest,
) -> Vec<String> {
    let secrets: BTreeMap<String, String> = texts
        .iter()
        .map(|(name, text)| (name.clone(), format!("*\n{text}")))
        .collect();
    let mut found: Vec<String> = request
        .domains
        .iter()
        .flat_map(|domain| {
            matcher::find_lookalikes(
                &secrets,
                domain,
                &settings.uri_keys,
                &settings.username_keys,
            )
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The credential of the fetched `secret` named `name`, released only when
/// it matches `request`. `totp` runs only for a match whose URL field names
/// the requested host itself.
pub fn release(
    name: &str,
    secret: &str,
    label: &str,
    settings: &StoreSettings,
    request: &FillRequest,
    totp: impl FnOnce() -> Option<String>,
) -> Option<FillCredential> {
    let secrets = BTreeMap::from([(name.to_owned(), secret.to_owned())]);
    let mut totp = Some(totp);
    let mut reveal = |domain: &str| -> Option<Credential> {
        let mut exact = false;
        let mut credential = matcher::reveal(
            &secrets,
            label,
            domain,
            name,
            &settings.uri_keys,
            &settings.username_keys,
            || {
                exact = true;
                None
            },
        )?;
        if exact {
            credential.otp = totp.take().and_then(|totp| totp());
        }
        Some(credential)
    };
    let credential = request
        .domains
        .iter()
        .find_map(|domain| reveal(domain))
        .or_else(|| {
            let app = request.app.as_ref()?;
            if names_app(secret, app, settings) {
                reveal(&app.package)
            } else {
                None
            }
        })?;
    Some(FillCredential {
        username: credential.username.clone(),
        password: credential.password.clone(),
        totp: credential.otp.clone(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::links::tests::CERT;

    fn texts(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect()
    }

    fn web(domain: &str) -> FillRequest {
        FillRequest {
            domains: vec![domain.into()],
            app: None,
        }
    }

    fn app(package: &str, cert: &str) -> FillRequest {
        FillRequest {
            domains: vec![],
            app: Some(AppRequest {
                package: package.into(),
                certificates: vec![cert.into()],
            }),
        }
    }

    fn names(found: &[FillMatch]) -> Vec<&str> {
        found.iter().map(|m| m.name.as_str()).collect()
    }

    #[test]
    fn package_names_spell_their_registrable_domain() {
        assert_eq!(package_domains("com.example.mail".into()), ["example.com"]);
        assert_eq!(
            package_domains("uk.co.example.app".into()),
            ["example.co.uk"]
        );
        assert_eq!(package_domains("com".into()), Vec::<String>::new());
        assert_eq!(package_domains("com..mail".into()), Vec::<String>::new());
    }

    #[test]
    fn a_web_query_returns_metadata_and_ranks_exact_hosts_first() {
        let lookup = texts(&[
            ("a", "url: https://login.example.com\nuser: alice"),
            ("b", "url: https://example.com\nuser: bob"),
            ("c", "url: https://other.org"),
        ]);
        let settings = StoreSettings::default();
        let found = matches(&lookup, "s", "Work", &settings, &web("example.com"));
        assert_eq!(names(&found), ["b", "a"]);
        assert!(found[0].exact && !found[1].exact);
        assert_eq!(found[0].username, "bob");
        assert_eq!(found[0].store_label, "Work");
    }

    #[test]
    fn several_domains_keep_the_best_match_per_secret() {
        let lookup = texts(&[("a", "url: https://mail.example.com")]);
        let request = FillRequest {
            domains: vec!["example.com".into(), "mail.example.com".into()],
            app: None,
        };
        let found = matches(&lookup, "s", "l", &StoreSettings::default(), &request);
        assert_eq!(found.len(), 1);
        assert!(found[0].exact);
    }

    #[test]
    fn an_app_matches_only_with_its_pinned_certificate() {
        let lookup = texts(&[
            (
                "pinned",
                &format!("url: androidapp://com.example.mail\nandroid-cert: {CERT}"),
            ),
            ("unpinned", "url: androidapp://com.example.mail"),
            (
                "other",
                &format!("url: androidapp://com.example.other\nandroid-cert: {CERT}"),
            ),
        ]);
        let settings = StoreSettings::default();
        let found = matches(&lookup, "s", "l", &settings, &app("com.example.mail", CERT));
        assert_eq!(names(&found), ["pinned"]);
        let forged = matches(
            &lookup,
            "s",
            "l",
            &settings,
            &app("com.example.mail", &"AB".repeat(32)),
        );
        assert!(forged.is_empty());
    }

    #[test]
    fn an_app_request_does_not_match_web_urls_or_names() {
        let lookup = texts(&[
            ("com.example.mail", ""),
            (
                "web",
                &format!("url: https://com.example.mail\nandroid-cert: {CERT}"),
            ),
        ]);
        let found = matches(
            &lookup,
            "s",
            "l",
            &StoreSettings::default(),
            &app("com.example.mail", CERT),
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn never_fill_secrets_are_hidden() {
        let lookup = texts(&[("a", "url: https://example.com\npasejo-fill: never")]);
        assert!(
            matches(
                &lookup,
                "s",
                "l",
                &StoreSettings::default(),
                &web("example.com")
            )
            .is_empty()
        );
    }

    #[test]
    fn lookalikes_are_named_when_a_domain_resembles_a_stored_one() {
        let lookup = texts(&[("a", "url: https://example.com")]);
        assert_eq!(
            lookalikes(&lookup, &StoreSettings::default(), &web("examp1e.com")),
            ["example.com"]
        );
        assert!(lookalikes(&lookup, &StoreSettings::default(), &web("example.com")).is_empty());
    }

    #[test]
    fn a_fill_releases_only_a_matching_secret() {
        let settings = StoreSettings::default();
        let secret = "hunter2\nurl: https://example.com\nuser: alice";
        let credential = release("a", secret, "l", &settings, &web("example.com"), || {
            Some("123456".into())
        })
        .unwrap();
        assert_eq!(
            (
                credential.username.as_str(),
                credential.password.as_str(),
                credential.totp.as_deref()
            ),
            ("alice", "hunter2", Some("123456"))
        );
        assert!(release("a", secret, "l", &settings, &web("attacker.com"), || None).is_none());
    }

    #[test]
    fn a_registrable_domain_match_carries_no_totp() {
        let settings = StoreSettings::default();
        let secret = "hunter2\nurl: https://login.example.com";
        let mut asked = false;
        let credential = release("a", secret, "l", &settings, &web("example.com"), || {
            asked = true;
            Some("123456".into())
        })
        .unwrap();
        assert_eq!(credential.totp, None);
        assert!(!asked);
    }

    #[test]
    fn an_app_fill_needs_the_url_and_the_certificate() {
        let settings = StoreSettings::default();
        let secret =
            format!("pw\nurl: androidapp://com.example.mail\nandroid-cert: {CERT}\nuser: alice");
        let released = release(
            "a",
            &secret,
            "l",
            &settings,
            &app("com.example.mail", CERT),
            || None,
        )
        .unwrap();
        assert_eq!(released.password, "pw");
        assert!(
            release(
                "a",
                &secret,
                "l",
                &settings,
                &app("com.example.mail", &"AB".repeat(32)),
                || None
            )
            .is_none()
        );
        assert!(
            release(
                "a",
                &secret,
                "l",
                &settings,
                &app("com.attacker.mail", CERT),
                || None
            )
            .is_none()
        );
    }

    struct Accepting(Vec<&'static str>, std::sync::Mutex<Vec<String>>);

    impl DomainVerifier for Accepting {
        fn verified(&self, domain: String) -> bool {
            let accepted = self.0.contains(&domain.as_str());
            self.1.lock().unwrap().push(domain);
            accepted
        }
    }

    #[test]
    fn secrets_naming_an_app_offer_their_other_hosts_as_candidates() {
        let lookup = texts(&[
            (
                "mail",
                "url: androidapp://com.example.mail\nurl: https://login.mail.example.org/sign-in",
            ),
            (
                "other",
                "url: androidapp://com.other\nurl: https://other.example",
            ),
            ("web", "url: https://unrelated.example"),
        ]);
        let request = app("com.example.mail", CERT);
        assert_eq!(
            app_candidates(
                &lookup,
                &StoreSettings::default(),
                request.app.as_ref().unwrap()
            ),
            ["login.mail.example.org"]
        );
    }

    #[test]
    fn only_verified_candidates_join_the_request() {
        let verifier = Accepting(vec!["good.example"], std::sync::Mutex::default());
        let request = with_candidates(
            app("com.example.mail", CERT),
            vec!["good.example".into(), "bad.example".into()],
            Some(&verifier),
        );
        assert_eq!(request.domains, ["good.example"]);
        assert_eq!(*verifier.1.lock().unwrap(), ["good.example", "bad.example"]);
        let web_request = with_candidates(
            web("example.com"),
            vec!["good.example".into()],
            Some(&verifier),
        );
        assert_eq!(web_request.domains, ["example.com"]);
        assert_eq!(
            with_candidates(
                app("com.example.mail", CERT),
                vec!["good.example".into()],
                None
            )
            .domains,
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_query_verifies_a_bounded_number_of_candidates() {
        let verifier = Accepting(vec![], std::sync::Mutex::default());
        let candidates = (0..20).map(|i| format!("d{i}.example")).collect();
        with_candidates(app("com.example.mail", CERT), candidates, Some(&verifier));
        assert_eq!(verifier.1.lock().unwrap().len(), MAX_CANDIDATES);
    }

    #[test]
    fn a_verified_candidate_matches_the_secret_naming_the_app() {
        let lookup = texts(&[(
            "mail",
            "user: alice\nurl: androidapp://com.example.mail\nurl: https://login.mail.example.org",
        )]);
        let settings = StoreSettings::default();
        let request = app("com.example.mail", "00".repeat(32).as_str());
        assert!(matches(&lookup, "s", "Work", &settings, &request).is_empty());
        let verifier = Accepting(vec!["login.mail.example.org"], std::sync::Mutex::default());
        let candidates = app_candidates(&lookup, &settings, request.app.as_ref().unwrap());
        let request = with_candidates(request, candidates, Some(&verifier));
        assert_eq!(
            names(&matches(&lookup, "s", "Work", &settings, &request)),
            ["mail"]
        );
    }

    #[test]
    fn the_debug_form_of_a_credential_hides_the_password() {
        let credential = FillCredential {
            username: "alice".into(),
            password: "hunter2".into(),
            totp: Some("123456".into()),
        };
        let shown = format!("{credential:?}");
        assert!(
            !shown.contains("hunter2") && !shown.contains("123456"),
            "{shown}"
        );
    }
}
