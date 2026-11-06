// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use serde::{Deserialize, Serialize};

use crate::browser::matcher::{Credential, Match, find_lookalikes, find_matches};
use crate::models::password_store::PasswordStore;

/// Wire-format version emitted in `ListStoresResponse::protocol_version`.
/// The extension refuses to operate against any host that doesn't
/// report the exact same value, so bumping this number is the
/// single mechanism we have for signalling a breaking change to the
/// request or response shapes. Version 2 is the two-phase shape: `query`
/// answers with metadata only and `reveal` releases one credential.
pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    Query {
        domain: String,
        #[serde(default)]
        store: Option<String>,
    },
    /// Releases the credential of one secret, after the host re-runs the
    /// matcher and finds `name` among the matches for `domain`. `store` is
    /// mandatory so a default store that changed since the query cannot
    /// redirect the reveal to a different secret of the same name.
    Reveal {
        store: String,
        name: String,
        domain: String,
    },
    ListStores,
}

/// Request envelope: pulls the optional `request_id` out of the JSON
/// payload before the action-specific deserialization. The field is
/// optional purely for defensive parsing — if a malformed payload
/// fails to deserialize as a full envelope, we still try to recover
/// the top-level `request_id` so the error reply can be correlated to
/// the request that triggered it. The extension always sends a
/// `request_id`; a host that sees `None` here is parsing a payload it
/// could not otherwise route.
#[derive(Debug, Deserialize)]
pub struct RequestEnvelope {
    #[serde(default)]
    pub request_id: Option<u64>,
    #[serde(flatten)]
    pub action: Request,
}

/// Response envelope: echoes the request's `request_id` verbatim
/// alongside the action-specific payload. `skip_serializing_if`
/// drops the field entirely when the inbound request had no
/// `request_id` (the defensive-parse path described above), so the
/// caller never sees a stray `null`. `untagged` on `ResponsePayload`
/// keeps the payload fields at the top level of the JSON object —
/// the wire shape is `{request_id?, ...payload}`, with no extra
/// nesting tag.
#[derive(Debug, Serialize)]
pub struct ResponseEnvelope<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<u64>,
    #[serde(flatten)]
    pub payload: &'a ResponsePayload,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ResponsePayload {
    Query(QueryResponse),
    Reveal(RevealResponse),
    ListStores(ListStoresResponse),
    Error(ErrorResponse),
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub results: Vec<Match>,
    /// Registrable domains of stored URLs that resemble the queried domain,
    /// present only when `results` is empty: the page may be imitating a
    /// site the user has an account for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lookalikes: Vec<String>,
}

