// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{Read, Write};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::browser::matcher::find_matches;
use crate::browser::protocol::{
    ErrorKind, ErrorResponse, ListStoresResponse, QueryResponse, Request, StoreEntry,
};
use crate::commands::store_op::{StoreMutation, with_store};
use crate::models::configuration::{Configuration, StoreRegistration};

// Chrome's documented native-messaging maximum is 1 MiB per direction.
// Rejecting larger length prefixes prevents a malformed peer from driving
// the host into multi-gigabyte allocations.
#[allow(dead_code)]
pub const MAX_MESSAGE_LEN: usize = 1024 * 1024;

pub struct HostContext<'a> {
    pub only_store: &'a [String],
    pub default_store: Option<&'a str>,
    pub uri_keys: &'a [String],
    pub username_keys: &'a [String],
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum ResponsePayload {
    Query(QueryResponse),
    ListStores(ListStoresResponse),
    Error(ErrorResponse),
}

#[allow(dead_code)]
pub fn handle_request(
    request: &Request,
    configuration: &Configuration,
    context: &HostContext<'_>,
    offline: bool,
) -> ResponsePayload {
    match request {
        Request::Query { domain, store } => {
            handle_query(configuration, context, store.as_deref(), domain, offline)
        }
        Request::ListStores => {
            ResponsePayload::ListStores(list_stores_response(configuration, context))
        }
    }
}

fn handle_query(
    configuration: &Configuration,
    context: &HostContext<'_>,
    requested_store: Option<&str>,
    domain: &str,
    offline: bool,
) -> ResponsePayload {
    let registration = match resolve_store(configuration, context, requested_store) {
        Ok(reg) => reg,
        Err(err) => return ResponsePayload::Error(err),
    };
    let store_name = registration.name.clone();
    match with_store(configuration, Some(&store_name), offline, |reg, store| {
        let results = find_matches(
            store,
            &reg.name,
            domain,
            context.uri_keys,
            context.username_keys,
        );
        Ok((results, StoreMutation::Unchanged))
    }) {
        Ok(results) => ResponsePayload::Query(QueryResponse { results }),
        Err(err) => ResponsePayload::Error(ErrorResponse {
            error: ErrorKind::IdentityLocked,
            store: Some(store_name),
            hint: Some(format!("{err:#}")),
        }),
    }
}

pub fn resolve_store<'a>(
    configuration: &'a Configuration,
    context: &HostContext<'_>,
    requested: Option<&str>,
) -> std::result::Result<&'a StoreRegistration, ErrorResponse> {
    if let Some(name) = requested {
        return resolve_explicit(configuration, context, name);
    }
    resolve_fallback(configuration, context)
}

fn resolve_explicit<'a>(
    configuration: &'a Configuration,
    context: &HostContext<'_>,
    name: &str,
) -> std::result::Result<&'a StoreRegistration, ErrorResponse> {
    if is_excluded(context, name) {
        return Err(unknown_store_error(name));
    }
    configuration
        .stores
        .iter()
        .find(|s| s.name == name)
        .ok_or_else(|| unknown_store_error(name))
}

fn resolve_fallback<'a>(
    configuration: &'a Configuration,
    context: &HostContext<'_>,
) -> std::result::Result<&'a StoreRegistration, ErrorResponse> {
    let registration = if let Some(default) = context.default_store {
        configuration
            .stores
            .iter()
            .find(|s| s.name == default)
            .ok_or_else(no_store_configured_error)?
    } else {
        configuration
            .select_store(None)
            .ok_or_else(no_store_configured_error)?
    };
    if is_excluded(context, &registration.name) {
        return Err(no_store_configured_error());
    }
    Ok(registration)
}

fn is_excluded(context: &HostContext<'_>, name: &str) -> bool {
    !context.only_store.is_empty() && !context.only_store.iter().any(|s| s == name)
}

fn unknown_store_error(name: &str) -> ErrorResponse {
    ErrorResponse {
        error: ErrorKind::UnknownStore,
        store: Some(name.to_owned()),
        hint: None,
    }
}

const fn no_store_configured_error() -> ErrorResponse {
    ErrorResponse {
        error: ErrorKind::NoStoreConfigured,
        store: None,
        hint: None,
    }
}

pub fn list_stores_response(
    configuration: &Configuration,
    context: &HostContext<'_>,
) -> ListStoresResponse {
    let default_name = resolve_fallback(configuration, context)
        .ok()
        .map(|r| r.name.clone());
    let stores = configuration
        .stores
        .iter()
        .filter(|reg| !is_excluded(context, &reg.name))
        .map(|reg| StoreEntry {
            name: reg.name.clone(),
            default: default_name.as_deref() == Some(reg.name.as_str()),
        })
        .collect();
    ListStoresResponse { stores }
}

