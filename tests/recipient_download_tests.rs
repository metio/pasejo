// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Hermetic coverage for `recipient add --codeberg/--github/--gitlab`.
//!
//! The `*_HOST` overrides accept a scheme, so these tests point the
//! downloader at a throwaway in-process HTTP server serving fixture
//! responses instead of the real providers. That keeps the suite offline,
//! immune to provider downtime and key rotation, and decoupled from any
//! maintainer's personal accounts — and it lets the multi-key, zero-key,
//! provider-error, and timeout cases be asserted directly.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use assert_fs::TempDir;
use predicates::prelude::*;

mod common;
use common::pasejo;

// Real, well-formed `ssh-ed25519` public keys. Their provenance does not
// matter — they only have to parse as age SSH recipients.
const KEY_A: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOycTzXsl1jetsf+Ezi/2FCVU8oydXxxJhb9D7n6MlGf";
const KEY_B: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJgE1240mCyWQRPB6hcGuVqX6wVtrJJZCGh1KSAaODmB";

// An `ecdsa-sha2-nistp256` key, a type age parses but cannot encrypt to.
const KEY_UNSUPPORTED: &str = "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBHFliOyIZs1gxGF3fmDxFykQhE88wy6AKDGFBfn0R6ZuvRmENABZQa9+pj9hMki+LX0qDJbmHTiWDbYv/cmFt/Q=";

/// How the fixture server answers a matched route.
enum Reply {
    /// `200 OK` with this body — a `.keys` payload.
    Body(String),
    /// A non-2xx status line (code + reason) with an empty body, modelling
    /// a provider returning `500`, `429 Too Many Requests`, `404`, etc.
    Status(u16, &'static str),
    /// Accept the connection, read the request, then never respond —
    /// modelling an unresponsive provider so the client-side global
    /// timeout is what ends the call.
    Hang,
}

/// Spawn a single-threaded HTTP/1.1 server bound to an ephemeral loopback
/// port. Each `GET /<path>` is answered from `routes`, or `404` when the
/// path is absent. Returns the `http://127.0.0.1:<port>` base; the detached
/// server thread is reaped when the test process exits.
fn spawn_fixture_server(routes: HashMap<String, Reply>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            handle_connection(stream, &routes);
        }
    });

    base
}

/// Convenience for the common single-route server.
fn server_with(path: &str, reply: Reply) -> String {
    spawn_fixture_server(HashMap::from([(String::from(path), reply)]))
}

