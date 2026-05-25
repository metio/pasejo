// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{IsTerminal, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Result, bail};

use crate::browser::matcher::find_matches;
use crate::browser::protocol::{
    ErrorKind, ErrorResponse, ListStoresResponse, PROTOCOL_VERSION, QueryResponse, Request,
    RequestEnvelope, ResponseEnvelope, ResponsePayload, StoreEntry,
};
use crate::cli::i18n;
use crate::commands::store_op::acquire_store_lock;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::HostArgs;
use crate::models::configuration::{Configuration, StoreRegistration};
use crate::models::password_store::PasswordStore;

// Chrome's documented native-messaging maximum is 1 MiB per direction.
// Rejecting larger length prefixes prevents a malformed peer from driving
// the host into multi-gigabyte allocations.
pub const MAX_MESSAGE_LEN: usize = 1024 * 1024;

/// Borrowed by every request handler. Owns its keys (rather than
/// borrowing from `HostArgs`) so the same value can ride an `Arc` into
/// detached worker threads without lifetime gymnastics.
#[derive(Debug, Default)]
pub struct HostContext {
    pub uri_keys: Vec<String>,
    pub username_keys: Vec<String>,
}

impl HostContext {
    pub fn from_args(args: &HostArgs) -> Self {
        Self {
            uri_keys: args.uri_keys.clone(),
            username_keys: args.username_keys.clone(),
        }
    }
}

pub fn handle_request(
    request: &Request,
    configuration: &Configuration,
    context: &HostContext,
    offline: bool,
) -> ResponsePayload {
    match request {
        Request::Query { domain, store } => {
            handle_query(configuration, context, store.as_deref(), domain, offline)
        }
        Request::ListStores => ResponsePayload::ListStores(list_stores_response(configuration)),
    }
}

fn handle_query(
    configuration: &Configuration,
    context: &HostContext,
    requested_store: Option<&str>,
    domain: &str,
    offline: bool,
) -> ResponsePayload {
    let registration = match resolve_store(configuration, requested_store) {
        Ok(reg) => reg,
        Err(err) => return ResponsePayload::Error(err),
    };
    let store_name = registration.name.clone();

    let store = match open_store_for_read(configuration, registration, offline) {
        Ok(store) => store,
        Err((kind, err)) => return error_response(kind, &store_name, &err),
    };

    let results = find_matches(
        &store,
        &store_name,
        domain,
        &context.uri_keys,
        &context.username_keys,
    );
    ResponsePayload::Query(QueryResponse { results })
}

/// Hand-rolled equivalent of the read-only prefix of `store_op::with_store`
/// (minus the encrypt + push tail, which a read-only operation never
/// needs). Pull hooks fire (throttled), the file lock is acquired,
/// `decrypt_store` runs, the lock is released, and the decrypted store is
/// returned. Each step's failure is classified into a distinct `ErrorKind`
/// so the extension can surface the right fix — the funnel itself returns
/// `anyhow::Error`, which would collapse those into a single chain and
/// leave the user guessing.
///
/// `acquire_store_lock` returning `Ok(None)` (the bootstrap path — a
/// store registered in the config but with no file on disk yet) is
/// mapped to `StoreUnreachable`; the browser host is never the right
/// entry point for bootstrapping, and letting `decrypt_store` fail
/// later would mis-classify as `IdentityLocked`.
fn open_store_for_read(
    configuration: &Configuration,
    registration: &StoreRegistration,
    offline: bool,
) -> std::result::Result<PasswordStore, (ErrorKind, anyhow::Error)> {
    let hooks = HookExecutor {
        configuration,
        registration,
        offline,
        force: false,
    };
    hooks
        .execute_pull_commands()
        .map_err(|err| (ErrorKind::HookFailed, err))?;

    let lock = match acquire_store_lock(registration.path()) {
        Ok(Some(lock)) => Some(lock),
        Ok(None) => {
            return Err((
                ErrorKind::StoreUnreachable,
                anyhow::anyhow!(
                    "store file does not exist at {}",
                    registration.path().display()
                ),
            ));
        }
        Err(err) => return Err((ErrorKind::StoreUnreachable, err)),
    };

    let store = configuration
        .decrypt_store(registration)
        .map_err(|err| (ErrorKind::IdentityLocked, err))?;
    drop(lock);
    Ok(store)
}

fn error_response(kind: ErrorKind, store: &str, err: &anyhow::Error) -> ResponsePayload {
    ResponsePayload::Error(ErrorResponse {
        error: kind,
        store: Some(store.to_owned()),
        hint: Some(format!("{err:#}")),
    })
}

pub fn resolve_store<'a>(
    configuration: &'a Configuration,
    requested: Option<&str>,
) -> std::result::Result<&'a StoreRegistration, ErrorResponse> {
    let owned = requested.map(String::from);
    configuration
        .select_store(owned.as_ref())
        .ok_or_else(|| requested.map_or_else(no_store_configured_error, unknown_store_error))
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

