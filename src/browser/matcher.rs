// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use serde::Serialize;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::browser::lookalike::lookalikes;
use crate::browser::policy::{FillPolicy, Policy};
use crate::exporters::parser::{ParsedSecret, parse_secret};
use crate::models::password_store::{OneTimePasswordType, PasswordStore};

const SCORE_URL_EXACT_HOST: f64 = 1.0;
const SCORE_URL_REGISTRABLE_DOMAIN: f64 = 0.9;
const SCORE_NAME: f64 = 0.5;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    UrlField,
    Name,
}

/// What a query discloses about a matching secret: enough to list and pick
/// it, never its password, other field values or one-time code.
#[derive(Debug, Serialize, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Match {
    pub name: String,
    pub store: String,
    pub username: String,
    pub url: Option<String>,
    pub match_kind: MatchKind,
    pub score: f64,
    /// The secret's `pasejo-confirm` policy asks the extension to confirm
    /// with the user before it fills.
    pub confirm: bool,
    /// A URL field names the queried host itself. Only such a match
    /// releases a one-time code, so a sibling subdomain or a secret that
    /// merely carries the domain in its name cannot obtain one.
    #[serde(skip)]
    pub exact_host: bool,
}

/// The secret material `reveal` releases for one matching secret.
#[derive(Debug, Serialize, Zeroize, ZeroizeOnDrop)]
pub struct Credential {
    pub name: String,
    pub store: String,
    pub username: String,
    pub password: String,
    /// Current TOTP code, present only for an exact-host match whose
    /// secret has a TOTP entry under the same name. HOTP entries are
    /// excluded: emitting an HOTP code without bumping the counter breaks
    /// RFC 4226's single-use contract, and bumping it would turn the
    /// read-only browser path into a writer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otp: Option<String>,
}

/// The canonical query host and its registrable domain. `parse` yields
/// `None` when the query has no host to compare against.
struct Query {
    domain: String,
    registrable: Option<String>,
}

impl Query {
    fn parse(domain: &str) -> Option<Self> {
        let domain = canonical_host(domain);
        if domain.is_empty() {
            return None;
        }
        let registrable = registrable_domain(&domain);
        Some(Self {
            domain,
            registrable,
        })
    }
}

