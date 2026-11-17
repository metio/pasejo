// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use serde::Serialize;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::lookalike::lookalikes;
use crate::parser::{ParsedSecret, parse_secret};
use crate::policy::{FillPolicy, Policy};

/// URLs naming an Android app by package, which are not web domains and
/// take no part in look-alike detection.
const APP_SCHEME: &str = "androidapp://";

pub const SCORE_URL_EXACT_HOST: f64 = 1.0;
pub const SCORE_URL_REGISTRABLE_DOMAIN: f64 = 0.9;
pub const SCORE_NAME: f64 = 0.5;

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

#[must_use]
pub fn find_matches(
    secrets: &BTreeMap<String, String>,
    store_name: &str,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<Match> {
    let Some(query) = Query::parse(domain) else {
        return vec![];
    };
    let mut matches: Vec<Match> = secrets
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

/// Stored domains that could be mistaken for `domain`.
///
/// These are the registrable domains of the store's URL fields; see
/// `lookalike::lookalikes`. Secret names are not consulted,
/// and a secret with `pasejo-fill: never` contributes nothing, so the browser
/// learns no more about it than a query would.
#[must_use]
pub fn find_lookalikes(
    secrets: &BTreeMap<String, String>,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<String> {
    let Some(query) = Query::parse(domain) else {
        return vec![];
    };
    let stored_hosts = secrets
        .values()
        .filter(|value| Policy::from_secret(value).fill != FillPolicy::Never)
        .flat_map(|value| {
            parse_secret(value, username_keys, uri_keys)
                .fields
                .into_iter()
                .filter(|(key, _)| uri_keys.contains(key))
                .filter(|(_, field_value)| !field_value.trim().starts_with(APP_SCHEME))
                .filter_map(|(_, field_value)| extract_host(&field_value))
        });
    lookalikes(&query.domain, stored_hosts)
}

/// Releases the credential of `secret_name` for `domain`.
///
/// The credential is released only when that secret is one
/// `find_matches` returns for `domain`. A missing secret and a secret that
/// does not match both yield `None`, so the answer reveals nothing about
/// which names exist. `totp` renders the current TOTP code of the secret and
/// runs only for an exact-host match.
pub fn reveal(
    secrets: &BTreeMap<String, String>,
    store_name: &str,
    domain: &str,
    secret_name: &str,
    uri_keys: &[String],
    username_keys: &[String],
    totp: impl FnOnce() -> Option<String>,
) -> Option<Credential> {
    let query = Query::parse(domain)?;
    let value = secrets.get(secret_name)?;
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
    let otp = if matched.exact_host { totp() } else { None };
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

/// Reports whether `input` starts with a URL scheme.
///
/// The scheme follows the RFC 3986 grammar
/// (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) "://"`). A plain
/// `contains("://")` is not enough: a scheme-less stored value whose query
/// string embeds another URL (`example.com/redirect?to=https://evil.com`) contains the marker
/// without *starting* with a scheme, and feeding it to `Url::parse`
/// directly fails — the value would silently stop matching.
#[must_use]
pub fn has_url_scheme(input: &str) -> bool {
    let Some((scheme, _)) = input.split_once("://") else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

#[must_use]
pub fn extract_host(input: &str) -> Option<String> {
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
    host.trim_suffix('.').to_owned()
}

/// IP literals have no registrable domain: the public-suffix list's
/// default rule would treat their last octets as one, so two unrelated
/// addresses ending in the same octets would compare equal.
pub fn registrable_domain(host: &str) -> Option<String> {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    psl::domain_str(host).map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secrets(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect()
    }

    fn keys(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn app_urls_are_not_compared_as_lookalike_domains() {
        let stored = secrets(&[("mail", "pw\nurl: androidapp://com.example.mail")]);
        let found = find_lookalikes(&stored, "examp1e.mail", &keys(&["url"]), &keys(&["user"]));
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn reveal_asks_for_the_totp_code_only_on_an_exact_host_match() {
        let stored = secrets(&[
            ("exact", "pw\nurl: https://example.com"),
            ("sub", "pw\nurl: https://login.example.com"),
        ]);
        let (uri, user) = (keys(&["url"]), keys(&["user"]));
        let exact = reveal(&stored, "s", "example.com", "exact", &uri, &user, || {
            Some("1".into())
        });
        assert_eq!(exact.and_then(|c| c.otp.clone()).as_deref(), Some("1"));
        let sub = reveal(&stored, "s", "example.com", "sub", &uri, &user, || {
            panic!("asked for a code on a registrable-domain match")
        });
        assert!(sub.is_some_and(|c| c.otp.is_none()));
    }
}