fn handle_connection(mut stream: TcpStream, routes: &HashMap<String, Reply>) {
    // Read until the end of the request headers. A `.keys` GET carries no
    // body, so the blank line that terminates the headers is the whole
    // request as far as this fixture is concerned.
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 512];
    loop {
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    let request = String::from_utf8_lossy(&buffer);
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let response = match routes.get(path) {
        Some(Reply::Body(body)) => format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ),
        Some(Reply::Status(code, reason)) => {
            format!("HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        }
        Some(Reply::Hang) => {
            // Hold the connection open and send nothing; the client's global
            // timeout terminates the call. The leftover sleep does not gate
            // the test, which completes as soon as the client gives up.
            thread::sleep(Duration::from_secs(30));
            return;
        }
        None => {
            String::from("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        }
    };
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Create a registered (but empty) store in a fresh config and return the
/// temp dir plus the config path, ready for a `recipient add` invocation.
fn setup_store() -> (TempDir, std::path::PathBuf) {
    let common::StoreScaffold { temp, config } = common::StoreScaffold::new().store("something");
    (temp, config)
}

/// Pin the per-request download timeout in the config so the timeout case
/// fails in about a second instead of the 30s default.
fn set_download_timeout(config: &std::path::Path, seconds: u32) {
    pasejo(config)
        .args(["config", "set", "key-download-timeout-seconds"])
        .arg(seconds.to_string())
        .assert()
        .success();
}

/// Run `recipient add --<flag> tester` against `base` and return the
/// configured assert handle for the caller to check.
fn add_recipient(
    config: &std::path::Path,
    flag: &str,
    host_env: &str,
    base: &str,
) -> assert_cmd::assert::Assert {
    pasejo(config)
        .args(["recipient", "add", flag, "tester"])
        .env(host_env, base)
        .assert()
}

#[test]
fn recipient_add_from_github_fixture_adds_the_downloaded_key() {
    let base = server_with("/tester.keys", Reply::Body(String::from(KEY_A)));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .success()
        .stderr(predicates::str::contains(format!(
            "Recipient for '{KEY_A}' added"
        )));
}

#[test]
fn recipient_add_from_codeberg_fixture_adds_the_downloaded_key() {
    let base = server_with("/tester.keys", Reply::Body(String::from(KEY_A)));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--codeberg", "CODEBERG_HOST", &base)
        .success()
        .stderr(predicates::str::contains(format!(
            "Recipient for '{KEY_A}' added"
        )));
}

#[test]
fn recipient_add_from_gitlab_fixture_adds_the_downloaded_key() {
    let base = server_with("/tester.keys", Reply::Body(String::from(KEY_A)));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--gitlab", "GITLAB_HOST", &base)
        .success()
        .stderr(predicates::str::contains(format!(
            "Recipient for '{KEY_A}' added"
        )));
}

#[test]
fn recipient_add_from_account_with_multiple_keys_adds_every_key() {
    // R1 regression: an account whose `.keys` body lists several keys must
    // register all of them, not just the first.
    let base = server_with("/tester.keys", Reply::Body(format!("{KEY_A}\n{KEY_B}\n")));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .success()
        .stderr(
            predicates::str::contains(format!("Recipient for '{KEY_A}' added")).and(
                predicates::str::contains(format!("Recipient for '{KEY_B}' added")),
            ),
        );
}

#[test]
fn recipient_add_skips_downloaded_keys_age_cannot_encrypt_to() {
    let base = server_with(
        "/tester.keys",
        Reply::Body(format!("{KEY_UNSUPPORTED}\n{KEY_A}\n")),
    );
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .success()
        .stderr(
            predicates::str::contains(format!("Recipient for '{KEY_A}' added"))
                .and(predicates::str::contains("ecdsa-sha2-nistp256"))
                .and(
                    predicates::str::contains(format!("Recipient for '{KEY_UNSUPPORTED}' added"))
                        .not(),
                ),
        );
}

#[test]
fn recipient_add_fails_when_every_downloaded_key_is_unusable() {
    let base = server_with("/tester.keys", Reply::Body(format!("{KEY_UNSUPPORTED}\n")));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .failure()
        .stderr(predicates::str::contains(
            "github user 'tester' has no public SSH keys to add as recipients",
        ));
}

#[test]
fn recipient_add_from_account_with_no_keys_fails_with_a_clear_error() {
    // R4 regression: an empty `.keys` body must be a clear "no public keys"
    // error, not a downstream `Invalid recipient ''`.
    let base = server_with("/tester.keys", Reply::Body(String::new()));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .failure()
        .stderr(predicates::str::contains("tester"));
}

#[test]
fn recipient_add_when_provider_returns_500_reports_a_server_error() {
    // A 5xx is an upstream fault — distinct from a missing user — and is
    // reported as such with the status code, no recipient written.
    let base = server_with("/tester.keys", Reply::Status(500, "Internal Server Error"));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .failure()
        .stderr(predicates::str::contains(
            "github returned an error (HTTP 500)",
        ));
}

#[test]
fn recipient_add_when_provider_rate_limits_with_429_says_rate_limited() {
    // A `429 Too Many Requests` gets its own retry-suggesting message.
    let base = server_with("/tester.keys", Reply::Status(429, "Too Many Requests"));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--gitlab", "GITLAB_HOST", &base)
        .failure()
        .stderr(predicates::str::contains("gitlab rate-limited the request"));
}

#[test]
fn recipient_add_for_unknown_user_404_says_user_not_found() {
    // The providers answer a non-existent account with `404`. Without a
    // matching route the fixture returns 404 too, exercising the same path;
    // the user gets a "no such user" message naming the username they typed.
    let base = server_with("/other.keys", Reply::Body(String::from(KEY_A)));
    let (_temp, config) = setup_store();

    add_recipient(&config, "--codeberg", "CODEBERG_HOST", &base)
        .failure()
        .stderr(predicates::str::contains("codeberg has no user 'tester'"));
}

#[test]
fn recipient_add_when_provider_is_unresponsive_reports_a_timeout() {
    // An unresponsive provider must not hang the CLI: the global timeout
    // (pinned to 1s here) ends the call with a timeout-specific message and
    // a non-zero exit.
    let base = server_with("/tester.keys", Reply::Hang);
    let (_temp, config) = setup_store();
    set_download_timeout(&config, 1);

    add_recipient(&config, "--github", "GITHUB_HOST", &base)
        .failure()
        .stderr(predicates::str::contains(
            "Downloading public key from github timed out",
        ));
}

#[test]
fn recipient_add_404_message_is_localized() {
    // Translation gate: the provider-error messages fire from a network
    // path the trycmd language suites cannot drive, so one non-English
    // assertion here proves the `.ftl` wiring and argument substitution
    // resolve for a translated locale rather than falling back to English.
    let base = server_with("/other.keys", Reply::Body(String::from(KEY_A)));
    let (_temp, config) = setup_store();

    // Override the helper's pinned `en` locale to drive the German bundle.
    pasejo(&config)
        .args(["recipient", "add", "--codeberg", "tester"])
        .env("CODEBERG_HOST", &base)
        .env("LANG", "de")
        .env("LC_ALL", "de")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "codeberg kennt keinen Benutzer 'tester'",
        ));
}

#[test]
fn recipient_add_unsupported_key_warning_is_localized() {
    let base = server_with(
        "/tester.keys",
        Reply::Body(format!("{KEY_UNSUPPORTED}\n{KEY_A}\n")),
    );
    let (_temp, config) = setup_store();

    pasejo(&config)
        .args(["recipient", "add", "--github", "tester"])
        .env("GITHUB_HOST", &base)
        .env("LANG", "de")
        .env("LC_ALL", "de")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "github-Schlüssel vom Typ 'ecdsa-sha2-nistp256' für Benutzer 'tester' wird übersprungen",
        ));
}
