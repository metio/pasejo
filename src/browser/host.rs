// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{IsTerminal, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use anyhow::{Result, bail};

use crate::browser::matcher::find_matches;
use crate::browser::protocol::{
    ErrorKind, ErrorResponse, ListStoresResponse, PROTOCOL_VERSION, QueryResponse, Request,
    RequestEnvelope, ResponseEnvelope, ResponsePayload, StoreEntry,
};
use crate::cli::i18n;
use crate::commands::store_op::try_acquire_store_lock;
use crate::hooks::executor::HookExecutor;
use crate::models::cli::HostArgs;
use crate::models::configuration::{Configuration, StoreRegistration, decrypt_store_with};
use crate::models::password_store::PasswordStore;

// Chrome's documented native-messaging maximum is 1 MiB per direction.
// Rejecting larger length prefixes prevents a malformed peer from driving
// the host into multi-gigabyte allocations.
pub const MAX_MESSAGE_LEN: usize = 1024 * 1024;

// Bounded worker pool. The workload is I/O-bound (file I/O, ssh-agent
// stdio, age plugin stdio, sometimes a YubiKey touch), not CPU-bound,
// so the pool size doesn't track `available_parallelism()`. 16 covers
// the realistic upper bound of concurrent stores a real user has
// configured (typical: 1–10, aggressive deployments: ~20) with
// headroom. Adding more workers wouldn't help — they'd all be waiting
// on the same single physical YubiKey or ssh-agent socket.
const HOST_WORKER_THREADS: usize = 16;

// Bounded request queue. Without it, a misbehaving caller pipelining
// thousands of requests could swap "thousands of OS threads" for
// "thousands of queued request payloads" — each up to MAX_MESSAGE_LEN
// (1 MiB), so an unbounded queue would cap at gigabytes of pending
// bytes. With a depth of 32 (≈2× the pool) the stdin read loop
// back-pressures cleanly when the queue is full: `send` blocks until
// a worker drains a slot, and the caller has to slow down rather
// than the host having to drop work.
const HOST_WORKER_QUEUE_DEPTH: usize = 32;