impl QueryResponse {
    /// The answer to a query for `domain` against one decrypted store.
    pub fn search(
        store: &PasswordStore,
        store_name: &str,
        domain: &str,
        uri_keys: &[String],
        username_keys: &[String],
    ) -> Self {
        let results = find_matches(store, store_name, domain, uri_keys, username_keys);
        let lookalikes = if results.is_empty() {
            find_lookalikes(store, domain, uri_keys, username_keys)
        } else {
            vec![]
        };
        Self {
            results,
            lookalikes,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RevealResponse {
    pub credential: Credential,
}

#[derive(Debug, Serialize)]
pub struct ListStoresResponse {
    pub protocol_version: u32,
    pub stores: Vec<StoreEntry>,
}

#[derive(Debug, Serialize)]
pub struct StoreEntry {
    pub name: String,
    pub default: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    UnknownStore,
    // Reserved for the day the host distinguishes a known-but-unsupported
    // action (e.g. a future `create` action sent against an older host).
    // Today, every serde deserialization failure — including unknown
    // variant names — surfaces as `DecodeError` with the parser message in
    // `hint`.
    #[allow(dead_code)]
    UnknownAction,
    DecodeError,
    /// The identities could not be read, or none of them decrypts the
    /// store: only passphrase-protected identities (the host cannot prompt
    /// for a passphrase, so it skips them), "no identity files configured",
    /// "identities don't match recipients", or a failing age plugin.
    IdentityLocked,
    /// A pull or push hook command returned non-zero or could not be spawned.
    /// The hint carries the underlying anyhow chain so the extension can
    /// surface which hook went wrong.
    HookFailed,
    /// The store file itself could not be opened or locked (missing file,
    /// permissions, or a lock another operation held for longer than the
    /// host waits). Distinct from `IdentityLocked` because the fix is
    /// filesystem-side, not identity-side.
    StoreUnreachable,
    NoStoreConfigured,
    /// The serialized reply would exceed the browser's 1 MiB
    /// host→browser native-messaging limit. Writing it anyway would get
    /// the whole connection killed by the browser, failing every
    /// in-flight request — so the host downgrades the oversized reply
    /// to this per-request error instead.
    ResponseTooLarge,
    /// A `reveal` named a secret that is not among the matches for the
    /// requested domain, or a secret that does not exist; the two are
    /// indistinguishable so the reply cannot probe for secret names.
    NotMatched,
    /// A `reveal` matched, but its release could not be recorded in the
    /// local release log, so the credential was withheld.
    ReleaseLogFailed,
    /// The browser started this host for an extension that is not among
    /// the host's `--allowed-caller` values. Every request is answered with
    /// this kind and nothing else, so an unknown extension learns neither
    /// store names nor credentials.
    CallerNotAllowed,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::browser::matcher::MatchKind;
    use crate::cli::constants::{default_uri_keys, default_username_keys};

    fn sample_match() -> Match {
        Match {
            name: "alice".into(),
            store: "personal".into(),
            username: "alice@example.com".into(),
            url: Some("https://example.com".into()),
            match_kind: MatchKind::UrlField,
            score: 1.0,
            confirm: false,
            exact_host: true,
        }
    }

    #[test]
    fn caller_not_allowed_serializes_in_snake_case() {
        let response = ErrorResponse {
            error: ErrorKind::CallerNotAllowed,
            store: None,
            hint: Some("refused".into()),
        };
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"error":"caller_not_allowed","hint":"refused"}"#
        );
    }

    #[test]
    fn query_response_serializes_to_expected_shape() {
        let response = QueryResponse {
            results: vec![sample_match()],
            lookalikes: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let results = parsed.get("results").unwrap().as_array().unwrap();
        assert_eq!(results.len(), 1);
        let first = &results[0];
        assert_eq!(first.get("name").and_then(Value::as_str), Some("alice"));
        assert_eq!(first.get("store").and_then(Value::as_str), Some("personal"));
        assert_eq!(
            first.get("username").and_then(Value::as_str),
            Some("alice@example.com")
        );
        assert_eq!(
            first.get("url").and_then(Value::as_str),
            Some("https://example.com")
        );
        assert_eq!(
            first.get("match_kind").and_then(Value::as_str),
            Some("url_field")
        );
        assert_eq!(first.get("score").and_then(Value::as_f64), Some(1.0));
        assert_eq!(first.get("confirm").and_then(Value::as_bool), Some(false));
        for absent in ["password", "fields", "otp", "exact_host"] {
            assert!(first.get(absent).is_none(), "{absent} leaked: {json}");
        }
    }

    fn store_with(secrets: &[(&str, &str)]) -> PasswordStore {
        let mut store = PasswordStore::default();
        for (name, value) in secrets {
            store
                .secrets
                .insert((*name).to_owned(), (*value).to_owned());
        }
        store
    }

    fn search(store: &PasswordStore, domain: &str) -> QueryResponse {
        QueryResponse::search(
            store,
            "personal",
            domain,
            &default_uri_keys(),
            &default_username_keys(),
        )
    }

    #[test]
    fn query_response_without_lookalikes_omits_the_field() {
        let response = QueryResponse {
            results: vec![],
            lookalikes: vec![],
        };
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"results":[]}"#
        );
    }

