// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::io::{Read, Write};
use std::process::{Command as StdCommand, Stdio};

use assert_cmd::Command as AssertCommand;
use assert_cmd::cargo::cargo_bin;
use assert_fs::TempDir;

// Fixed age key fixture (also used in tests/cmd/export/bitwarden). Using
// a known value keeps the test free of the `age` crate's key-generation
// API surface — the same code path is exercised either way.
const TEST_PRIVATE_KEY: &str = "# public key: age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd\n\
     AGE-SECRET-KEY-14LGKWZMHDDWMLCL5EUU2FSQNEX33DZYR3F8VEPZSA670QRC3DA3SNSD535";
const TEST_PUBLIC_KEY: &str = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd";

fn setup_store_with_secret(secret_name: &str, secret_value: &str) -> TempDir {
    let temp = TempDir::new().unwrap();
    let config = temp.path().join("config.toml");
    let identity_file = temp.path().join("identity");
    let store_path = temp.path().join("store");

    std::fs::write(&identity_file, TEST_PRIVATE_KEY).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&identity_file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }

    AssertCommand::cargo_bin("pasejo")
        .unwrap()
        .args(["store", "add", "--name", "test", "--path"])
        .arg(&store_path)
        .env("PASEJO_CONFIG", &config)
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .assert()
        .success();

    AssertCommand::cargo_bin("pasejo")
        .unwrap()
        .args(["identity", "add", "--file"])
        .arg(&identity_file)
        .env("PASEJO_CONFIG", &config)
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .assert()
        .success();

    AssertCommand::cargo_bin("pasejo")
        .unwrap()
        .args(["recipient", "add", "--public-key", TEST_PUBLIC_KEY])
        .env("PASEJO_CONFIG", &config)
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .assert()
        .success();

    AssertCommand::cargo_bin("pasejo")
        .unwrap()
        .args(["secret", "add", secret_name])
        .env("PASEJO_CONFIG", &config)
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .write_stdin(secret_value)
        .assert()
        .success();

    temp
}

fn write_request(stdin: &mut impl Write, payload: &[u8]) {
    let len = u32::try_from(payload.len()).unwrap();
    stdin.write_all(&len.to_le_bytes()).unwrap();
    stdin.write_all(payload).unwrap();
    stdin.flush().unwrap();
}

fn read_response(stdout: &mut impl Read) -> String {
    let mut len_buf = [0u8; 4];
    stdout.read_exact(&mut len_buf).unwrap();
    let len = u32::from_le_bytes(len_buf) as usize;
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
    assert!(response.contains(r#""password":"hunter2""#));
    assert!(response.contains(r#""match_kind":"url_field""#));
    assert!(response.contains(r#""score":1.0"#));

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
        response.contains(r#""protocol_version":1"#),
        "list_stores must include protocol_version so the extension can refuse a mismatched host; response was: {response}"
    );

    drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
}

#[test]
fn host_reports_store_unreachable_when_store_file_missing() {
    // Register a store and identity but delete the store file before
    // querying. The bootstrap path (`acquire_store_lock` returning
    // `Ok(None)`) must surface as `store_unreachable`, not as the
    // catch-all `identity_locked` — the actionable fix is filesystem-side,
    // not identity-side.
    let temp = setup_store_with_secret("alice", "hunter2\n");
    let config = temp.path().join("config.toml");
    let store_path = temp.path().join("store");
    std::fs::remove_file(&store_path).unwrap();

    let mut child = StdCommand::new(cargo_bin("pasejo"))
        .args(["browser", "host"])
        .env("PASEJO_CONFIG", &config)
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
        AssertCommand::cargo_bin("pasejo")
            .unwrap()
            .args(["store", "add", "--name", name, "--path"])
            .arg(path)
            .env("PASEJO_CONFIG", &config)
            .env("LANG", "en")
            .env("LC_ALL", "en")
            .assert()
            .success();
    }

    AssertCommand::cargo_bin("pasejo")
        .unwrap()
        .args(["identity", "add", "--global", "--file"])
        .arg(&identity_file)
        .env("PASEJO_CONFIG", &config)
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .assert()
        .success();

    for name in ["slow", "fast"] {
        AssertCommand::cargo_bin("pasejo")
            .unwrap()
            .args([
                "recipient",
                "add",
                "--store",
                name,
                "--public-key",
                TEST_PUBLIC_KEY,
            ])
            .env("PASEJO_CONFIG", &config)
            .env("LANG", "en")
            .env("LC_ALL", "en")
            .assert()
            .success();

        AssertCommand::cargo_bin("pasejo")
            .unwrap()
            .args(["secret", "add", "--store", name, &format!("{name}-secret")])
            .env("PASEJO_CONFIG", &config)
            .env("LANG", "en")
            .env("LC_ALL", "en")
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