pub fn find_matches(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<Match> {
    let Some(query) = Query::parse(domain) else {
        return vec![];
    };
    let mut matches: Vec<Match> = store
        .secrets
        .iter()
        .filter_map(|(name, value)| {
            let parsed = parse_secret(value, username_keys, uri_keys);
            let policy = Policy::from_secret(value);
            match_secret(
                name,
                &parsed,
                policy,
                store_name,
                &query,
                uri_keys,
                username_keys,
            )
        })
        .collect();
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches
}

/// The registrable domains of the store's URL fields that could be mistaken
/// for `domain`; see `lookalike::lookalikes`. Secret names are not consulted,
/// and a secret with `pasejo-fill: never` contributes nothing, so the browser
/// learns no more about it than a query would.
pub fn find_lookalikes(
    store: &PasswordStore,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<String> {
    let Some(query) = Query::parse(domain) else {
        return vec![];
    };
    let stored_hosts = store
        .secrets
        .values()
        .filter(|value| Policy::from_secret(value).fill != FillPolicy::Never)
        .flat_map(|value| {
            parse_secret(value, username_keys, uri_keys)
                .fields
                .into_iter()
                .filter(|(key, _)| uri_keys.contains(key))
                .filter_map(|(_, field_value)| extract_host(&field_value))
        });
    lookalikes(&query.domain, stored_hosts)
}

/// Releases the credential of `secret_name` only when that secret is one
/// `find_matches` returns for `domain`. A missing secret and a secret that
/// does not match both yield `None`, so the answer reveals nothing about
/// which names exist.
pub fn reveal(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    secret_name: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Option<Credential> {
    let query = Query::parse(domain)?;
    let value = store.secrets.get(secret_name)?;
    let parsed = parse_secret(value, username_keys, uri_keys);
    let matched = match_secret(
        secret_name,
        &parsed,
        Policy::from_secret(value),
        store_name,
        &query,
        uri_keys,
        username_keys,
    )?;
    let otp = if matched.exact_host {
        totp_code_for(store, secret_name)
    } else {
        None
    };
    Some(Credential {
        name: matched.name,
        store: matched.store,
        username: matched.username,
        password: parsed.password.unwrap_or_default(),
        otp,
    })
}

/// Applies the secret's fill policy on top of the domain match: `never`
/// matches nothing, and `exact-host` keeps only a URL field naming the
/// queried host itself.
fn match_secret(
    name: &str,
    parsed: &ParsedSecret,
    policy: Policy,
    store_name: &str,
    query: &Query,
    uri_keys: &[String],
    username_keys: &[String],
) -> Option<Match> {
    if policy.fill == FillPolicy::Never {
        return None;
    }
    let mut username = String::new();
    let mut best_url: Option<(f64, String)> = None;

    for (key, field_value) in &parsed.fields {
        if username_keys.contains(key) {
            username.clone_from(field_value);
            continue;
        }
        if uri_keys.contains(key)
            && let Some((score, url)) =
                score_url_field(field_value, &query.domain, query.registrable.as_deref())
            && best_url
                .as_ref()
                .is_none_or(|(current_score, _)| score > *current_score)
        {
            best_url = Some((score, url));
        }
    }

    if let Some((score, url)) = best_url {
        let exact_host = score >= SCORE_URL_EXACT_HOST;
        if policy.fill == FillPolicy::ExactHost && !exact_host {
            return None;
        }
        return Some(Match {
            name: name.to_owned(),
            store: store_name.to_owned(),
            username,
            url: Some(url),
            match_kind: MatchKind::UrlField,
            exact_host,
            score,
            confirm: policy.confirm,
        });
    }

    if policy.fill == FillPolicy::AnyMatch
        && name_matches_domain(name, &query.domain, query.registrable.as_deref())
    {
        return Some(Match {
            name: name.to_owned(),
            store: store_name.to_owned(),
            username,
            url: None,
            match_kind: MatchKind::Name,
            score: SCORE_NAME,
            exact_host: false,
            confirm: policy.confirm,
        });
    }

    None
}

/// Look up a TOTP entry stored under the same name as the matched
/// secret and render its current code. Returns `None` for non-existent
/// entries, for HOTP entries (see the doc comment on `Credential::otp`), and
/// when code generation itself fails (a broken `otpauth://` secret
/// shouldn't crash the whole query).
fn totp_code_for(store: &PasswordStore, secret_name: &str) -> Option<String> {
    let entry = store.otp.get(secret_name)?;
    if entry.otp_type != OneTimePasswordType::Totp {
        return None;
    }
    let mut clone = entry.clone();
    let code = clone.generate().ok()?;
    Some(format!("{code:0width$}", width = entry.digits as usize))
}

fn score_url_field(
    field_value: &str,
    query_domain: &str,
    query_registrable: Option<&str>,
) -> Option<(f64, String)> {
    let host = extract_host(field_value)?;
    if host == query_domain {
        return Some((SCORE_URL_EXACT_HOST, field_value.to_owned()));
    }
    if let (Some(qr), Some(hr)) = (query_registrable, registrable_domain(&host))
        && qr == hr
    {
        return Some((SCORE_URL_REGISTRABLE_DOMAIN, field_value.to_owned()));
    }
    None
}

/// Subdomain segments only count when the query has a registrable domain;
/// without one (an IP, a public suffix, a single-label host) the suffix
/// rule would cross site boundaries, so only the exact host matches.
fn name_matches_domain(name: &str, query_domain: &str, query_registrable: Option<&str>) -> bool {
    let dotted = query_registrable.map(|registrable| format!(".{registrable}"));
    let needle = query_registrable.unwrap_or(query_domain);
    name.split(['/', '\\']).any(|segment| {
        let lower = if segment.is_ascii() {
            segment.to_lowercase()
        } else {
            canonical_host(segment)
        };
        lower == needle || dotted.as_deref().is_some_and(|d| lower.ends_with(d))
    })
}

/// Reports whether `input` starts with a URL scheme per the RFC 3986
/// grammar (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) "://"`). A
/// plain `contains("://")` is not enough: a scheme-less stored value
/// whose query string embeds another URL
/// (`example.com/redirect?to=https://evil.com`) contains the marker
/// without *starting* with a scheme, and feeding it to `Url::parse`
/// directly fails — the value would silently stop matching.
fn has_url_scheme(input: &str) -> bool {
    let Some((scheme, _)) = input.split_once("://") else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

fn extract_host(input: &str) -> Option<String> {
    // A leading space defeats `has_url_scheme`, so the value is trimmed
    // here as well as in `parse_secret`.
    let input = input.trim();
    // The `url` crate's parser is the trust boundary for URL shape:
    // it handles percent-encoded hostnames, IDN punycode, userinfo,
    // port, IPv6 brackets, and fragment-only inputs without us
    // re-implementing the spec. `Url::parse` requires an explicit
    // scheme, so for scheme-less stored values (`example.com`,
    // `example.com/login`) prepend a placeholder before parsing —
    // the scheme doesn't affect the extracted host, only the
    // parser's ability to find an authority.
    let parsed = if has_url_scheme(input) {
        url::Url::parse(input).ok()?
    } else {
        url::Url::parse(&format!("http://{input}")).ok()?
    };
    // `host()` (not `host_str()`) so IPv6 addresses come back
    // unbracketed, the same form `canonical_host` gives the query.
    let host = host_string(&parsed.host()?);
    if host.is_empty() { None } else { Some(host) }
}

/// The form both sides of every comparison use: IDNA-encoded and
/// lowercased by the `url` host parser, IPv6 unbracketed, and without
/// the DNS root dot. Input the parser rejects is only lowercased.
fn canonical_host(input: &str) -> String {
    let input = input.trim();
    url::Host::parse(input).map_or_else(
        |_| strip_root_dot(&input.to_lowercase()),
        |host| host_string(&host),
    )
}

fn host_string<S: AsRef<str>>(host: &url::Host<S>) -> String {
    let rendered = match host {
        url::Host::Ipv6(addr) => addr.to_string(),
        other => other.to_string(),
    };
    strip_root_dot(&rendered.to_lowercase())
}

fn strip_root_dot(host: &str) -> String {
    host.strip_suffix('.').unwrap_or(host).to_owned()
}

/// IP literals have no registrable domain: the public-suffix list's
/// default rule would treat their last octets as one, so two unrelated
/// addresses ending in the same octets would compare equal.
fn registrable_domain(host: &str) -> Option<String> {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    psl::domain_str(host).map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::password_store::{
        OneTimePassword, OneTimePasswordAlgorithm, OneTimePasswordType,
    };

    fn store_with(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (name, value) in secrets {
            store
                .secrets
                .insert((*name).to_owned(), (*value).to_owned());
        }
        store
    }

    fn totp_entry() -> OneTimePassword {
        OneTimePassword {
            secret: "JBSWY3DPEHPK3PXP".into(),
            otp_type: OneTimePasswordType::Totp,
            algorithm: OneTimePasswordAlgorithm::Sha1,
            digits: 6,
            period: 30,
            counter: 0,
            skew: 0,
        }
    }

    fn hotp_entry() -> OneTimePassword {
        let mut entry = totp_entry();
        entry.otp_type = OneTimePasswordType::Hotp;
        entry
    }

    fn uri_keys() -> Vec<String> {
        vec!["uri".into(), "url".into(), "link".into(), "site".into()]
    }

    fn username_keys() -> Vec<String> {
        vec!["login".into(), "email".into(), "username".into()]
    }

    #[test]
    fn empty_domain_returns_no_matches() {
        let store = store_with(&[("example.com/alice", "hunter2\nurl: https://example.com")]);
        let result = find_matches(&store, "personal", "", &uri_keys(), &username_keys());
        assert!(result.is_empty());
    }

    #[test]
    fn empty_store_returns_no_matches() {
        let store = PasswordStore::default();
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn url_field_exact_host_match_scores_one() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com/login")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
        assert_eq!(result[0].url.as_deref(), Some("https://example.com/login"));
        assert_eq!(result[0].store, "personal");
    }

    #[test]
    fn url_field_subdomain_via_registrable_domain_scores_lower() {
        let store = store_with(&[("alice", "hunter2\nurl: https://www.example.com")]);
        let result = find_matches(
            &store,
            "personal",
            "mail.example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
        assert!((result[0].score - SCORE_URL_REGISTRABLE_DOMAIN).abs() < f64::EPSILON);
    }

    #[test]
    fn different_registrable_domain_does_not_match() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.org")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn co_uk_etld_is_handled_correctly() {
        let store = store_with(&[("alice", "hunter2\nurl: https://attacker.co.uk")]);
        let result = find_matches(
            &store,
            "personal",
            "victim.co.uk",
            &uri_keys(),
            &username_keys(),
        );
        assert!(
            result.is_empty(),
            "matches across a public suffix would be a security bug"
        );
    }

    #[test]
    fn name_fallback_matches_when_no_url_field_present() {
        let store = store_with(&[("sites/example.com/alice", "hunter2")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::Name);
        assert!((result[0].score - SCORE_NAME).abs() < f64::EPSILON);
        assert!(result[0].url.is_none());
    }

    #[test]
    fn name_fallback_matches_subdomain_segments() {
        let store = store_with(&[("sites/mail.example.com/alice", "hunter2")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::Name);
    }

    #[test]
    fn name_substring_outside_path_segment_does_not_match() {
        let store = store_with(&[("example.com_archive", "hunter2")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn url_field_match_wins_over_name_match_on_same_secret() {
        let store = store_with(&[(
            "sites/example.com/alice",
            "hunter2\nurl: https://example.com",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
    }

    #[test]
    fn results_are_sorted_by_score_descending() {
        let store = store_with(&[
            ("sites/example.com/alice", "hunter2"),
            ("bob", "hunter3\nurl: https://www.example.com"),
            ("carol", "hunter4\nurl: https://example.com"),
        ]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 3);
        assert!(result[0].score >= result[1].score);
        assert!(result[1].score >= result[2].score);
        assert_eq!(result[0].name, "carol");
    }

    #[test]
    fn name_fallback_applies_when_url_field_names_another_host() {
        let store = store_with(&[("example.com/sso", "hunter2\nurl: https://login.other.org")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::Name);
        assert!(result[0].url.is_none());
    }

    #[test]
    fn username_field_populates_username() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\nlogin: alice@example.com",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result[0].username, "alice@example.com");
    }

    #[test]
    fn aligned_username_and_url_values_are_trimmed() {
        let store = store_with(&[(
            "alice",
            " hunter2 \nurl:      https://example.com  \nlogin:    alice\t\nnote:   padded  ",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].username, "alice");
        assert_eq!(result[0].url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn revealed_password_is_kept_verbatim() {
        let store = store_with(&[(
            "alice",
            " hunter2 \nurl:      https://example.com  \nlogin:    alice\t",
        )]);
        let credential = reveal(
            &store,
            "personal",
            "example.com",
            "alice",
            &uri_keys(),
            &username_keys(),
        )
        .expect("alice matches example.com");
        assert_eq!(credential.password, " hunter2 ");
        assert_eq!(credential.username, "alice");
    }

    #[test]
    fn query_matches_carry_no_secret_material_on_the_wire() {
        let mut store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\nlogin: alice\nnote: backup codes here",
        )]);
        store.otp.insert("alice".into(), totp_entry());
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        let json = serde_json::to_value(&result[0]).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            [
                "confirm",
                "match_kind",
                "name",
                "score",
                "store",
                "url",
                "username"
            ],
            "{json}"
        );
        let text = json.to_string();
        assert!(!text.contains("hunter2"), "{text}");
        assert!(!text.contains("backup codes"), "{text}");
    }

    #[test]
    fn custom_uri_keys_parameter_is_honored() {
        let store = store_with(&[("alice", "hunter2\nsite: https://example.com")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &["site".into()],
            &username_keys(),
        );
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &["url".into()],
            &username_keys(),
        );
        assert!(result.is_empty());
    }

    #[test]
    fn query_domain_is_case_insensitive() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        let result = find_matches(
            &store,
            "personal",
            "EXAMPLE.COM",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn bare_host_without_scheme_is_matched() {
        let store = store_with(&[("alice", "hunter2\nurl: example.com")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
    }

    #[test]
    fn url_with_port_and_userinfo_extracts_host() {
        let store = store_with(&[("alice", "hunter2\nurl: https://user:pw@example.com:8443/x")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
    }

    #[test]
    fn ipv6_bracketed_url_matches_the_unbracketed_query_domain() {
        // The extension strips the brackets from `URL.hostname` before
        // sending the query domain, so the stored `[::1]` and the
        // queried `::1` must land on the same canonical (unbracketed)
        // form. An assertion that tolerates the empty result would
        // mask a host-extraction regression — this one demands the
        // match.
        let store = store_with(&[("alice", "hunter2\nurl: https://[::1]:8443/x")]);
        let result = find_matches(&store, "personal", "::1", &uri_keys(), &username_keys());
        assert_eq!(result.len(), 1, "IPv6 URL must match the unbracketed query");
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
    }

    #[test]
    fn bracketed_ipv6_query_matches_the_stored_address() {
        let store = store_with(&[("alice", "hunter2\nurl: https://[::1]:8443/x")]);
        let result = find_matches(&store, "personal", "[::1]", &uri_keys(), &username_keys());
        assert_eq!(result.len(), 1, "{result:?}");
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
    }

    #[test]
    fn fully_qualified_query_is_an_exact_host_match() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        let result = find_matches(
            &store,
            "personal",
            "example.com.",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1, "{result:?}");
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
    }

    #[test]
    fn unicode_query_matches_a_stored_unicode_url() {
        let store = store_with(&[("alice", "hunter2\nurl: https://bücher.de")]);
        let result = find_matches(
            &store,
            "personal",
            "Bücher.de",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1, "{result:?}");
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
    }

    #[test]
    fn unicode_name_segment_matches_unicode_and_punycode_queries() {
        let store = store_with(&[("sites/bücher.de/alice", "hunter2")]);
        for query in ["bücher.de", "xn--bcher-kva.de"] {
            let result = find_matches(&store, "personal", query, &uri_keys(), &username_keys());
            assert_eq!(result.len(), 1, "{query}: {result:?}");
            assert_eq!(result[0].match_kind, MatchKind::Name);
        }
    }

    #[test]
    fn query_of_only_a_dot_returns_no_matches() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        let result = find_matches(&store, "personal", ".", &uri_keys(), &username_keys());
        assert!(result.is_empty(), "{result:?}");
    }

    #[test]
    fn extract_host_returns_ipv6_addresses_unbracketed() {
        assert_eq!(extract_host("https://[::1]:8443/x"), Some("::1".to_owned()),);
        assert_eq!(
            extract_host("https://[2001:db8::1]/"),
            Some("2001:db8::1".to_owned()),
        );
    }

    #[test]
    fn extract_host_handles_a_scheme_less_value_with_an_embedded_url() {
        // `contains("://")` is not scheme detection: this value has no
        // scheme of its own but embeds one in the query string. It
        // must resolve to its own host, not fail to parse (and
        // certainly not resolve to the embedded URL's host).
        assert_eq!(
            extract_host("example.com/redirect?to=https://evil.com"),
            Some("example.com".to_owned()),
        );
    }

    #[test]
    fn has_url_scheme_accepts_rfc3986_schemes_and_rejects_lookalikes() {
        assert!(has_url_scheme("https://example.com"));
        assert!(has_url_scheme("ssh://git@example.com"));
        assert!(has_url_scheme("chrome-extension://abc/"));
        // Path/query characters before the marker disqualify it.
        assert!(!has_url_scheme("example.com/redirect?to=https://evil.com"));
        assert!(!has_url_scheme("example.com"));
        assert!(!has_url_scheme("://no-scheme"));
        assert!(!has_url_scheme("1http://digit-first"));
    }

    #[test]
    fn extract_host_handles_idn_punycode_consistently() {
        // The `url` crate normalises IDN to punycode at parse time, so
        // a stored URL written with the unicode form and a query for
        // the punycode form (or vice versa) need to agree on which
        // representation reaches the comparison. Both sides flow
        // through the same parser, so the result is consistent —
        // pinned here so a future revert to hand-rolled splitting
        // can't silently regress.
        assert_eq!(
            extract_host("https://xn--bcher-kva.example/"),
            Some("xn--bcher-kva.example".to_owned()),
        );
    }

    #[test]
    fn extract_host_rejects_empty_authority() {
        // Stored garbage like `https://` shouldn't yield a phantom
        // empty host that compares equal to other empty inputs.
        assert!(extract_host("https://").is_none());
        assert!(extract_host("").is_none());
    }

    #[test]
    fn extract_host_accepts_scheme_less_input_with_path_query_fragment() {
        // Stored values like `example.com/login?ref=x#section` come
        // up when users paste from address bars without the scheme;
        // the parser prepends a placeholder scheme transparently.
        assert_eq!(
            extract_host("example.com/login?ref=x#section"),
            Some("example.com".to_owned()),
        );
    }

    #[test]
    fn extract_host_trims_surrounding_whitespace() {
        // `url: <tab/spaces> https://example.com` keeps the leading
        // whitespace through the parser; trimming keeps it matching.
        assert_eq!(
            extract_host("   https://example.com"),
            Some("example.com".to_owned()),
        );
        assert_eq!(
            extract_host("\texample.com/login  "),
            Some("example.com".to_owned()),
        );
    }

    #[test]
    fn localhost_matches_only_on_exact_host() {
        let store = store_with(&[("alice", "hunter2\nurl: http://localhost:3000")]);
        let result = find_matches(
            &store,
            "personal",
            "localhost",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].match_kind, MatchKind::UrlField);
    }

    #[test]
    fn ipv4_hosts_sharing_trailing_octets_do_not_match() {
        let store = store_with(&[("router", "pw\nurl: http://192.168.0.1")]);
        let result = find_matches(
            &store,
            "personal",
            "198.51.0.1",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result.is_empty(), "{result:?}");
    }

    #[test]
    fn name_fallback_on_a_public_suffix_query_does_not_match_its_subdomains() {
        let store = store_with(&[("sites/myapp.herokuapp.com/alice", "pw")]);
        let result = find_matches(
            &store,
            "personal",
            "herokuapp.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result.is_empty(), "{result:?}");
    }

    #[test]
    fn name_fallback_on_an_ip_query_needs_the_exact_address() {
        let store = store_with(&[("devices/10.0.0.1", "pw"), ("devices/198.51.0.1", "pw")]);
        let result = find_matches(
            &store,
            "personal",
            "198.51.0.1",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1, "{result:?}");
        assert_eq!(result[0].name, "devices/198.51.0.1");
    }

    #[test]
    fn ip_literals_have_no_registrable_domain() {
        assert_eq!(registrable_domain("10.0.1.1"), None);
        assert_eq!(registrable_domain("2001:db8::1"), None);
    }

    #[test]
    fn ipv4_url_field_matches_its_exact_host() {
        let store = store_with(&[("router", "pw\nurl: http://192.168.0.1:8080/")]);
        let result = find_matches(
            &store,
            "personal",
            "192.168.0.1",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
    }

    fn reveal_for(store: &PasswordStore, domain: &str, name: &str) -> Option<Credential> {
        reveal(
            store,
            "personal",
            domain,
            name,
            &uri_keys(),
            &username_keys(),
        )
    }

    #[test]
    fn reveal_returns_the_credential_of_a_matching_secret() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://www.example.com\nlogin: alice@example.com",
        )]);
        let credential =
            reveal_for(&store, "mail.example.com", "alice").expect("registrable match");
        assert_eq!(credential.name, "alice");
        assert_eq!(credential.store, "personal");
        assert_eq!(credential.username, "alice@example.com");
        assert_eq!(credential.password, "hunter2");
    }

    #[test]
    fn reveal_refuses_a_secret_that_does_not_match_the_domain() {
        let store = store_with(&[
            ("alice", "hunter2\nurl: https://example.com"),
            ("bob", "hunter3\nurl: https://example.org"),
        ]);
        assert!(reveal_for(&store, "example.com", "bob").is_none());
        assert!(reveal_for(&store, "example.org", "alice").is_none());
    }

    #[test]
    fn reveal_refuses_a_secret_that_does_not_exist() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        assert!(reveal_for(&store, "example.com", "carol").is_none());
    }

    #[test]
    fn reveal_refuses_an_empty_domain() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        assert!(reveal_for(&store, "", "alice").is_none());
    }

    #[test]
    fn reveal_admits_exactly_the_secrets_the_query_returns() {
        let store = store_with(&[
            ("sites/example.com/alice", "pw1"),
            ("bob", "pw2\nurl: https://www.example.com"),
            ("carol", "pw3\nurl: https://example.com"),
            ("dave", "pw4\nurl: https://example.org"),
            ("example.com_archive", "pw5"),
            (
                "sites/attacker.co.uk/eve",
                "pw6\nurl: https://attacker.co.uk",
            ),
        ]);
        for domain in [
            "example.com",
            "www.example.com",
            "example.org",
            "victim.co.uk",
            "attacker.co.uk",
            "",
        ] {
            let matched: Vec<String> =
                find_matches(&store, "personal", domain, &uri_keys(), &username_keys())
                    .into_iter()
                    .map(|m| m.name)
                    .collect();
            for name in store.secrets.keys() {
                assert_eq!(
                    reveal_for(&store, domain, name).is_some(),
                    matched.contains(name),
                    "{domain} / {name}"
                );
            }
        }
    }

    #[test]
    fn reveal_includes_totp_code_on_an_exact_host_match() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("alice".into(), totp_entry());
        let credential = reveal_for(&store, "example.com", "alice").expect("exact match");
        let otp = credential.otp.as_deref().expect("expected TOTP code");
        assert_eq!(otp.len(), 6, "TOTP should be zero-padded to digit width");
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn reveal_omits_totp_code_on_a_registrable_domain_match() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://www.example.com")]);
        store.otp.insert("alice".into(), totp_entry());
        let credential =
            reveal_for(&store, "mail.example.com", "alice").expect("registrable match");
        assert_eq!(credential.password, "hunter2");
        assert!(credential.otp.is_none());
    }

    #[test]
    fn reveal_omits_totp_code_on_a_name_match() {
        let mut store = store_with(&[("sites/example.com/alice", "hunter2")]);
        store
            .otp
            .insert("sites/example.com/alice".into(), totp_entry());
        let credential =
            reveal_for(&store, "example.com", "sites/example.com/alice").expect("name match");
        assert!(credential.otp.is_none());
    }

    #[test]
    fn reveal_omits_otp_for_hotp_entry_to_preserve_counter() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("alice".into(), hotp_entry());
        let credential = reveal_for(&store, "example.com", "alice").expect("exact match");
        assert!(credential.otp.is_none());
    }

    #[test]
    fn reveal_omits_otp_when_no_otp_entry_matches_secret_name() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("bob".into(), totp_entry());
        let credential = reveal_for(&store, "example.com", "alice").expect("exact match");
        assert!(credential.otp.is_none());
    }

    #[test]
    fn credential_serializes_without_an_absent_otp() {
        let credential = Credential {
            name: "alice".into(),
            store: "personal".into(),
            username: "alice@example.com".into(),
            password: "hunter2".into(),
            otp: None,
        };
        let json = serde_json::to_value(&credential).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["name", "password", "store", "username"], "{json}");
    }

    fn matched_names(store: &PasswordStore, domain: &str) -> Vec<String> {
        find_matches(store, "personal", domain, &uri_keys(), &username_keys())
            .into_iter()
            .map(|m| m.name)
            .collect()
    }

    #[test]
    fn fill_never_excludes_the_secret_from_query_and_reveal() {
        let store = store_with(&[
            (
                "alice",
                "hunter2\nurl: https://example.com\npasejo-fill: never",
            ),
            ("sites/example.com/bob", "hunter3\npasejo-fill: never"),
        ]);
        assert!(matched_names(&store, "example.com").is_empty());
        assert!(reveal_for(&store, "example.com", "alice").is_none());
        assert!(reveal_for(&store, "example.com", "sites/example.com/bob").is_none());
    }

    #[test]
    fn fill_exact_host_admits_only_exact_host_url_matches() {
        let store = store_with(&[
            (
                "alice",
                "hunter2\nurl: https://login.example.com\npasejo-fill: exact-host",
            ),
            ("sites/example.com/bob", "hunter3\npasejo-fill: exact-host"),
        ]);
        assert_eq!(matched_names(&store, "login.example.com"), ["alice"]);
        assert!(reveal_for(&store, "login.example.com", "alice").is_some());
        assert!(matched_names(&store, "www.example.com").is_empty());
        assert!(reveal_for(&store, "www.example.com", "alice").is_none());
        assert!(matched_names(&store, "example.com").is_empty());
        assert!(reveal_for(&store, "example.com", "sites/example.com/bob").is_none());
    }

    #[test]
    fn fill_exact_host_falls_back_to_no_match_when_only_a_sibling_url_matches() {
        let store = store_with(&[(
            "sites/example.com/alice",
            "hunter2\nurl: https://www.example.com\npasejo-fill: exact-host",
        )]);
        assert!(matched_names(&store, "mail.example.com").is_empty());
    }

    #[test]
    fn an_unrecognised_fill_policy_excludes_the_secret() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\npasejo-fill: exact_host",
        )]);
        assert!(matched_names(&store, "example.com").is_empty());
        assert!(reveal_for(&store, "example.com", "alice").is_none());
    }

    #[test]
    fn reveal_admits_exactly_the_secrets_the_query_returns_under_policies() {
        let store = store_with(&[
            ("a", "pw\nurl: https://example.com\npasejo-fill: exact-host"),
            (
                "b",
                "pw\nurl: https://www.example.com\npasejo-fill: exact-host",
            ),
            ("c", "pw\nurl: https://example.com\npasejo-fill: never"),
            ("sites/example.com/d", "pw\npasejo-fill: exact-host"),
            ("e", "pw\nurl: https://example.com\npasejo-confirm: always"),
        ]);
        for domain in ["example.com", "www.example.com", "mail.example.com"] {
            let matched = matched_names(&store, domain);
            for name in store.secrets.keys() {
                assert_eq!(
                    reveal_for(&store, domain, name).is_some(),
                    matched.contains(name),
                    "{domain} / {name}"
                );
            }
        }
    }

    #[test]
    fn confirm_policy_is_reported_in_the_query_metadata() {
        let store = store_with(&[
            (
                "alice",
                "hunter2\nurl: https://example.com\npasejo-confirm: always",
            ),
            ("bob", "hunter3\nurl: https://example.com"),
        ]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        let confirm_of = |name: &str| {
            result
                .iter()
                .find(|m| m.name == name)
                .map(|m| m.confirm)
                .unwrap()
        };
        assert!(confirm_of("alice"));
        assert!(!confirm_of("bob"));
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json[0]["confirm"], serde_json::Value::Bool(true), "{json}");
        assert_eq!(json[1]["confirm"], serde_json::Value::Bool(false), "{json}");
    }

    #[test]
    fn reserved_policy_fields_are_never_taken_as_the_username_or_url() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\npasejo-confirm: always",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result[0].username, "");
        assert_eq!(result[0].url.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn highest_scoring_url_field_per_secret_is_chosen() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://www.example.com\nlink: https://example.com",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        assert!((result[0].score - SCORE_URL_EXACT_HOST).abs() < f64::EPSILON);
        assert_eq!(result[0].url.as_deref(), Some("https://example.com"));
    }

    fn lookalikes_in(store: &PasswordStore, domain: &str) -> Vec<String> {
        find_lookalikes(store, domain, &uri_keys(), &username_keys())
    }

    #[test]
    fn lookalikes_name_stored_url_domains_resembling_the_query() {
        let store = store_with(&[
            ("alice", "hunter2\nurl: https://login.example.com"),
            ("bob", "hunter3\nurl: https://unrelated.org"),
        ]);
        assert_eq!(lookalikes_in(&store, "www.examp1e.com"), ["example.com"]);
    }

    #[test]
    fn lookalikes_consider_every_url_field_of_a_secret() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://unrelated.org\nlink: example.com/login",
        )]);
        assert_eq!(lookalikes_in(&store, "examp1e.com"), ["example.com"]);
    }

    #[test]
    fn lookalikes_ignore_fields_outside_the_uri_keys_and_secret_names() {
        let store = store_with(&[
            ("alice", "hunter2\nnotes: https://example.com"),
            ("sites/example.com/bob", "hunter3"),
        ]);
        assert!(lookalikes_in(&store, "examp1e.com").is_empty());
    }

    #[test]
    fn lookalikes_ignore_secrets_the_browser_may_never_fill() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\npasejo-fill: never",
        )]);
        assert!(lookalikes_in(&store, "examp1e.com").is_empty());
    }

    #[test]
    fn lookalikes_are_empty_for_a_query_without_a_host() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        assert!(lookalikes_in(&store, "").is_empty());
        assert!(lookalikes_in(&store, ".").is_empty());
    }

    #[test]
    fn lookalikes_compare_the_canonical_query_host() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        assert_eq!(lookalikes_in(&store, " EXAMP1E.com. "), ["example.com"]);
    }

    #[test]
    fn a_secret_held_back_by_its_fill_policy_keeps_its_own_domain_quiet() {
        let store = store_with(&[
            (
                "alice",
                "hunter2\nurl: https://www.example.com\npasejo-fill: exact-host",
            ),
            ("bob", "hunter3\nurl: https://example.net"),
        ]);
        assert!(matched_names(&store, "login.example.com").is_empty());
        assert!(lookalikes_in(&store, "login.example.com").is_empty());
    }
}