#[allow(dead_code)]
pub fn read_message<R: Read>(reader: &mut R) -> Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_LEN {
        bail!("message length {len} exceeds the {MAX_MESSAGE_LEN}-byte native-messaging limit");
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    Ok(Some(buf))
}

#[allow(dead_code)]
pub fn write_message<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<()> {
    let len = u32::try_from(bytes.len())?;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::cli::environment_variables::PASEJO_DEFAULT_STORE;

    fn registration(name: &str) -> StoreRegistration {
        StoreRegistration {
            name: name.to_owned(),
            ..StoreRegistration::default()
        }
    }

    fn configuration_with(stores: &[&str], default_store: Option<&str>) -> Configuration {
        Configuration {
            stores: stores.iter().map(|n| registration(n)).collect(),
            default_store: default_store.map(str::to_owned),
            ..Configuration::default()
        }
    }

    fn context<'a>(only_store: &'a [String], default_store: Option<&'a str>) -> HostContext<'a> {
        HostContext {
            only_store,
            default_store,
            uri_keys: &[],
            username_keys: &[],
        }
    }

    /// `Configuration::select_store(None)` consults `PASEJO_DEFAULT_STORE`
    /// before falling back to the config field; running these tests with
    /// that variable set would mask the in-process fallback. Skip rather
    /// than fail so the suite stays green on a developer's environment
    /// while still asserting behaviour everywhere else.
    fn skip_if_default_store_env_set() -> bool {
        std::env::var_os(PASEJO_DEFAULT_STORE).is_some()
    }

    // --- Framing ----------------------------------------------------------

    #[test]
    fn framing_round_trips_a_simple_payload() {
        let mut buf = Vec::new();
        write_message(&mut buf, b"hello world").unwrap();
        let mut reader = Cursor::new(buf);
        let out = read_message(&mut reader).unwrap();
        assert_eq!(out.as_deref(), Some(&b"hello world"[..]));
    }

    #[test]
    fn framing_round_trips_an_empty_payload() {
        let mut buf = Vec::new();
        write_message(&mut buf, b"").unwrap();
        let mut reader = Cursor::new(buf);
        let out = read_message(&mut reader).unwrap();
        assert_eq!(out.as_deref(), Some(&b""[..]));
    }

    #[test]
    fn read_message_returns_none_on_immediate_eof() {
        let mut reader = Cursor::new(Vec::<u8>::new());
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn read_message_treats_truncated_length_prefix_as_eof() {
        // The 4-byte read fails with `UnexpectedEof` whether 0 or 1–3
        // bytes were available; the host treats both as a clean browser
        // disconnect rather than erroring on the malformed tail.
        let mut reader = Cursor::new(vec![1, 0]);
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn read_message_errors_on_truncated_payload() {
        // Length prefix claims 10 bytes; only 3 follow.
        let mut bytes = (10u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(b"abc");
        let mut reader = Cursor::new(bytes);
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn read_message_rejects_oversized_payload() {
        let oversized = u32::try_from(MAX_MESSAGE_LEN + 1).unwrap();
        let bytes = oversized.to_le_bytes().to_vec();
        let mut reader = Cursor::new(bytes);
        let err = read_message(&mut reader).unwrap_err().to_string();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn read_message_accepts_payload_at_size_limit() {
        let max_len = u32::try_from(MAX_MESSAGE_LEN).unwrap();
        let mut bytes = max_len.to_le_bytes().to_vec();
        bytes.extend(std::iter::repeat_n(0u8, MAX_MESSAGE_LEN));
        let mut reader = Cursor::new(bytes);
        let out = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(out.len(), MAX_MESSAGE_LEN);
    }

    // --- resolve_store ----------------------------------------------------

    #[test]
    fn resolve_explicit_finds_named_store() {
        let cfg = configuration_with(&["personal", "work"], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let reg = resolve_store(&cfg, &ctx, Some("work")).unwrap();
        assert_eq!(reg.name, "work");
    }

    #[test]
    fn resolve_explicit_missing_returns_unknown_store() {
        let cfg = configuration_with(&["personal"], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let err = resolve_store(&cfg, &ctx, Some("ghost")).unwrap_err();
        assert_eq!(err.error, ErrorKind::UnknownStore);
        assert_eq!(err.store.as_deref(), Some("ghost"));
    }

    #[test]
    fn resolve_explicit_excluded_by_only_store_returns_unknown_store() {
        // Policy-leak prevention: a store that exists but is outside the
        // --only-store allowlist must be indistinguishable from a store
        // that does not exist at all.
        let cfg = configuration_with(&["personal", "work"], None);
        let only = vec!["personal".to_owned()];
        let ctx = context(&only, None);
        let err = resolve_store(&cfg, &ctx, Some("work")).unwrap_err();
        assert_eq!(err.error, ErrorKind::UnknownStore);
        assert_eq!(err.store.as_deref(), Some("work"));
    }

    #[test]
    fn resolve_fallback_prefers_host_default_over_config_default() {
        let cfg = configuration_with(&["personal", "work"], Some("personal"));
        let only: Vec<String> = vec![];
        let ctx = context(&only, Some("work"));
        let reg = resolve_store(&cfg, &ctx, None).unwrap();
        assert_eq!(reg.name, "work");
    }

    #[test]
    fn resolve_fallback_uses_config_default_when_no_host_default() {
        let cfg = configuration_with(&["personal", "work"], Some("work"));
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let reg = resolve_store(&cfg, &ctx, None).unwrap();
        assert_eq!(reg.name, "work");
    }

    #[test]
    fn resolve_fallback_uses_first_store_when_no_defaults() {
        if skip_if_default_store_env_set() {
            return;
        }
        let cfg = configuration_with(&["personal", "work"], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let reg = resolve_store(&cfg, &ctx, None).unwrap();
        assert_eq!(reg.name, "personal");
    }

    #[test]
    fn resolve_fallback_with_no_stores_returns_no_store_configured() {
        let cfg = configuration_with(&[], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let err = resolve_store(&cfg, &ctx, None).unwrap_err();
        assert_eq!(err.error, ErrorKind::NoStoreConfigured);
    }

    #[test]
    fn resolve_fallback_with_missing_host_default_returns_no_store_configured() {
        let cfg = configuration_with(&["personal"], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, Some("ghost"));
        let err = resolve_store(&cfg, &ctx, None).unwrap_err();
        assert_eq!(err.error, ErrorKind::NoStoreConfigured);
    }

    #[test]
    fn resolve_fallback_picks_only_excluded_store_returns_no_store_configured() {
        if skip_if_default_store_env_set() {
            return;
        }
        // The fallback chain would pick "personal" (first store), but
        // --only-store=work excludes it.
        let cfg = configuration_with(&["personal", "work"], None);
        let only = vec!["work".to_owned()];
        let ctx = context(&only, None);
        let err = resolve_store(&cfg, &ctx, None).unwrap_err();
        assert_eq!(err.error, ErrorKind::NoStoreConfigured);
    }

    // --- list_stores_response --------------------------------------------

    #[test]
    fn list_stores_empty_configuration_returns_empty_list() {
        let cfg = configuration_with(&[], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let response = list_stores_response(&cfg, &ctx);
        assert!(response.stores.is_empty());
    }

    #[test]
    fn list_stores_marks_fallback_as_default() {
        if skip_if_default_store_env_set() {
            return;
        }
        let cfg = configuration_with(&["personal", "work"], None);
        let only: Vec<String> = vec![];
        let ctx = context(&only, None);
        let response = list_stores_response(&cfg, &ctx);
        assert_eq!(response.stores.len(), 2);
        assert!(
            response
                .stores
                .iter()
                .find(|s| s.name == "personal")
                .unwrap()
                .default
        );
        assert!(
            !response
                .stores
                .iter()
                .find(|s| s.name == "work")
                .unwrap()
                .default
        );
    }

    #[test]
    fn list_stores_respects_only_store_filter() {
        let cfg = configuration_with(&["personal", "work", "side"], None);
        let only = vec!["personal".to_owned(), "side".to_owned()];
        let ctx = context(&only, None);
        let response = list_stores_response(&cfg, &ctx);
        let names: Vec<&str> = response.stores.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["personal", "side"]);
    }

    #[test]
    fn list_stores_default_marker_respects_host_default() {
        let cfg = configuration_with(&["personal", "work"], Some("personal"));
        let only: Vec<String> = vec![];
        let ctx = context(&only, Some("work"));
        let response = list_stores_response(&cfg, &ctx);
        assert!(
            response
                .stores
                .iter()
                .find(|s| s.name == "work")
                .unwrap()
                .default
        );
        assert!(
            !response
                .stores
                .iter()
                .find(|s| s.name == "personal")
                .unwrap()
                .default
        );
    }

    #[test]
    fn list_stores_no_default_marker_when_fallback_excluded() {
        if skip_if_default_store_env_set() {
            return;
        }
        // Fallback would pick "personal" (first store), but --only-store=work
        // excludes it. Result: "work" is visible but has no default marker
        // because there is no resolvable fallback in the allowlist.
        let cfg = configuration_with(&["personal", "work"], None);
        let only = vec!["work".to_owned()];
        let ctx = context(&only, None);
        let response = list_stores_response(&cfg, &ctx);
        assert_eq!(response.stores.len(), 1);
        assert!(!response.stores[0].default);
    }
}