    #[test]
    fn query_response_lists_its_lookalikes_after_the_results() {
        let response = QueryResponse {
            results: vec![],
            lookalikes: vec!["example.com".into(), "example.net".into()],
        };
        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"results":[],"lookalikes":["example.com","example.net"]}"#
        );
    }

    #[test]
    fn search_names_lookalikes_when_nothing_matches() {
        let store = store_with(&[("alice", "hunter2\nurl: https://example.com")]);
        let response = search(&store, "examp1e.com");
        assert!(response.results.is_empty());
        assert_eq!(response.lookalikes, ["example.com"]);
    }

    #[test]
    fn search_names_no_lookalikes_next_to_a_match() {
        let store = store_with(&[
            ("sites/example.com/alice", "hunter2"),
            ("bob", "hunter3\nurl: https://example.net"),
        ]);
        let response = search(&store, "example.com");
        assert_eq!(response.results.len(), 1);
        assert!(response.lookalikes.is_empty());
    }

    #[test]
    fn reveal_response_nests_the_credential() {
        let payload = ResponsePayload::Reveal(RevealResponse {
            credential: Credential {
                name: "alice".into(),
                store: "personal".into(),
                username: "alice@example.com".into(),
                password: "hunter2".into(),
                otp: Some("123456".into()),
            },
        });
        let envelope = ResponseEnvelope {
            request_id: Some(9),
            payload: &payload,
        };
        let parsed: Value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!({
                "request_id": 9,
                "credential": {
                    "name": "alice",
                    "store": "personal",
                    "username": "alice@example.com",
                    "password": "hunter2",
                    "otp": "123456",
                },
            })
        );
    }

    #[test]
    fn not_matched_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&ErrorKind::NotMatched).unwrap(),
            "\"not_matched\""
        );
    }

    #[test]
    fn release_log_failed_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&ErrorKind::ReleaseLogFailed).unwrap(),
            "\"release_log_failed\""
        );
    }

    #[test]
    fn request_reveal_round_trips() {
        let raw =
            r#"{"action": "reveal", "store": "work", "name": "alice", "domain": "example.com"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        match request {
            Request::Reveal {
                store,
                name,
                domain,
            } => {
                assert_eq!(store, "work");
                assert_eq!(name, "alice");
                assert_eq!(domain, "example.com");
            }
            other => panic!("expected Reveal variant, got {other:?}"),
        }
    }

    #[test]
    fn request_reveal_requires_store_name_and_domain() {
        for raw in [
            r#"{"action": "reveal", "name": "alice", "domain": "example.com"}"#,
            r#"{"action": "reveal", "store": "work", "domain": "example.com"}"#,
            r#"{"action": "reveal", "store": "work", "name": "alice"}"#,
        ] {
            assert!(serde_json::from_str::<Request>(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn match_kind_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&MatchKind::UrlField).unwrap(),
            "\"url_field\""
        );
        assert_eq!(serde_json::to_string(&MatchKind::Name).unwrap(), "\"name\"");
    }

    #[test]
    fn list_stores_response_marks_default() {
        let response = ListStoresResponse {
            protocol_version: PROTOCOL_VERSION,
            stores: vec![
                StoreEntry {
                    name: "personal".into(),
                    default: true,
                },
                StoreEntry {
                    name: "work".into(),
                    default: false,
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        let stores = parsed.get("stores").unwrap().as_array().unwrap();
        assert_eq!(stores.len(), 2);
        assert_eq!(
            stores[0].get("default").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            stores[1].get("default").and_then(Value::as_bool),
            Some(false)
        );
    }

    #[test]
    fn list_stores_response_includes_protocol_version() {
        let response = ListStoresResponse {
            protocol_version: PROTOCOL_VERSION,
            stores: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.get("protocol_version").and_then(Value::as_u64),
            Some(u64::from(PROTOCOL_VERSION))
        );
    }

    #[test]
    fn error_response_omits_absent_fields() {
        let response = ErrorResponse {
            error: ErrorKind::UnknownStore,
            store: Some("wrok".into()),
            hint: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.get("error").and_then(Value::as_str),
            Some("unknown_store")
        );
        assert_eq!(parsed.get("store").and_then(Value::as_str), Some("wrok"));
        assert!(parsed.get("hint").is_none());
    }

    #[test]
    fn request_query_with_store_field_round_trips() {
        let raw = r#"{"action": "query", "domain": "example.com", "store": "work"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        match request {
            Request::Query { domain, store } => {
                assert_eq!(domain, "example.com");
                assert_eq!(store.as_deref(), Some("work"));
            }
            other => panic!("expected Query variant, got {other:?}"),
        }
    }

    #[test]
    fn request_query_without_store_field_defaults_to_none() {
        let raw = r#"{"action": "query", "domain": "example.com"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        match request {
            Request::Query { store, .. } => assert!(store.is_none()),
            other => panic!("expected Query variant, got {other:?}"),
        }
    }

    #[test]
    fn request_list_stores_round_trips() {
        let raw = r#"{"action": "list_stores"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        assert!(matches!(request, Request::ListStores));
    }
}