pub fn list_stores_response(configuration: &Configuration) -> ListStoresResponse {
    let default_name = configuration.select_store(None).map(|r| r.name.as_str());
    ListStoresResponse {
        protocol_version: PROTOCOL_VERSION,
        stores: configuration
            .stores
            .iter()
            .map(|reg| StoreEntry {
                name: reg.name.clone(),
                default: default_name == Some(reg.name.as_str()),
            })
            .collect(),
    }
}

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

pub fn write_message<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<()> {
    let len = u32::try_from(bytes.len())?;
    writer.write_all(&len.to_le_bytes())?;
    writer.write_all(bytes)?;
    writer.flush()?;
    Ok(())
}

pub fn run(configuration: &Configuration, args: &HostArgs, offline: bool) -> Result<()> {
    if std::io::stdin().is_terminal() {
        bail!(i18n::error_browser_host_requires_pipe());
    }

    let configuration = Arc::new(configuration.clone());
    let context = Arc::new(HostContext::from_args(args));
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let stdin = std::io::stdin();
    let mut input = stdin.lock();

    // Detached worker per request. The main thread keeps draining stdin
    // so the next request can be dispatched while earlier ones are
    // still blocked inside `decrypt_store` (e.g. waiting on a YubiKey
    // touch) — a slow decrypt holds back only its own slot, every
    // other store answers as fast as its own decrypt allows. The
    // process exits as soon as stdin closes — Rust auto-terminates
    // the workers, which is fine because every step is read-only on
    // the store file (the lock is released by the OS on fd close,
    // hooks are restartable, age plugins exit on stdin EOF).
    while let Some(bytes) = read_message(&mut input)? {
        let configuration = Arc::clone(&configuration);
        let context = Arc::clone(&context);
        let stdout = Arc::clone(&stdout);
        thread::spawn(move || {
            handle_one_message(&bytes, &configuration, &context, offline, &stdout);
        });
    }
    Ok(())
}

/// Per-worker entry point: parse, dispatch, serialize, and write. Any
/// I/O error on the final stdout write is logged to stderr (the only
/// channel still available) and silently dropped — the worker has
/// nowhere else to report it, and the extension's per-request timeout
/// will surface a transport error to the user anyway.
fn handle_one_message(
    bytes: &[u8],
    configuration: &Configuration,
    context: &HostContext,
    offline: bool,
    stdout: &Mutex<std::io::Stdout>,
) {
    let (request_id, payload) = process_message(bytes, configuration, context, offline);
    let envelope = ResponseEnvelope {
        request_id,
        payload: &payload,
    };
    let response_bytes = match serde_json::to_vec(&envelope) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("pasejo browser host: failed to serialize response: {err}");
            return;
        }
    };
    let mut out = match stdout.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Err(err) = write_message(&mut *out, &response_bytes) {
        eprintln!("pasejo browser host: failed to write response: {err}");
    }
}

fn process_message(
    bytes: &[u8],
    configuration: &Configuration,
    context: &HostContext,
    offline: bool,
) -> (Option<u64>, ResponsePayload) {
    match serde_json::from_slice::<RequestEnvelope>(bytes) {
        Ok(envelope) => (
            envelope.request_id,
            handle_request(&envelope.action, configuration, context, offline),
        ),
        Err(err) => (
            decode_error_request_id(bytes),
            ResponsePayload::Error(ErrorResponse {
                error: ErrorKind::DecodeError,
                store: None,
                hint: Some(err.to_string()),
            }),
        ),
    }
}

