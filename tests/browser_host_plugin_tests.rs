// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The browser host decrypting through an age plugin identity, the way a
//! hardware key is used through `age-plugin-yubikey`. A shell script stands
//! in for the plugin binary: it speaks the identity side of the age plugin
//! protocol and can wait for a "touch" (a file appearing), ask for a PIN or a
//! confirmation, or report an error, as `age-plugin-yubikey` does.

#![cfg(unix)]

use std::collections::HashSet;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command as StdCommand, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use age::secrecy::ExposeSecret;
use age_core::format::{FileKey, Stanza};
use assert_cmd::cargo::cargo_bin;
use assert_fs::TempDir;

mod common;
use common::{TEST_PRIVATE_KEY, TEST_PUBLIC_KEY, pasejo};

/// Long enough for a debug build on a loaded CI runner, short enough that a
/// host stuck on a prompt fails the test instead of hanging the suite.
const REPLY_TIMEOUT: Duration = Duration::from_secs(20);

const PLUGIN_IDENTITY: &str = "AGE-PLUGIN-PASEJOTEST-1WPSHXET2DUKHGETNWSKKKETE0JLNWH";
const PLUGIN_STANZA_TAG: &str = "pasejo-test";

/// The base64 encoding (unpadded) of the PIN `123456` the fake pinentry
/// answers with.
const PIN_BASE64: &str = "MTIzNDU2";

/// The identity side of the age plugin protocol. Every stanza body it sends
/// is a single base64 line shorter than 64 characters, the only shape this
/// script needs. The file key travels in clear inside the `pasejo-test`
/// stanza, which [`PlainStanzaRecipient`] writes.
const FAKE_PLUGIN: &str = r#"#!/bin/sh
[ "$1" = "--age-plugin=identity-v1" ] || exit 1

read_stanza() {
  IFS= read -r header || exit 1
  body=
  while IFS= read -r line; do
    body=$body$line
    [ ${#line} -lt 64 ] && break
  done
}

send() {
  printf '%s\n%s\n' "$1" "$2"
}

fail() {
  send '-> error identity 0' "$1"
  read_stanza
  send '-> done' ''
  exit 0
}

key=
while read_stanza; do
  set -- $header
  case "$2" in
    done) break ;;
    recipient-stanza) [ "$4" = pasejo-test ] && key=$body ;;
  esac
done

if [ -n "$PASEJO_TEST_PLUGIN_LOG" ]; then
  echo unwrap >> "$PASEJO_TEST_PLUGIN_LOG"
fi
if [ -z "$key" ]; then
  send '-> done' ''
  exit 0
fi
if [ -n "$PASEJO_TEST_PLUGIN_FAIL" ]; then
  fail dG91Y2ggdGltZWQgb3V0IQ
fi
if [ -n "$PASEJO_TEST_PLUGIN_CONFIRM" ]; then
  send '-> confirm Q29udGludWU U2tpcA' SW5zZXJ0IHRoZSBrZXk
  read_stanza
  [ "$header" = '-> ok yes' ] || fail S2V5IG5vdCBmb3VuZA
fi
if [ -n "$PASEJO_TEST_PLUGIN_PIN" ]; then
  send '-> request-secret' UElO
  read_stanza
  [ "$header" = '-> ok' ] && [ "$body" = "$PASEJO_TEST_PLUGIN_PIN" ] || fail UElOIHJlamVjdGVk
fi
if [ -n "$PASEJO_TEST_PLUGIN_TOUCH" ]; then
  send '-> msg' VG91Y2ggeW91ciBrZXk
  read_stanza
  while [ ! -e "$PASEJO_TEST_PLUGIN_TOUCH" ]; do
    /bin/sleep 0.05
  done
fi
send '-> file-key 0' "$key"
read_stanza
send '-> done' ''
"#;

/// A pinentry that answers every request with `OK`, `GETPIN` with the PIN
/// `123456`, and appends each request line to `log`.
fn fake_pinentry(dir: &Path, log: &Path) -> PathBuf {
    let path = dir.join("pinentry-fake");
    write_executable(
        &path,
        &format!(
            r#"#!/bin/sh
echo OK
while IFS= read -r line; do
  printf '%s\n' "$line" >> '{}'
  set -- $line
  case "$1" in
    GETPIN) echo 'D 123456'; echo OK ;;
    BYE) echo OK; exit 0 ;;
    *) echo OK ;;
  esac
