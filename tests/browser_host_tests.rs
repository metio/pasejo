// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{Read, Write};
use std::process::{Command as StdCommand, Stdio};

use assert_cmd::cargo::cargo_bin;
use assert_fs::TempDir;

mod common;
use common::{TEST_PRIVATE_KEY, TEST_PUBLIC_KEY, pasejo};

fn setup_store_with_secret(secret_name: &str, secret_value: &str) -> TempDir {
    common::StoreScaffold::new()
        .store("test")
        .identity()
        .recipient()
        .secret(secret_name, secret_value)
        .temp
}

fn write_request(stdin: &mut impl Write, payload: &[u8]) {
    let len = u32::try_from(payload.len()).unwrap();
    stdin.write_all(&len.to_ne_bytes()).unwrap();
    stdin.write_all(payload).unwrap();
    stdin.flush().unwrap();
}

fn read_response(stdout: &mut impl Read) -> String {
    let mut len_buf = [0u8; 4];
    stdout.read_exact(&mut len_buf).unwrap();
    let len = u32::from_ne_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stdout.read_exact(&mut buf).unwrap();
    String::from_utf8(buf).unwrap()
}

#[test]
fn host_answers_query_with_matching_secret() {
    let temp = setup_store_with_secret(
        "alice",
        "hunter2\nurl: https://example.com\nlogin: alice@example.com\n",
    );
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let response = read_response(&mut stdout);

    assert!(
        response.contains(r#""name":"alice""#),
        "response was: {response}"
    );
    assert!(response.contains(r#""store":"test""#));
    assert!(response.contains(r#""url":"https://example.com""#));
    assert!(response.contains(r#""username":"alice@example.com""#));
    assert!(response.contains(r#""match_kind":"url_field""#));
    assert!(response.contains(r#""score":1.0"#));
    assert!(
        !response.contains("hunter2"),
        "a query must not carry the password; response was: {response}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success(), "host exited with {status:?}");
}

#[test]
fn host_emits_decode_error_for_malformed_request() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, b"not a json document");
    let response = read_response(&mut stdout);

    assert!(
        response.contains(r#""error":"decode_error""#),
        "response was: {response}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn host_lists_stores_with_default_marker() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"list_stores"}"#);
    let response = read_response(&mut stdout);

    assert!(
        response.contains(r#""name":"test""#),
        "response was: {response}"
    );
    assert!(response.contains(r#""default":true"#));
    assert!(
        response.contains(r#""protocol_version":2"#),
        "list_stores must include protocol_version so the extension can refuse a mismatched host; response was: {response}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn host_reports_store_unreachable_when_store_file_missing() {
    // Register a store and identity but delete the store file before
    // querying. A registered store without a file must surface as
    // `store_unreachable`, not as the
    // catch-all `identity_locked` — the actionable fix is filesystem-side,
    // not identity-side.
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");
    let store_path = temp.path().join("store");
    std::fs::remove_file(&store_path).unwrap();

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let response = read_response(&mut stdout);

    assert!(
        response.contains(r#""error":"store_unreachable""#),
        "response was: {response}"
    );
    assert!(response.contains(r#""store":"test""#));

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn host_echoes_request_id_on_a_query() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"request_id":42,"action":"query","domain":"example.com"}"#,
    );
    let response = read_response(&mut stdout);
    assert!(
        response.contains(r#""request_id":42"#),
        "host must echo request_id; response was: {response}"
    );
    assert!(response.contains(r#""name":"alice""#));

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success(), "host exited with {status:?}");
}

#[test]
fn host_omits_request_id_when_caller_does_not_send_one() {
    // Defensive contract: the host echoes `request_id` only when it
    // received one; a malformed (or in tests, deliberately incomplete)
    // request must not pick up a synthesised id that the caller could
    // not correlate. Locked in so a future change can't quietly start
    // emitting a `request_id: null` for the no-id path.
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"list_stores"}"#);
    let response = read_response(&mut stdout);
    assert!(
        !response.contains("request_id"),
        "v1 caller must not see a request_id field in the response; got: {response}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn host_drains_a_pipeline_deeper_than_the_worker_queue_without_dropping_requests() {
    // Item T regression: the host's worker pool caps concurrent
    // threads via `HOST_WORKER_THREADS = 16` and back-pressures the
    // stdin reader through a `mpsc::sync_channel` of capacity
    // `HOST_WORKER_QUEUE_DEPTH = 32`. Pipeline more requests than
    // those two combined and verify every one of them eventually
    // gets a response (none is dropped, the host doesn't deadlock,
    // and the request_id correlation stays intact). 60 requests
    // exceeds the 48-slot in-flight cap by a healthy margin.
    let temp = setup_store_with_secret("alice", "value: hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    // Drain the stdout side concurrently with the write loop so the
    // pool doesn't backpressure on stdout (it serialises writes
    // through a single Mutex<Stdout>; a piled-up reader would
    // deadlock the worker that's waiting to flush). Also: stdin
    // must stay open until every response is drained — if we close
    // it first, the host's main loop returns and the process exits
    // before in-flight workers can flush the rest of the responses,
    // matching the production "stdin close → process exits" contract
    // intentionally. Test pinning is the inverse: keep stdin alive
    // long enough to confirm every queued request is honoured.
    let reader = std::thread::spawn(move || {
        let mut received = Vec::with_capacity(60);
        for _ in 0..60 {
            received.push(read_response(&mut stdout));
        }
        received
    });

    for i in 1..=60u64 {
        let payload = format!(r#"{{"request_id":{i},"action":"list_stores"}}"#);
        write_request(&mut stdin, payload.as_bytes());
    }

    let responses = reader.join().unwrap();
    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());

    // Every request_id from 1..=60 must appear exactly once in the
    // reply stream — order doesn't matter (workers race), but no
    // request can be silently dropped or duplicated.
    let mut ids: Vec<u64> = responses
        .iter()
        .filter_map(|r| {
            // The wire shape is `{"request_id":N,...}`. Parse it out
            // by hand to avoid pulling serde_json into the test.
            let needle = "\"request_id\":";
            let start = r.find(needle)? + needle.len();
            let rest = &r[start..];
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse().ok()
        })
        .collect();
    ids.sort_unstable();
    let expected: Vec<u64> = (1..=60).collect();
    assert_eq!(
        ids, expected,
        "every pipelined request_id must come back exactly once"
    );
}

#[test]
fn host_processes_requests_concurrently_so_a_slow_store_does_not_gate_a_fast_one() {
    // Two stores share the same identity / recipient; only the slow
    // one carries a `pull_commands = ["sleep 1"]` hook. We pipeline
    // BOTH queries into stdin before draining stdout — under v1's
    // single-threaded model the slow query would have to finish
    // before the fast one starts, and the fast reply would come back
    // second. With per-request worker threads the fast reply comes
    // back first; we identify which is which by `request_id`.
    let temp = setup_two_stores_with_slow_pull();
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        // Disable hook throttling so the sleep ALWAYS fires on the
        // slow store — without this the second test run within the
        // throttling window would skip the sleep and the timing
        // ordering would be ambiguous.
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"query","store":"slow","domain":"example.com"}"#,
    );
    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"query","store":"fast","domain":"example.com"}"#,
    );

    let first = read_response(&mut stdout);
    let second = read_response(&mut stdout);

    assert!(
        first.contains(r#""request_id":2"#),
        "fast store should win the race; first response was: {first}"
    );
    assert!(
        second.contains(r#""request_id":1"#),
        "slow store should arrive second; second response was: {second}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

fn setup_two_stores_with_slow_pull() -> TempDir {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("config.toml");
    let identity_file = temp.path().join("identity");
    let slow_store = temp.path().join("slow.store");
    let fast_store = temp.path().join("fast.store");

    std::fs::write(&identity_file, TEST_PRIVATE_KEY).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&identity_file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    for (name, path) in [("slow", &slow_store), ("fast", &fast_store)] {
        pasejo(&config)
            .args(["store", "add", "--name", name, "--path"])
            .arg(path)
            .assert()
            .success();
    }

    pasejo(&config)
        .args(["identity", "add", "--global", "--file"])
        .arg(&identity_file)
        .assert()
        .success();

    for name in ["slow", "fast"] {
        pasejo(&config)
            .args([
                "recipient",
                "add",
                "--store",
                name,
                "--public-key",
                TEST_PUBLIC_KEY,
            ])
            .assert()
            .success();

        pasejo(&config)
            .args(["secret", "add", "--store", name, &format!("{name}-secret")])
            .write_stdin("password\nurl: https://example.com\n")
            .assert()
            .success();
    }

    // Set the slow store's `pull_commands` to `["sleep 1"]` by
    // patching the on-disk TOML — the CLI doesn't expose a "configure
    // pull hooks" command. `StoreRegistration` serialises path, name,
    // identities, pull_commands, push_commands in a fixed order with
    // recipients living inside the encrypted store file rather than in
    // the config, so this exact byte-level match is stable.
    let contents = std::fs::read_to_string(&config).unwrap();
    let injected = contents.replace(
        "name = \"slow\"\nidentities = []\npull_commands = []\n",
        "name = \"slow\"\nidentities = []\npull_commands = [\"sleep 1\"]\n",
    );
    assert_ne!(
        contents, injected,
        "expected to find the slow store entry to patch; config was:\n{contents}"
    );
    std::fs::write(&config, injected).unwrap();

    temp
}

#[test]
fn host_handles_multiple_requests_in_sequence() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"list_stores"}"#);
    let first = read_response(&mut stdout);
    assert!(first.contains(r#""stores":"#));

    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let second = read_response(&mut stdout);
    assert!(second.contains(r#""results":"#));
    assert!(second.contains(r#""name":"alice""#));

    write_request(&mut stdin, br#"{"action":"query","domain":"nothing.test"}"#);
    let third = read_response(&mut stdout);
    assert!(third.contains(r#""results":[]"#));

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn host_runs_pull_hooks_of_stores_sharing_a_directory_one_at_a_time() {
    let temp = setup_two_stores_with_slow_pull();
    let config = temp.path().join("config.toml");
    let contents = std::fs::read_to_string(&config).unwrap();
    let (global, stores) = contents.split_once("[[stores]]").unwrap();
    let global_hook = global.replace(
        "pull_commands = []\n",
        "pull_commands = [\"sh -c 'mkdir lk && sleep 1 && rmdir lk'\"]\n",
    );
    assert_ne!(global, global_hook, "config was:\n{contents}");
    let stores = stores.replace("pull_commands = [\"sleep 1\"]\n", "pull_commands = []\n");
    std::fs::write(&config, format!("{global_hook}[[stores]]{stores}")).unwrap();

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"query","store":"slow","domain":"example.com"}"#,
    );
    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"query","store":"fast","domain":"example.com"}"#,
    );

    let replies = [read_response(&mut stdout), read_response(&mut stdout)];
    for reply in &replies {
        assert!(
            !reply.contains(r#""error":"hook_failed""#),
            "pull hooks of stores in one directory overlapped: {reply}"
        );
    }

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

/// The release log goes to the directory holding `config`, never to the
/// developer's data directory.
fn spawn_host_in(config: &std::path::Path, language: &str) -> std::process::Child {
    StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", config)
        .env("PASEJO_DATA_DIR", config.parent().unwrap())
        .env_remove("LANGUAGE")
        .env("LANG", language)
        .env("LC_ALL", language)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn host_reveals_the_credential_only_for_the_domain_the_secret_matches() {
    let temp = setup_store_with_secret(
        "alice",
        "hunter2\nurl: https://example.com\nlogin: alice@example.com\n",
    );
    let config = temp.path().join("config.toml");

    let mut child = spawn_host_in(&config, "en");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"reveal","store":"test","name":"alice","domain":"example.com"}"#,
    );
    let revealed = read_response(&mut stdout);
    assert_eq!(
        revealed,
        r#"{"request_id":1,"credential":{"name":"alice","store":"test","username":"alice@example.com","password":"hunter2"}}"#
    );

    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"reveal","store":"test","name":"alice","domain":"example.org"}"#,
    );
    let refused = read_response(&mut stdout);
    assert_eq!(
        refused,
        r#"{"request_id":2,"error":"not_matched","store":"test"}"#
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn host_reveals_a_totp_code_only_on_an_exact_host_match() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://login.example.com\n");
    let config = temp.path().join("config.toml");
    pasejo(&config)
        .args([
            "otp",
            "add",
            "--type",
            "totp",
            "--secret",
            "JEQDYMZAN5YGK3RAONXXK4TDMU",
            "alice",
        ])
        .assert()
        .success();

    let mut child = spawn_host_in(&config, "en");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"action":"reveal","store":"test","name":"alice","domain":"login.example.com"}"#,
    );
    let exact = read_response(&mut stdout);
    assert!(exact.contains(r#""otp":""#), "response was: {exact}");

    write_request(
        &mut stdin,
        br#"{"action":"reveal","store":"test","name":"alice","domain":"www.example.com"}"#,
    );
    let sibling = read_response(&mut stdout);
    assert!(
        sibling.contains(r#""password":"hunter2""#),
        "response was: {sibling}"
    );
    assert!(!sibling.contains("otp"), "response was: {sibling}");

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn host_translates_the_missing_store_file_hint() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");
    std::fs::remove_file(temp.path().join("store")).unwrap();

    let mut child = spawn_host_in(&config, "de");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let response = read_response(&mut stdout);

    assert!(
        response.contains("Speicherdatei existiert nicht: <store>"),
        "response was: {response}"
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn host_translates_the_oversized_message_error() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = spawn_host_in(&config, "de");
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(&u32::MAX.to_ne_bytes()).unwrap();
    drop(stdin);

    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("überschreitet die Native-Messaging-Grenze"),
        "stderr was: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn host_runs_no_pull_hook_while_another_process_holds_the_store_lock() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");
    let marker = temp.path().join("pulled");
    pasejo(&config)
        .args(["hook", "set", "--pull"])
        .arg(format!("touch {}", marker.display()))
        .assert()
        .success();
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(temp.path().join("store.lock"))
        .unwrap();
    lock.lock().unwrap();

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let response = read_response(&mut stdout);
    let pulled_under_lock = marker.exists();
    drop(lock);
    drop(stdin);
    assert!(child.wait().unwrap().success());

    assert!(
        response.contains(r#""error":"store_unreachable""#),
        "response was: {response}"
    );
    assert!(
        !pulled_under_lock,
        "the host ran its pull hook while another process held the store lock"
    );
}

#[cfg(unix)]
fn passphrase_protected_identity() -> Vec<u8> {
    let mut recipient = age::scrypt::Recipient::new(age::secrecy::SecretString::from(
        String::from("correct horse battery staple"),
    ));
    recipient.set_work_factor(10);
    age::encrypt_and_armor(&recipient, TEST_PRIVATE_KEY.as_bytes())
        .unwrap()
        .into_bytes()
}

#[cfg(unix)]
fn recording_pinentry(temp: &TempDir, marker: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let pinentry = temp.path().join("pinentry");
    std::fs::write(
        &pinentry,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&pinentry, std::fs::Permissions::from_mode(0o700)).unwrap();
    pinentry
}

#[cfg(unix)]
fn query_with_pinentry(config: &std::path::Path, pinentry: &std::path::Path) -> String {
    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", config)
        .env("PINENTRY_PROGRAM", pinentry)
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_request(&mut stdin, br#"{"action":"query","domain":"example.com"}"#);
    let response = read_response(&mut stdout);
    drop(stdin);
    assert!(child.wait().unwrap().success());
    response
}

#[cfg(unix)]
#[test]
fn host_reports_a_passphrase_protected_identity_as_locked_without_prompting() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");
    std::fs::write(
        temp.path().join("identity"),
        passphrase_protected_identity(),
    )
    .unwrap();
    let marker = temp.path().join("prompted");
    let pinentry = recording_pinentry(&temp, &marker);

    let response = query_with_pinentry(&config, &pinentry);

    assert!(
        !marker.exists(),
        "the host asked for the passphrase of an encrypted identity"
    );
    assert!(
        response.contains(r#""error":"identity_locked""#),
        "response was: {response}"
    );
    assert!(response.contains("passphrase"), "response was: {response}");
}

#[cfg(unix)]
#[test]
fn host_decrypts_with_an_unencrypted_identity_next_to_a_passphrase_protected_one() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");
    let protected = temp.path().join("a-protected-identity");
    std::fs::write(&protected, passphrase_protected_identity()).unwrap();
    pasejo(&config)
        .args(["identity", "add", "--global", "--file"])
        .arg(&protected)
        .assert()
        .success();
    let marker = temp.path().join("prompted");
    let pinentry = recording_pinentry(&temp, &marker);

    let response = query_with_pinentry(&config, &pinentry);

    assert!(
        response.contains(r#""name":"alice""#),
        "response was: {response}"
    );
    assert!(
        !marker.exists(),
        "the host asked for the passphrase of an encrypted identity"
    );
}

#[test]
fn host_honours_the_fill_and_confirm_policies_stored_in_secrets() {
    let scaffold = common::StoreScaffold::new()
        .store("test")
        .identity()
        .recipient()
        .secret(
            "alice",
            "hunter2\nurl: https://example.com\npasejo-confirm: always\n",
        )
        .secret(
            "bob",
            "hunter3\nurl: https://example.com\npasejo-fill: never\n",
        );
    let mut child = spawn_host_in(&scaffold.config, "en");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"query","domain":"example.com"}"#,
    );
    let query = read_response(&mut stdout);
    assert_eq!(
        query,
        r#"{"request_id":1,"results":[{"name":"alice","store":"test","username":"","url":"https://example.com","match_kind":"url_field","score":1.0,"confirm":true}]}"#
    );

    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"reveal","store":"test","name":"bob","domain":"example.com"}"#,
    );
    let refused = read_response(&mut stdout);
    assert_eq!(
        refused,
        r#"{"request_id":2,"error":"not_matched","store":"test"}"#
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

fn host_with_browser_args_answers_list_stores(browser_args: &[&str]) {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .args(browser_args)
        .env("PASEJO_CONFIG", &config)
        .env("PASEJO_DATA_DIR", temp.path())
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_request(&mut stdin, br#"{"request_id":1,"action":"list_stores"}"#);
    let response = read_response(&mut stdout);
    assert!(
        response.contains(r#""protocol_version""#),
        "response was: {response}"
    );

    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn host_accepts_the_origin_argument_chromium_browsers_pass() {
    host_with_browser_args_answers_list_stores(&[
        "chrome-extension://abcdefgh/",
        "--parent-window=0",
    ]);
}

#[test]
fn host_accepts_the_manifest_and_add_on_arguments_firefox_passes() {
    host_with_browser_args_answers_list_stores(&[
        "/home/alice/.mozilla/native-messaging-hosts/wtf.metio.pasejo.json",
        "pasejo@metio.wtf",
    ]);
}

#[test]
fn host_records_each_release_and_browser_log_prints_it() {
    let temp = setup_store_with_secret(
        "alice",
        "hunter2\nurl: https://example.com\nlogin: alice@example.com\n",
    );
    let config = temp.path().join("config.toml");

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host", "chrome-extension://abcdefgh/"])
        .env("PASEJO_CONFIG", &config)
        .env("PASEJO_DATA_DIR", temp.path())
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"reveal","store":"test","name":"alice","domain":"example.com"}"#,
    );
    let revealed = read_response(&mut stdout);
    assert!(revealed.contains(r#""credential""#), "{revealed}");
    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"reveal","store":"test","name":"alice","domain":"example.org"}"#,
    );
    let refused = read_response(&mut stdout);
    assert!(refused.contains("not_matched"), "{refused}");
    drop(stdin);
    assert!(child.wait().unwrap().success());

    let raw =
        std::fs::read_to_string(temp.path().join("pasejo").join("browser-releases.jsonl")).unwrap();
    assert_eq!(raw.lines().count(), 1, "{raw}");
    assert!(!raw.contains("hunter2"), "{raw}");

    let output = pasejo(&config)
        .args(["browser", "log"])
        .env("PASEJO_DATA_DIR", temp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let printed = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 2, "{printed}");
    assert_eq!(lines[0], "Time\tDomain\tStore\tSecret\tCaller");
    let columns: Vec<&str> = lines[1].split('\t').collect();
    assert_eq!(
        columns[1..],
        [
            "example.com",
            "test",
            "alice",
            "chrome-extension://abcdefgh/"
        ],
        "{printed}"
    );
    assert!(columns[0].ends_with('Z'), "{printed}");
}

#[test]
fn host_withholds_a_credential_whose_release_cannot_be_recorded() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");
    std::fs::create_dir_all(temp.path().join("pasejo").join("browser-releases.jsonl")).unwrap();

    let mut child = spawn_host_in(&config, "de");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_request(
        &mut stdin,
        br#"{"request_id":1,"action":"reveal","store":"test","name":"alice","domain":"example.com"}"#,
    );
    let response = read_response(&mut stdout);
    drop(stdin);
    assert!(child.wait().unwrap().success());

    assert!(!response.contains("hunter2"), "response was: {response}");
    assert!(
        response.contains(r#""error":"release_log_failed""#),
        "response was: {response}"
    );
    assert!(
        response.contains("Browser-Freigabeprotokoll"),
        "response was: {response}"
    );
}

fn spawn_host_with_args(
    config: &std::path::Path,
    host_args: &[&str],
    language: &str,
) -> std::process::Child {
    StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .args(host_args)
        .env("PASEJO_CONFIG", config)
        .env("PASEJO_DATA_DIR", config.parent().unwrap())
        .env_remove("LANGUAGE")
        .env("LANG", language)
        .env("LC_ALL", language)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn host_answers_callers_among_its_allowed_callers() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");
    let allowed = [
        "--allowed-caller",
        "chrome-extension://abcdefgh/",
        "--allowed-caller",
        "pasejo@metio.wtf",
    ];

    for browser_args in [
        &["chrome-extension://abcdefgh/", "--parent-window=0"][..],
        &[
            "/home/alice/.mozilla/native-messaging-hosts/wtf.metio.pasejo.json",
            "pasejo@metio.wtf",
        ][..],
    ] {
        let host_args: Vec<&str> = allowed.iter().chain(browser_args).copied().collect();
        let mut child = spawn_host_with_args(&config, &host_args, "en");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();
        write_request(&mut stdin, br#"{"request_id":1,"action":"list_stores"}"#);
        let response = read_response(&mut stdout);
        assert_eq!(
            response,
            r#"{"request_id":1,"protocol_version":2,"stores":[{"name":"test","default":true}]}"#,
            "{browser_args:?}"
        );
        drop(stdin);
        assert!(child.wait().unwrap().success());
    }
}

#[test]
fn host_refuses_a_caller_missing_from_its_allowed_callers() {
    let temp = setup_store_with_secret("alice", "hunter2\nurl: https://example.com\n");
    let config = temp.path().join("config.toml");

    let mut child = spawn_host_with_args(
        &config,
        &[
            "--allowed-caller",
            "chrome-extension://abcdefgh/",
            "chrome-extension://ponmlkji/",
        ],
        "de",
    );
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_request(&mut stdin, br#"{"request_id":1,"action":"list_stores"}"#);
    let listed = read_response(&mut stdout);
    write_request(
        &mut stdin,
        br#"{"request_id":2,"action":"reveal","store":"test","name":"alice","domain":"example.com"}"#,
    );
    let revealed = read_response(&mut stdout);
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    for response in [&listed, &revealed] {
        assert!(!response.contains("hunter2"), "{response}");
        assert!(!response.contains(r#""test""#), "{response}");
        assert!(
            response.contains(r#""error":"caller_not_allowed""#),
            "{response}"
        );
        assert!(
            response.contains("Die Browser-Erweiterung 'chrome-extension://ponmlkji/'"),
            "{response}"
        );
    }
    assert!(listed.starts_with(r#"{"request_id":1,"#), "{listed}");
    assert!(revealed.starts_with(r#"{"request_id":2,"#), "{revealed}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("chrome-extension://ponmlkji/"), "{stderr}");
    assert!(
        !temp
            .path()
            .join("pasejo")
            .join("browser-releases.jsonl")
            .exists()
    );
}

#[test]
fn host_refuses_an_unnamed_caller_when_allowed_callers_are_set() {
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");

    let mut child = spawn_host_with_args(&config, &["--allowed-caller", "pasejo@metio.wtf"], "en");
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_request(&mut stdin, br#"{"request_id":1,"action":"list_stores"}"#);
    let response = read_response(&mut stdout);
    drop(stdin);
    assert!(child.wait().unwrap().success());

    assert!(
        response.starts_with(r#"{"request_id":1,"error":"caller_not_allowed","hint":"#),
        "{response}"
    );
}
