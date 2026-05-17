// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::collections::BTreeMap;

use serde::Serialize;

use crate::exporters::parser::parse_secret;
use crate::models::password_store::PasswordStore;

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
}

pub fn find_matches(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<Match> {
    if domain.is_empty() {
        return vec![];
    }
    let query_domain = domain.to_lowercase();
    let query_registrable = registrable_domain(&query_domain);

    let mut matches: Vec<Match> = store
        .secrets
        .iter()
        .filter_map(|(name, value)| {
            match_secret(
                name,
                value,
                store_name,
                &query_domain,
                query_registrable.as_deref(),
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

fn match_secret(
    name: &str,
    value: &str,
    store_name: &str,
    query_domain: &str,
    query_registrable: Option<&str>,
    uri_keys: &[String],
    username_keys: &[String],
) -> Option<Match> {
    let parsed = parse_secret(value);

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

fn name_matches_domain(name: &str, query_domain: &str, query_registrable: Option<&str>) -> bool {
    let needle = query_registrable.unwrap_or(query_domain);
    let dotted = format!(".{needle}");
    for segment in name.split(['/', '\\']) {
        let lower = segment.to_lowercase();
        if lower == needle || lower.ends_with(&dotted) {
            return true;
        }
    }
    false
}

fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .filter(|s| !s.is_empty())?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(stripped) = host_port.strip_prefix('[') {
        stripped.split_once(']').map(|(h, _)| h)?
    } else {
        host_port.rsplit_once(':').map_or(host_port, |(h, _)| h)
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

fn registrable_domain(host: &str) -> Option<String> {
    psl::domain_str(host).map(str::to_lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (name, value) in secrets {
            store
                .secrets
                .insert((*name).to_owned(), (*value).to_owned());
        }
        store
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
    fn ipv6_bracketed_host_does_not_crash() {
        let store = store_with(&[("alice", "hunter2\nurl: https://[::1]:8443/x")]);
        let result = find_matches(&store, "personal", "::1", &uri_keys(), &username_keys());
        assert!(result.is_empty() || result[0].match_kind == MatchKind::UrlField);
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