done
"#,
            log.display()
        ),
    );
    path
}

fn write_executable(path: &Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}

/// Wraps the file key in a stanza the fake plugin reads back verbatim.
struct PlainStanzaRecipient;

impl age::Recipient for PlainStanzaRecipient {
    fn wrap_file_key(
        &self,
        file_key: &FileKey,
    ) -> Result<(Vec<Stanza>, HashSet<String>), age::EncryptError> {
        Ok((
            vec![Stanza {
                tag: String::from(PLUGIN_STANZA_TAG),
                args: vec![],
                body: file_key.expose_secret().to_vec(),
            }],
            HashSet::new(),
        ))
    }
}

/// A temp directory holding the config, the stores, their identities, and
/// a `bin` directory with the fake plugin. The host runs with `bin` as its
/// whole `PATH`, so no real pinentry is found unless a test names one.
struct World {
    temp: TempDir,
    config: PathBuf,
}

impl World {
    fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let config = temp.path().join("config.toml");
        let bin = temp.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        write_executable(&bin.join("age-plugin-pasejotest"), FAKE_PLUGIN);
        Self { temp, config }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.temp.path().join(name)
    }

    /// Registers store `name` with the secret `alice` for `example.com`.
    /// With `plugin`, the store is re-encrypted so that only the plugin
    /// identity unwraps it, and the store's identity file names that
    /// identity.
    fn store(self, name: &str, plugin: bool) -> Self {
        let store = self.path(&format!("{name}.store"));
        let identity = self.path(&format!("{name}.identity"));
        std::fs::write(&identity, TEST_PRIVATE_KEY).unwrap();
        std::fs::set_permissions(&identity, std::fs::Permissions::from_mode(0o600)).unwrap();
        pasejo(&self.config)
            .args(["store", "add", "--name", name, "--path"])
            .arg(&store)
            .assert()
            .success();
        pasejo(&self.config)
            .args(["identity", "add", "--store", name, "--file"])
            .arg(&identity)
            .assert()
            .success();
        pasejo(&self.config)
            .args(["recipient", "add", "--store", name])
            .args(["--public-key", TEST_PUBLIC_KEY])
            .assert()
            .success();
        pasejo(&self.config)
            .args(["secret", "add", "--store", name, "alice"])
            .write_stdin("hunter2\nurl: https://example.com\nlogin: alice@example.com\n")
            .assert()
            .success();
        if plugin {
            reencrypt_for_plugin(&store);
            std::fs::write(&identity, format!("{PLUGIN_IDENTITY}\n")).unwrap();
        }
        self
    }

    fn host(&self, env: &[(&str, &Path)]) -> Host {
        self.host_in("en", env)
    }

    fn host_in(&self, language: &str, env: &[(&str, &Path)]) -> Host {
        let mut command = StdCommand::new(cargo_bin("pasejo"));
        command
            .args(["browser", "host"])
            .env_clear()
            .env("PATH", self.path("bin"))
            .env("HOME", self.temp.path())
            .env("PASEJO_CONFIG", &self.config)
            .env("PASEJO_DATA_DIR", self.temp.path())
            .env("LANG", language)
            .env("LC_ALL", language)
            .env("PASEJO_TEST_PLUGIN_LOG", self.path("plugin.log"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in env {
            command.env(key, value);
        }
        Host::spawn(command)
    }

    /// How many times the plugin was asked to unwrap a file key.
    fn plugin_runs(&self) -> usize {
        std::fs::read_to_string(self.path("plugin.log"))
            .map(|log| log.lines().count())
            .unwrap_or(0)
    }
}

fn reencrypt_for_plugin(store: &Path) {
    let identity: age::x25519::Identity = TEST_PRIVATE_KEY
        .lines()
        .find(|line| line.starts_with("AGE-SECRET-KEY-"))
        .unwrap()
        .parse()
        .unwrap();
    let plaintext = age::decrypt(&identity, &std::fs::read(store).unwrap()).unwrap();
    let encryptor = age::Encryptor::with_recipients(std::iter::once(
        &PlainStanzaRecipient as &dyn age::Recipient,
    ))
    .unwrap();
    let mut ciphertext = vec![];
    let mut writer = encryptor.wrap_output(&mut ciphertext).unwrap();
    writer.write_all(&plaintext).unwrap();
    writer.finish().unwrap();
    std::fs::write(store, ciphertext).unwrap();
}

/// A running `browser host` whose replies are read on a separate thread, so
/// a host stuck on a prompt fails the test after [`REPLY_TIMEOUT`] instead
/// of blocking it. The process is killed when the value is dropped.
struct Host {
    child: Child,
    stdin: Option<ChildStdin>,
    replies: mpsc::Receiver<String>,
}

impl Host {
    fn spawn(mut command: StdCommand) -> Self {
        let mut child = command.spawn().unwrap();
        let stdin = child.stdin.take();
        let mut stdout = child.stdout.take().unwrap();
        let (sender, replies) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let mut len = [0u8; 4];
                if stdout.read_exact(&mut len).is_err() {
                    return;
                }
                let mut buf = vec![0u8; u32::from_ne_bytes(len) as usize];
                if stdout.read_exact(&mut buf).is_err() {
                    return;
                }
                if sender.send(String::from_utf8(buf).unwrap()).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            replies,
        }
    }

    fn send(&mut self, payload: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin
            .write_all(&u32::try_from(payload.len()).unwrap().to_ne_bytes())
            .unwrap();
        stdin.write_all(payload.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    fn reply(&self) -> String {
        self.replies
            .recv_timeout(REPLY_TIMEOUT)
            .expect("the host did not answer in time")
    }

    fn assert_no_reply_within(&self, wait: Duration) {
        if let Ok(reply) = self.replies.recv_timeout(wait) {
            panic!("the host answered early: {reply}");
        }
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn reveal(request_id: u32, store: &str) -> String {
    format!(
        r#"{{"request_id":{request_id},"action":"reveal","store":"{store}","name":"alice","domain":"example.com"}}"#
    )
}

fn credential(request_id: u32, store: &str) -> String {
    format!(
        r#"{{"request_id":{request_id},"credential":{{"name":"alice","store":"{store}","username":"alice@example.com","password":"hunter2"}}}}"#
    )
}

fn wait_until(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + REPLY_TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "condition not met in time");
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn host_runs_the_plugin_once_per_query_and_once_per_reveal() {
    let world = World::new().store("key", true);
    let mut host = world.host(&[]);

    host.send(r#"{"request_id":1,"action":"query","store":"key","domain":"example.com"}"#);
    let query = host.reply();
    assert!(query.contains(r#""name":"alice""#), "reply was: {query}");
    assert_eq!(world.plugin_runs(), 1);

    host.send(&reveal(2, "key"));
    assert_eq!(host.reply(), credential(2, "key"));
    assert_eq!(world.plugin_runs(), 2);
}

#[test]
fn host_answers_another_store_while_a_plugin_waits_for_a_touch() {
    let world = World::new().store("key", true).store("file", false);
    let touch = world.path("touched");
    let mut host = world.host(&[("PASEJO_TEST_PLUGIN_TOUCH", &touch)]);

    host.send(&reveal(1, "key"));
    host.send(&reveal(2, "file"));

    assert_eq!(host.reply(), credential(2, "file"));
    host.assert_no_reply_within(Duration::from_millis(300));

    std::fs::write(&touch, b"").unwrap();
    assert_eq!(host.reply(), credential(1, "key"));
}

#[test]
fn host_releases_the_store_lock_while_a_plugin_waits_for_a_touch() {
    let world = World::new().store("key", true);
    let touch = world.path("touched");
    let mut host = world.host(&[("PASEJO_TEST_PLUGIN_TOUCH", &touch)]);

    host.send(&reveal(1, "key"));
    wait_until(|| world.plugin_runs() == 1);

    let lock = std::fs::File::open(world.path("key.store.lock")).unwrap();
    let locked_by_host = lock.try_lock().is_err();
    drop(lock);
    std::fs::write(&touch, b"").unwrap();
    assert_eq!(host.reply(), credential(1, "key"));
    assert!(
        !locked_by_host,
        "the host held the store lock while the plugin waited for a touch"
    );
}

#[test]
fn host_reports_a_plugin_error_as_identity_locked() {
    let world = World::new().store("key", true);
    let fail = world.path("unused");
    let mut host = world.host(&[("PASEJO_TEST_PLUGIN_FAIL", &fail)]);

    host.send(&reveal(1, "key"));
    let reply = host.reply();

    assert!(
        reply.starts_with(r#"{"request_id":1,"error":"identity_locked","store":"key""#),
        "reply was: {reply}"
    );
    assert!(reply.contains("touch timed out!"), "reply was: {reply}");
}

#[test]
fn host_asks_for_a_plugin_pin_through_the_configured_pinentry() {
    let world = World::new().store("key", true);
    let pinentry_log = world.path("pinentry.log");
    let pinentry = fake_pinentry(&world.path("bin"), &pinentry_log);
    let mut host = world.host(&[
        ("PASEJO_TEST_PLUGIN_PIN", Path::new(PIN_BASE64)),
        ("PINENTRY_PROGRAM", &pinentry),
    ]);

    host.send(&reveal(1, "key"));

    assert_eq!(host.reply(), credential(1, "key"));
    let log = std::fs::read_to_string(&pinentry_log).unwrap();
    assert!(log.lines().any(|line| line == "GETPIN"), "log was: {log}");
    assert!(
        log.lines().any(|line| line == "SETDESC PIN"),
        "the dialog does not show the plugin's request; log was: {log}"
    );
    assert!(
        log.lines()
            .any(|line| line == "SETPROMPT PIN or passphrase"),
        "log was: {log}"
    );
}

#[test]
fn host_labels_the_pinentry_prompt_in_the_user_language() {
    let world = World::new().store("key", true);
    let pinentry_log = world.path("pinentry.log");
    let pinentry = fake_pinentry(&world.path("bin"), &pinentry_log);
    let mut host = world.host_in(
        "de",
        &[
            ("PASEJO_TEST_PLUGIN_PIN", Path::new(PIN_BASE64)),
            ("PINENTRY_PROGRAM", &pinentry),
        ],
    );

    host.send(&reveal(1, "key"));

    assert_eq!(host.reply(), credential(1, "key"));
    let log = std::fs::read_to_string(&pinentry_log).unwrap();
    assert!(
        log.lines()
            .any(|line| line == "SETPROMPT PIN oder Passphrase"),
        "log was: {log}"
    );
}

#[test]
fn host_asks_for_a_plugin_confirmation_through_the_configured_pinentry() {
    let world = World::new().store("key", true);
    let pinentry_log = world.path("pinentry.log");
    let pinentry = fake_pinentry(&world.path("bin"), &pinentry_log);
    let mut host = world.host(&[
        ("PASEJO_TEST_PLUGIN_CONFIRM", Path::new("1")),
        ("PINENTRY_PROGRAM", &pinentry),
    ]);

    host.send(&reveal(1, "key"));

    assert_eq!(host.reply(), credential(1, "key"));
    let log = std::fs::read_to_string(&pinentry_log).unwrap();
    assert!(log.lines().any(|line| line == "CONFIRM"), "log was: {log}");
    assert!(
        log.lines().any(|line| line == "SETDESC Insert the key"),
        "log was: {log}"
    );
    assert!(
        log.lines().any(|line| line == "SETOK Continue"),
        "log was: {log}"
    );
    assert!(
        log.lines().any(|line| line == "SETCANCEL Skip"),
        "log was: {log}"
    );
}

#[test]
fn host_declines_a_plugin_confirmation_when_no_pinentry_is_available() {
    let world = World::new().store("key", true);
    let mut host = world.host(&[("PASEJO_TEST_PLUGIN_CONFIRM", Path::new("1"))]);

    host.send(&reveal(1, "key"));
    let reply = host.reply();

    assert!(
        reply.starts_with(r#"{"request_id":1,"error":"identity_locked","store":"key""#),
        "reply was: {reply}"
    );
    assert!(reply.contains("Key not found"), "reply was: {reply}");
}

#[test]
fn host_declines_a_plugin_pin_request_when_no_pinentry_is_available() {
    let world = World::new().store("key", true);
    let mut host = world.host(&[("PASEJO_TEST_PLUGIN_PIN", Path::new(PIN_BASE64))]);

    host.send(&reveal(1, "key"));
    let reply = host.reply();

    assert!(
        reply.starts_with(r#"{"request_id":1,"error":"identity_locked","store":"key""#),
        "reply was: {reply}"
    );
    assert!(reply.contains("PIN rejected"), "reply was: {reply}");
}
