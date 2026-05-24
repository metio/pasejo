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