// `secret edit` holds the store lock for a whole editor session. A query
// that cannot get the lock within this window answers `store_unreachable`
// instead of pinning a worker until the editor closes and running into the
// extension's own request timeout.
const STORE_LOCK_WAIT: Duration = Duration::from_secs(5);

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
        Err((kind, err)) => return error_response(kind, configuration, registration, &err),
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
/// needs). Pull hooks fire (throttled), the file lock is acquired, the
/// identities are read and the store decrypted, the lock is released, and
/// the decrypted store is returned. Each step's failure is classified into a
/// distinct `ErrorKind` so the extension can surface the right fix — the funnel
/// itself returns `anyhow::Error`, which would collapse those into a single
/// chain and leave the user guessing.
///
/// `try_acquire_store_lock` returning `Ok(None)` (the bootstrap path — a
/// store registered in the config but with no file on disk yet) is
/// mapped to `StoreUnreachable`; the browser host is never the right
/// entry point for bootstrapping.
fn open_store_for_read(
    configuration: &Configuration,
    registration: &StoreRegistration,
    offline: bool,
) -> std::result::Result<PasswordStore, (ErrorKind, anyhow::Error)> {
    let hooks = HookExecutor::new(configuration, registration, offline);
    hooks
        .execute_pull_commands()
        .map_err(|err| (ErrorKind::HookFailed, err))?;

    let lock = match try_acquire_store_lock(registration.path(), STORE_LOCK_WAIT) {
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

    let identities = configuration
        .read_identities(registration)
        .map_err(|err| (ErrorKind::IdentityLocked, err))?;
    let store = decrypt_store_with(registration.path(), &identities)
        .map_err(|err| (decrypt_error_kind(&err), err))?;
    drop(lock);
    Ok(store)
}

/// Decryption fails on the identity side when none of the identities
/// unwraps the file key; every other failure (an unreadable file, a broken
/// age header or payload, a plaintext that is not a store) lies with the
/// store file.
fn decrypt_error_kind(err: &anyhow::Error) -> ErrorKind {
    match err.downcast_ref::<age::DecryptError>() {
        Some(
            age::DecryptError::NoMatchingKeys
            | age::DecryptError::KeyDecryptionFailed
            | age::DecryptError::Plugin(_)
            | age::DecryptError::PluginResolve(_),
        ) => ErrorKind::IdentityLocked,
        _ => ErrorKind::StoreUnreachable,
    }
}

/// Placeholder substituted for the store file's absolute path in the
/// outgoing error `hint`. Chosen to be cross-platform (no leading `~/`
/// since that only renders meaningfully on Unix-shaped shells) and
/// distinctive enough that a reader scanning a screenshot can see
/// what was removed.
const STORE_PATH_PLACEHOLDER: &str = "<store>";

/// Placeholder substituted for every configured age / SSH identity
/// path. Same redaction story as the store path: the local user
/// already knows where their identity files live; the popup's status
/// line should not be the surface where those paths end up on a
/// shared screen or a support-channel screenshot.
const IDENTITY_PATH_PLACEHOLDER: &str = "<identity>";

/// Builds an error payload from an anyhow chain after redacting the
/// store file path and every identity path the configuration knows
/// about. The `Path::display()` form is the canonical surface — that
/// is what every anyhow context in this module formats with — so the
/// redaction is exact-string-substitution against that form rather
/// than an attempt to normalise canonicalisation across the chain.
/// Paths that leak through a different formatter (e.g. `{:?}` which
/// adds quotes) keep their original shape; this is an acceptable
/// limitation for a defence-in-depth measure rather than a security
/// boundary.
fn error_response(
    kind: ErrorKind,
    configuration: &Configuration,
    registration: &StoreRegistration,
    err: &anyhow::Error,
) -> ResponsePayload {
    let identity_files = configuration.all_identity_files(registration);
    let mut pairs: Vec<(&Path, &str)> = Vec::with_capacity(identity_files.len().saturating_add(1));
    pairs.push((registration.path(), STORE_PATH_PLACEHOLDER));
    for file in &identity_files {
        pairs.push((file.as_path(), IDENTITY_PATH_PLACEHOLDER));
    }
    let raw = format!("{err:#}");
    let hint = redact_sensitive_paths(&raw, &pairs);
    ResponsePayload::Error(ErrorResponse {
        error: kind,
        store: Some(registration.name.clone()),
        hint: Some(hint),
    })
}

/// Substitutes the `display()` form of every supplied path in `input`
/// with the matching placeholder, longest-first so an identity path
/// nested inside a store path's directory can't fragment the longer
/// match before it gets a chance to fire. Empty paths are dropped
/// silently — a `StoreRegistration::default()` value carries an empty
/// `PathBuf`, and an empty needle would match between every byte of
/// the input.
fn redact_sensitive_paths(input: &str, pairs: &[(&Path, &str)]) -> String {
    let mut needles: Vec<(String, &str)> = pairs
        .iter()
        .map(|(path, placeholder)| (path.display().to_string(), *placeholder))
        .filter(|(needle, _)| !needle.is_empty())
        .collect();
    // Descending length so a path nested inside another (e.g. an
    // identity file living inside the store's directory) replaces the
    // outer match first; reversed comparison via `Reverse` keeps
    // clippy happy without flipping operands.
    needles.sort_by_key(|(needle, _)| std::cmp::Reverse(needle.len()));
    let mut out = input.to_owned();
    for (needle, placeholder) in &needles {
        if out.contains(needle.as_str()) {
            out = out.replace(needle.as_str(), placeholder);
        }
    }
    out
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

    // Fixed-size worker pool: the main thread reads one request at a
    // time and hands the raw bytes off through a bounded channel; the
    // workers race to take the next slot. The previous per-request
    // `thread::spawn` had no cap, so a misbehaving caller pipelining
    // thousands of requests could exhaust the process's thread budget.
    // With this pool the worst case is HOST_WORKER_THREADS concurrent
    // threads regardless of how fast the caller writes, and the
    // bounded queue back-pressures the stdin reader when full instead
    // of buffering unbounded request bytes in memory.
    //
    // Workers run detached: when stdin closes, the `while let`
    // returns, we drop the sender, and idle workers exit when
    // `recv()` returns `Err`. Workers still inside `decrypt_store`
    // (e.g. blocked on a YubiKey touch) keep running until the
    // process itself exits, at which point the OS reaps them. Every
    // step is read-only on the store file — OS releases the lock on
    // fd close, hooks are restartable, age plugins exit on stdin
    // EOF — so this is safe.
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(HOST_WORKER_QUEUE_DEPTH);
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..HOST_WORKER_THREADS {
        let configuration = Arc::clone(&configuration);
        let context = Arc::clone(&context);
        let stdout = Arc::clone(&stdout);
        let rx = Arc::clone(&rx);
        thread::spawn(move || worker_loop(&rx, &configuration, &context, offline, &stdout));
    }

    while let Some(bytes) = read_message(&mut input)? {
        // A full queue blocks here until a worker drains a slot —
        // that's the load-bearing back-pressure: the caller can't
        // outrun us into memory exhaustion. A `send` failure means
        // every worker has dropped its receiver clone, which only
        // happens after `rx` itself is dropped; that can't happen
        // while we still hold a clone in the loop above, so this
        // branch is genuinely unreachable in normal operation. The
        // `if` keeps it from looking like an unwrap.
        if tx.send(bytes).is_err() {
            break;
        }
    }
    drop(tx);
    Ok(())
}

// Worker body: pulls one request at a time off the shared receiver.
// The Mutex around `Receiver` is held only across the `recv()` call
// itself — once a message lands the guard drops before the worker
// runs `handle_one_message`, so two workers process requests
// concurrently rather than serially. When the sender drops and the
// queue empties, `recv()` returns `Err` and the worker exits.
fn worker_loop(
    rx: &Mutex<mpsc::Receiver<Vec<u8>>>,
    configuration: &Configuration,
    context: &HostContext,
    offline: bool,
    stdout: &Mutex<std::io::Stdout>,
) {
    loop {
        let bytes = {
            let Ok(guard) = rx.lock() else {
                return;
            };
            match guard.recv() {
                Ok(bytes) => bytes,
                Err(_) => return,
            }
        };
        handle_one_message(&bytes, configuration, context, offline, stdout);
    }
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
    let Some(response_bytes) = serialize_capped(request_id, &payload) else {
        return;
    };
    let mut out = match stdout.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Err(err) = write_message(&mut *out, &response_bytes) {
        eprintln!("pasejo browser host: failed to write response: {err}");
    }
}

/// Serializes the response envelope and enforces the browser's
/// host→browser message limit on the OUTBOUND side. Chrome kills the
/// native-messaging port when the host writes a message larger than
/// 1 MiB — which would fail every in-flight request on the
/// connection, not just the oversized one. A reply that exceeds
/// `MAX_MESSAGE_LEN` (a store whose matches carry very large secret
/// bodies, for instance) is therefore downgraded to a small
/// per-request error envelope that keeps the connection alive and
/// tells the extension which request could not be answered. The
/// replacement envelope is a fixed shape whose size does not depend
/// on any user data, so it cannot itself exceed the limit.
fn serialize_capped(request_id: Option<u64>, payload: &ResponsePayload) -> Option<Vec<u8>> {
    let envelope = ResponseEnvelope {
        request_id,
        payload,
    };
    let response_bytes = match serde_json::to_vec(&envelope) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("pasejo browser host: failed to serialize response: {err}");
            return None;
        }
    };
    if response_bytes.len() <= MAX_MESSAGE_LEN {
        return Some(response_bytes);
    }
    let too_large = ResponsePayload::Error(ErrorResponse {
        error: ErrorKind::ResponseTooLarge,
        store: None,
        hint: Some(format!(
            "response of {} bytes exceeds the {MAX_MESSAGE_LEN}-byte native-messaging limit",
            response_bytes.len()
        )),
    });
    let fallback = ResponseEnvelope {
        request_id,
        payload: &too_large,
    };
    match serde_json::to_vec(&fallback) {
        Ok(bytes) => Some(bytes),
        Err(err) => {
            eprintln!("pasejo browser host: failed to serialize fallback response: {err}");
            None
        }
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

    // --- serialize_capped (review8 item D) --------------------------------

    #[test]
    fn serialize_capped_passes_a_normal_reply_through_unchanged() {
        let payload = ResponsePayload::Error(ErrorResponse {
            error: ErrorKind::UnknownStore,
            store: Some("personal".into()),
            hint: None,
        });
        let bytes = serialize_capped(Some(7), &payload).expect("must serialize");
        let text = String::from_utf8(bytes).expect("valid utf8");
        assert!(text.contains(r#""request_id":7"#));
        assert!(text.contains(r#""error":"unknown_store""#));
    }

    #[test]
    fn serialize_capped_downgrades_an_oversized_reply_to_response_too_large() {
        // A hint just past the limit pushes the serialized envelope over
        // MAX_MESSAGE_LEN. Writing it would get the connection killed by
        // the browser; the cap converts it into a small per-request
        // error that preserves the request_id correlation.
        let payload = ResponsePayload::Error(ErrorResponse {
            error: ErrorKind::HookFailed,
            store: Some("personal".into()),
            hint: Some("x".repeat(MAX_MESSAGE_LEN + 1)),
        });
        let bytes = serialize_capped(Some(42), &payload).expect("fallback must serialize");
        assert!(
            bytes.len() <= MAX_MESSAGE_LEN,
            "fallback must fit the limit, got {} bytes",
            bytes.len()
        );
        let text = String::from_utf8(bytes).expect("valid utf8");
        assert!(text.contains(r#""error":"response_too_large""#));
        assert!(
            text.contains(r#""request_id":42"#),
            "the oversized reply's request_id must survive the downgrade so the extension can route the error"
        );
        assert!(text.contains("native-messaging limit"));
    }

    // --- redact_sensitive_paths / error_response (item F) ----------------

    use std::path::PathBuf;

    use crate::models::configuration::Identity;

    #[test]
    fn redact_sensitive_paths_returns_input_unchanged_when_no_path_appears() {
        let store = PathBuf::from("/home/alice/.local/share/pasejo/personal.age");
        let pairs: Vec<(&Path, &str)> = vec![(store.as_path(), STORE_PATH_PLACEHOLDER)];
        let input = "decryption failed: identity does not match recipients";
        assert_eq!(redact_sensitive_paths(input, &pairs), input);
    }

    #[test]
    fn redact_sensitive_paths_replaces_the_store_path() {
        let store = PathBuf::from("/home/alice/.local/share/pasejo/personal.age");
        let pairs: Vec<(&Path, &str)> = vec![(store.as_path(), STORE_PATH_PLACEHOLDER)];
        let input = format!("store file does not exist at {}", store.display());
        let redacted = redact_sensitive_paths(&input, &pairs);
        assert_eq!(redacted, "store file does not exist at <store>");
    }

    #[test]
    fn redact_sensitive_paths_replaces_every_occurrence_of_the_same_path() {
        let store = PathBuf::from("/data/personal.age");
        let pairs: Vec<(&Path, &str)> = vec![(store.as_path(), STORE_PATH_PLACEHOLDER)];
        let input = format!(
            "failed to open {} (path: {}, retrying via {} once)",
            store.display(),
            store.display(),
            store.display(),
        );
        let redacted = redact_sensitive_paths(&input, &pairs);
        // All three occurrences are replaced — a single hint can mention
        // the same path multiple times when the anyhow chain repeats the
        // context across layers.
        assert_eq!(
            redacted,
            "failed to open <store> (path: <store>, retrying via <store> once)"
        );
    }

    #[test]
    fn redact_sensitive_paths_replaces_identity_paths_with_a_distinct_placeholder() {
        let identity_a = PathBuf::from("/home/alice/.config/pasejo/identities/key.txt");
        let identity_b = PathBuf::from("/home/alice/.ssh/id_ed25519");
        let pairs: Vec<(&Path, &str)> = vec![
            (identity_a.as_path(), IDENTITY_PATH_PLACEHOLDER),
            (identity_b.as_path(), IDENTITY_PATH_PLACEHOLDER),
        ];
        let input = format!(
            "could not decrypt with {}; also tried {}",
            identity_a.display(),
            identity_b.display()
        );
        let redacted = redact_sensitive_paths(&input, &pairs);
        assert_eq!(
            redacted,
            "could not decrypt with <identity>; also tried <identity>"
        );
    }

    #[test]
    fn redact_sensitive_paths_applies_longest_first_so_a_nested_match_does_not_fragment_the_outer()
    {
        // Identity path lives inside the store path's directory. Without
        // longest-first ordering, replacing the identity path first
        // would mangle the surrounding string and the store match would
        // never fire on the now-fragmented input.
        let outer = PathBuf::from("/data/stores/personal/store.age");
        let inner = PathBuf::from("/data/stores/personal");
        let pairs: Vec<(&Path, &str)> = vec![
            (inner.as_path(), IDENTITY_PATH_PLACEHOLDER),
            (outer.as_path(), STORE_PATH_PLACEHOLDER),
        ];
        let input = format!("store {} inside dir {}", outer.display(), inner.display());
        let redacted = redact_sensitive_paths(&input, &pairs);
        assert_eq!(redacted, "store <store> inside dir <identity>");
    }

    #[test]
    fn redact_sensitive_paths_drops_empty_paths_so_default_registrations_do_not_carpet_bomb() {
        // `StoreRegistration::default()` has an empty `PathBuf`. The
        // replacement loop must not run with an empty needle — that
        // would match between every UTF-8 boundary and turn the input
        // into a sea of placeholders. The helper filters empties out.
        let empty = PathBuf::new();
        let pairs: Vec<(&Path, &str)> = vec![(empty.as_path(), STORE_PATH_PLACEHOLDER)];
        let input = "anyhow chain text";
        assert_eq!(redact_sensitive_paths(input, &pairs), input);
    }

    #[test]
    fn error_response_redacts_the_store_path_from_the_outgoing_hint() {
        let store_path = PathBuf::from("/data/personal.age");
        let mut reg = registration("personal");
        reg.path = store_path.clone();
        reg.identities = vec![Identity {
            file: PathBuf::from("/keys/personal.key"),
        }];
        let cfg = Configuration {
            stores: vec![reg.clone()],
            ..Configuration::default()
        };
        let err = anyhow::anyhow!("store file does not exist at {}", store_path.display());
        let payload = error_response(ErrorKind::StoreUnreachable, &cfg, &reg, &err);
        let ResponsePayload::Error(response) = payload else {
            panic!("expected error payload");
        };
        let hint = response.hint.expect("hint is always populated");
        assert!(
            !hint.contains("/data/personal.age"),
            "raw store path leaked: {hint}"
        );
        assert!(
            hint.contains("<store>"),
            "store placeholder missing: {hint}"
        );
        // The structured `store` field still carries the user-chosen
        // name — names are not filesystem leaks and the popup needs
        // them for routing.
        assert_eq!(response.store.as_deref(), Some("personal"));
    }

    #[test]
    fn error_response_redacts_identity_paths_combined_from_global_and_per_store_lists() {
        let identity_global = PathBuf::from("/keys/shared.key");
        let identity_local = PathBuf::from("/keys/personal.key");
        let mut reg = registration("personal");
        reg.path = PathBuf::from("/data/personal.age");
        reg.identities = vec![Identity {
            file: identity_local.clone(),
        }];
        let cfg = Configuration {
            stores: vec![reg.clone()],
            identities: vec![Identity {
                file: identity_global.clone(),
            }],
            ..Configuration::default()
        };
        let err = anyhow::anyhow!(
            "could not decrypt with {}; also tried {}",
            identity_global.display(),
            identity_local.display()
        );
        let payload = error_response(ErrorKind::IdentityLocked, &cfg, &reg, &err);
        let ResponsePayload::Error(response) = payload else {
            panic!("expected error payload");
        };
        let hint = response.hint.expect("hint is always populated");
        assert!(
            !hint.contains("/keys/shared.key"),
            "global identity leaked: {hint}"
        );
        assert!(
            !hint.contains("/keys/personal.key"),
            "per-store identity leaked: {hint}"
        );
        assert_eq!(hint.matches("<identity>").count(), 2);
    }

    #[test]
    fn open_store_for_read_reports_store_unreachable_while_the_store_lock_is_held() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let store_path = temp.path().join("personal.age");
        std::fs::write(&store_path, b"locked").unwrap();
        let held = std::fs::File::open(&store_path).unwrap();
        held.lock().unwrap();

        let mut reg = registration("personal");
        reg.path = store_path;
        let cfg = Configuration {
            stores: vec![reg.clone()],
            ..Configuration::default()
        };
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let kind = open_store_for_read(&cfg, &reg, true)
                .err()
                .map(|(kind, _)| kind);
            let _ = tx.send(kind);
        });

        let outcome = rx.recv_timeout(STORE_LOCK_WAIT * 2);
        drop(held);
        assert_eq!(outcome, Ok(Some(ErrorKind::StoreUnreachable)));
    }

    fn configuration_with_identity(
        temp: &assert_fs::TempDir,
        store_path: PathBuf,
    ) -> Configuration {
        let identity = age::x25519::Identity::generate();
        let identity_file = temp.path().join("identity");
        std::fs::write(
            &identity_file,
            age::secrecy::ExposeSecret::expose_secret(&identity.to_string()),
        )
        .unwrap();
        let mut reg = registration("personal");
        reg.path = store_path;
        Configuration {
            stores: vec![reg],
            identities: vec![Identity {
                file: identity_file,
            }],
            ..Configuration::default()
        }
    }

    fn open_store_for_read_error_kind(cfg: &Configuration) -> Option<ErrorKind> {
        open_store_for_read(cfg, &cfg.stores[0], true)
            .err()
            .map(|(kind, _)| kind)
    }

    #[test]
    fn open_store_for_read_reports_an_empty_store_file_as_store_unreachable() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let store_path = temp.path().join("personal.age");
        std::fs::write(&store_path, b"").unwrap();
        let cfg = configuration_with_identity(&temp, store_path);
        assert_eq!(
            open_store_for_read_error_kind(&cfg),
            Some(ErrorKind::StoreUnreachable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn open_store_for_read_reports_a_directory_at_the_store_path_as_store_unreachable() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let store_path = temp.path().join("personal.age");
        std::fs::create_dir(&store_path).unwrap();
        let cfg = configuration_with_identity(&temp, store_path);
        assert_eq!(
            open_store_for_read_error_kind(&cfg),
            Some(ErrorKind::StoreUnreachable)
        );
    }

    #[test]
    fn open_store_for_read_reports_a_store_for_other_recipients_as_identity_locked() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let store_path = temp.path().join("personal.age");
        let stranger = age::x25519::Identity::generate().to_public();
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(stranger)];
        crate::secrets::encrypt("", &store_path, &recipients).unwrap();
        let cfg = configuration_with_identity(&temp, store_path);
        assert_eq!(
            open_store_for_read_error_kind(&cfg),
            Some(ErrorKind::IdentityLocked)
        );
    }

    #[test]
    fn open_store_for_read_reports_a_store_that_is_not_toml_as_store_unreachable() {
        crate::cli::i18n::init_for_tests();
        let temp = assert_fs::TempDir::new().unwrap();
        let store_path = temp.path().join("personal.age");
        let cfg = configuration_with_identity(&temp, store_path.clone());
        let identity_text = std::fs::read_to_string(&cfg.identities[0].file).unwrap();
        let identity: age::x25519::Identity = identity_text.trim().parse().unwrap();
        let recipients: Vec<Box<dyn age::Recipient + Send>> = vec![Box::new(identity.to_public())];
        crate::secrets::encrypt("not = [toml", &store_path, &recipients).unwrap();
        assert_eq!(
            open_store_for_read_error_kind(&cfg),
            Some(ErrorKind::StoreUnreachable)
        );
    }

    #[test]
    fn error_response_leaves_non_path_diagnostic_text_intact() {
        // The redaction is targeted at filesystem paths — error category
        // text, hook stderr, and similar diagnostic content must come
        // through unchanged so the user can act on it.
        let mut reg = registration("personal");
        reg.path = PathBuf::from("/data/personal.age");
        let cfg = Configuration {
            stores: vec![reg.clone()],
            ..Configuration::default()
        };
        let err = anyhow::anyhow!("pull hook returned exit code 42");
        let payload = error_response(ErrorKind::HookFailed, &cfg, &reg, &err);
        let ResponsePayload::Error(response) = payload else {
            panic!("expected error payload");
        };
        let hint = response.hint.expect("hint is always populated");
        assert_eq!(hint, "pull hook returned exit code 42");
    }
}
