// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use serde::Serialize;

use crate::exporters::parser::parse_secret;
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

#[derive(Debug, Serialize, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Match {
    pub name: String,
    pub store: String,
    pub username: String,
    pub password: String,
    pub url: Option<String>,
    pub match_kind: MatchKind,
    pub score: f64,
    pub fields: BTreeMap<String, String>,
    /// Current TOTP code if `store.otp` has a TOTP entry under the same
    /// name as the matched secret. HOTP entries are deliberately
    /// excluded: emitting an HOTP code without bumping the counter
    /// breaks RFC 4226's single-use contract, and bumping it would turn
    /// the browser query path (currently read-only) into a writer that
    /// has to re-encrypt the store on every popup open.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub otp: Option<String>,
}

pub fn find_matches(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<Match> {
    let query_domain = canonical_host(domain);
    if query_domain.is_empty() {
        return vec![];
    }
    let query_registrable = registrable_domain(&query_domain);

    let mut matches: Vec<Match> = store
        .secrets
        .iter()
        .filter_map(|(name, value)| {
            let mut m = match_secret(
                name,
                value,
                store_name,
                &query_domain,
                query_registrable.as_deref(),
                uri_keys,
                username_keys,
            )?;
            m.otp = totp_code_for(store, name);
            Some(m)
        })
        .collect();
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    matches
}

fn match_secret(
    name: &str,
    value: &str,
    store_name: &str,
    query_domain: &str,
    query_registrable: Option<&str>,
    uri_keys: &[String],
    username_keys: &[String],
) -> Option<Match> {
    let parsed = parse_secret(value, username_keys, uri_keys);

    let mut username = String::new();
    let mut other_fields: BTreeMap<String, String> = BTreeMap::new();
    let mut best_url: Option<(f64, String)> = None;

    for (key, field_value) in &parsed.fields {
        if username_keys.contains(key) {
            username.clone_from(field_value);
            continue;
        }
        if uri_keys.contains(key) {
            if let Some((score, url)) =
                score_url_field(field_value, query_domain, query_registrable)
                && best_url
                    .as_ref()
                    .is_none_or(|(current_score, _)| score > *current_score)
            {
                best_url = Some((score, url));
            }
            continue;
        }
        other_fields.insert(key.clone(), field_value.clone());
    }

    if let Some((score, url)) = best_url {
        return Some(Match {
            name: name.to_owned(),
            store: store_name.to_owned(),
            username,
            password: parsed.password.unwrap_or_default(),
            url: Some(url),
            match_kind: MatchKind::UrlField,
            score,
            fields: other_fields,
            otp: None,
        });
    }

    if name_matches_domain(name, query_domain, query_registrable) {
        return Some(Match {
            name: name.to_owned(),
            store: store_name.to_owned(),
            username,
            password: parsed.password.unwrap_or_default(),
            url: None,
            match_kind: MatchKind::Name,
            score: SCORE_NAME,
            fields: other_fields,
            otp: None,
        });
    }

    None
}

/// Look up a TOTP entry stored under the same name as the matched
/// secret and render its current code. Returns `None` for non-existent
/// entries, for HOTP entries (see the doc comment on `Match::otp`), and
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
        assert_eq!(result[0].password, "hunter2");
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
    fn username_field_populates_username_not_fields() {
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
        assert!(!result[0].fields.contains_key("login"));
    }

    #[test]
    fn aligned_username_and_url_values_are_trimmed_but_password_and_fields_are_not() {
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
        assert_eq!(result[0].password, " hunter2 ");
        assert_eq!(
            result[0].fields.get("note").map(String::as_str),
            Some("  padded  ")
        );
    }

    #[test]
    fn custom_fields_outside_username_and_uri_keys_appear_in_fields() {
        let store = store_with(&[(
            "alice",
            "hunter2\nurl: https://example.com\nnote: backup codes here",
        )]);
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(
            result[0].fields.get("note").map(String::as_str),
            Some("backup codes here")
        );
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

    #[test]
    fn match_includes_totp_code_when_otp_entry_exists_for_secret_name() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("alice".into(), totp_entry());
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert_eq!(result.len(), 1);
        let otp = result[0].otp.as_deref().expect("expected TOTP code");
        assert_eq!(otp.len(), 6, "TOTP should be zero-padded to digit width");
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn match_omits_otp_for_hotp_entry_to_preserve_counter() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("alice".into(), hotp_entry());
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result[0].otp.is_none());
    }

    #[test]
    fn match_omits_otp_when_no_otp_entry_matches_secret_name() {
        let mut store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        store.otp.insert("bob".into(), totp_entry());
        let result = find_matches(
            &store,
            "personal",
            "example.com",
            &uri_keys(),
            &username_keys(),
        );
        assert!(result[0].otp.is_none());
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
}
