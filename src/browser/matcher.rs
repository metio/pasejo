// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The matcher of `pasejo-match` over a decrypted [`PasswordStore`].

pub use pasejo_match::matcher::{Credential, Match};
#[cfg(test)]
use pasejo_match::matcher::{
    MatchKind, SCORE_NAME, SCORE_URL_EXACT_HOST, SCORE_URL_REGISTRABLE_DOMAIN, extract_host,
    has_url_scheme, registrable_domain,
};

use crate::models::password_store::{OneTimePasswordType, PasswordStore};

/// See [`pasejo_match::matcher::find_matches`].
pub fn find_matches(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<Match> {
    pasejo_match::matcher::find_matches(&store.secrets, store_name, domain, uri_keys, username_keys)
}

/// See [`pasejo_match::matcher::find_lookalikes`].
pub fn find_lookalikes(
    store: &PasswordStore,
    domain: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Vec<String> {
    pasejo_match::matcher::find_lookalikes(&store.secrets, domain, uri_keys, username_keys)
}

/// See [`pasejo_match::matcher::reveal`]; the TOTP code comes from the
/// store's entry under the same name.
pub fn reveal(
    store: &PasswordStore,
    store_name: &str,
    domain: &str,
    secret_name: &str,
    uri_keys: &[String],
    username_keys: &[String],
) -> Option<Credential> {
    pasejo_match::matcher::reveal(
        &store.secrets,
        store_name,
        domain,
        secret_name,
        uri_keys,
        username_keys,
        || totp_code_for(store, secret_name),
    )
}

/// The current code of the TOTP entry stored under `secret_name`. `None` for
/// a missing entry, an HOTP entry (a code without bumping the counter breaks
/// RFC 4226), and a secret whose code cannot be generated.
fn totp_code_for(store: &PasswordStore, secret_name: &str) -> Option<String> {
    let entry = store.otp.get(secret_name)?;
    if entry.otp_type != OneTimePasswordType::Totp {
        return None;
    }
    let mut clone = entry.clone();
    let code = clone.generate().ok()?;
    Some(format!("{code:0width$}", width = entry.digits as usize))
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
