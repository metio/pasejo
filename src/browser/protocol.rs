// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use serde::{Deserialize, Serialize};

use crate::browser::matcher::Match;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Request {
    Query {
        domain: String,
        #[serde(default)]
        store: Option<String>,
    },
    ListStores,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub results: Vec<Match>,
}

#[derive(Debug, Serialize)]
pub struct ListStoresResponse {
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
    IdentityLocked,
    NoStoreConfigured,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::Value;

    use super::*;
    use crate::browser::matcher::MatchKind;

    fn sample_match() -> Match {
        Match {
            name: "alice".into(),
            store: "personal".into(),
            username: "alice@example.com".into(),
            password: "hunter2".into(),
            url: Some("https://example.com".into()),
            match_kind: MatchKind::UrlField,
            score: 1.0,
            fields: BTreeMap::new(),
        }
    }

    #[test]
    fn query_response_serializes_to_expected_shape() {
        let response = QueryResponse {
            results: vec![sample_match()],
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
            first.get("password").and_then(Value::as_str),
            Some("hunter2")
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
        assert!(first.get("fields").unwrap().is_object());
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
            Request::ListStores => panic!("expected Query variant"),
        }
    }

    #[test]
    fn request_query_without_store_field_defaults_to_none() {
        let raw = r#"{"action": "query", "domain": "example.com"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        match request {
            Request::Query { store, .. } => assert!(store.is_none()),
            Request::ListStores => panic!("expected Query variant"),
        }
    }

    #[test]
    fn request_list_stores_round_trips() {
        let raw = r#"{"action": "list_stores"}"#;
        let request: Request = serde_json::from_str(raw).unwrap();
        assert!(matches!(request, Request::ListStores));
    }
}