/// On a malformed payload the envelope deserialization fails wholesale,
/// so we still want to echo `request_id` if the caller managed to put
/// a parseable number under that key — a multiplexing client needs to
/// match the error reply back to its request. Best-effort: returns
/// `None` for anything that doesn't yield a u64.
fn decode_error_request_id(bytes: &[u8]) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    value.get("request_id")?.as_u64()
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

    fn empty_context() -> HostContext {
        HostContext::default()
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
    fn resolve_store_with_explicit_name_finds_named_store() {
        let cfg = configuration_with(&["personal", "work"], None);
        let reg = resolve_store(&cfg, Some("work")).unwrap();
        assert_eq!(reg.name, "work");
    }

    #[test]
    fn resolve_store_with_missing_name_returns_unknown_store() {
        let cfg = configuration_with(&["personal"], None);
        let err = resolve_store(&cfg, Some("ghost")).unwrap_err();
        assert_eq!(err.error, ErrorKind::UnknownStore);
        assert_eq!(err.store.as_deref(), Some("ghost"));
    }

    #[test]
    fn resolve_store_without_name_uses_config_default() {
        let cfg = configuration_with(&["personal", "work"], Some("work"));
        let reg = resolve_store(&cfg, None).unwrap();
        assert_eq!(reg.name, "work");
    }

    #[test]
    fn resolve_store_without_name_falls_back_to_first_store() {
        if skip_if_default_store_env_set() {
            return;
        }
        let cfg = configuration_with(&["personal", "work"], None);
        let reg = resolve_store(&cfg, None).unwrap();
        assert_eq!(reg.name, "personal");
    }

    #[test]
    fn resolve_store_with_empty_configuration_returns_no_store_configured() {
        let cfg = configuration_with(&[], None);
        let err = resolve_store(&cfg, None).unwrap_err();
        assert_eq!(err.error, ErrorKind::NoStoreConfigured);
    }

    // --- list_stores_response --------------------------------------------

    #[test]
    fn list_stores_empty_configuration_returns_empty_list() {
        let cfg = configuration_with(&[], None);
        let response = list_stores_response(&cfg);
        assert!(response.stores.is_empty());
    }

    #[test]
    fn list_stores_marks_fallback_store_as_default() {
        if skip_if_default_store_env_set() {
            return;
        }
        let cfg = configuration_with(&["personal", "work"], None);
        let response = list_stores_response(&cfg);
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
    fn list_stores_marks_config_default_store_as_default() {
        let cfg = configuration_with(&["personal", "work"], Some("work"));
        let response = list_stores_response(&cfg);
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

    // --- process_message --------------------------------------------------

    #[test]
    fn process_message_emits_decode_error_for_malformed_json() {
        let cfg = configuration_with(&[], None);
        let (request_id, response) =
            process_message(b"not json at all", &cfg, &empty_context(), true);
        assert!(request_id.is_none());
        match response {
            ResponsePayload::Error(err) => {
                assert_eq!(err.error, ErrorKind::DecodeError);
                assert!(err.hint.is_some());
            }
            _ => panic!("expected DecodeError"),
        }
    }

    #[test]
    fn process_message_emits_decode_error_for_unknown_action() {
        // Serde's tag-based deserialization rejects unknown variants as a
        // parse error; the host surfaces this as DecodeError with the
        // serde message in `hint`, instead of synthesizing a distinct
        // `UnknownAction` kind. The UnknownAction variant is reserved for
        // a future two-step parse that needs to differentiate.
        let cfg = configuration_with(&[], None);
        let (_request_id, response) =
            process_message(br#"{"action":"create"}"#, &cfg, &empty_context(), true);
        match response {
            ResponsePayload::Error(err) => assert_eq!(err.error, ErrorKind::DecodeError),
            _ => panic!("expected DecodeError"),
        }
    }

    #[test]
    fn process_message_routes_list_stores_to_response() {
        let cfg = configuration_with(&["personal"], None);
        let (_request_id, response) =
            process_message(br#"{"action":"list_stores"}"#, &cfg, &empty_context(), true);
        match response {
            ResponsePayload::ListStores(payload) => {
                assert_eq!(payload.stores.len(), 1);
                assert_eq!(payload.stores[0].name, "personal");
            }
            _ => panic!("expected ListStores"),
        }
    }

    #[test]
    fn process_message_echoes_request_id_on_success() {
        let cfg = configuration_with(&["personal"], None);
        let (request_id, _response) = process_message(
            br#"{"request_id":42,"action":"list_stores"}"#,
            &cfg,
            &empty_context(),
            true,
        );
        assert_eq!(request_id, Some(42));
    }

    #[test]
    fn process_message_echoes_request_id_on_decode_error_when_recoverable() {
        // Malformed *body* but the request_id at the top level is still
        // a parseable number — a multiplexing client must be able to
        // match the error reply back to its request.
        let cfg = configuration_with(&[], None);
        let (request_id, response) = process_message(
            br#"{"request_id":7,"action":"create"}"#,
            &cfg,
            &empty_context(),
            true,
        );
        assert_eq!(request_id, Some(7));
        assert!(matches!(response, ResponsePayload::Error(_)));
    }

    #[test]
    fn process_message_returns_none_request_id_when_payload_is_not_json() {
        let cfg = configuration_with(&[], None);
        let (request_id, _response) =
            process_message(b"not json at all", &cfg, &empty_context(), true);
        assert!(request_id.is_none());
    }

    #[test]
    fn process_message_returns_none_request_id_when_field_is_non_numeric() {
        // A client that sends `"request_id": "abc"` cannot be correlated
        // — we don't invent an id and we don't echo a non-number.
        let cfg = configuration_with(&[], None);
        let (request_id, _response) = process_message(
            br#"{"request_id":"abc","action":"create"}"#,
            &cfg,
            &empty_context(),
            true,
        );
        assert!(request_id.is_none());
    }

    #[test]
    fn protocol_version_in_list_stores_is_one() {
        let cfg = configuration_with(&["personal"], None);
        let response = list_stores_response(&cfg);
        assert_eq!(response.protocol_version, PROTOCOL_VERSION);
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}
