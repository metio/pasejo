// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `pasejo` CLI against a real server. Every test starts a server, gives
//! each person their own installation (configuration, data directory and
//! age identity in a temporary directory) and runs the built binary as a
//! person would. Commands run in `block_in_place` so the server keeps
//! serving on the other runtime workers.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::Duration;

use age::secrecy::ExposeSecret;
use common::{Harness, Options, between, capitalize};
use reqwest::StatusCode;
use serde_json::{Value, json};
use tempfile::TempDir;

/// The `pasejo` binary, built once per test run. `PASEJO_BIN` names a
/// binary built elsewhere.
fn binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        if let Ok(path) = std::env::var("PASEJO_BIN") {
            return PathBuf::from(path);
        }
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let output = Command::new(cargo)
            .args([
                "build",
                "--quiet",
                "--package",
                "pasejo",
                "--bin",
                "pasejo",
                "--message-format=json",
            ])
            .current_dir(workspace)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| message["target"]["name"] == "pasejo")
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo names the pasejo executable")
    })
}

fn sync<T>(work: impl FnOnce() -> T) -> T {
    tokio::task::block_in_place(work)
}

/// The outcome of one command.
struct Run {
    success: bool,
    stdout: String,
    stderr: String,
}

impl Run {
    fn from(output: &Output) -> Self {
        Self {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// One person's pasejo installation.
struct Installation {
    dir: TempDir,
    url: String,
    identity: age::x25519::Identity,
}

impl Installation {
    /// An installation with a global age identity and nothing else.
    fn new(harness: &Harness) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let identity = age::x25519::Identity::generate();
        let installation = Self {
            dir,
            url: harness.url.clone(),
            identity,
        };
        let file = installation.identity_file();
        std::fs::write(&file, installation.identity.to_string().expose_secret()).unwrap();
        installation.ok(&[
            "identity",
            "add",
            "--global",
            "--file",
            file.to_str().unwrap(),
        ]);
        installation
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn identity_file(&self) -> PathBuf {
        self.path().join("identity")
    }

    fn recipient(&self) -> String {
        self.identity.to_public().to_string()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(binary());
        command
            .args(args)
            .current_dir(self.path())
            .env("PASEJO_CONFIG", self.path().join("config.toml"))
            .env("PASEJO_DATA_DIR", self.path().join("data"))
            .env("HOME", self.path())
            .env("XDG_CONFIG_HOME", self.path().join("xdg-config"))
            .env("XDG_DATA_HOME", self.path().join("xdg-data"))
            .env("XDG_RUNTIME_DIR", self.path())
            .env("LANG", "en")
            .env("LC_ALL", "en")
            .env("EDITOR", "false")
            .env("VISUAL", "")
            .env_remove("LANGUAGE")
            .env_remove("PASEJO_DEFAULT_STORE")
            .stdin(Stdio::null());
        command
    }

    fn run(&self, args: &[&str]) -> Run {
        sync(|| Run::from(&self.command(args).output().unwrap()))
    }

    fn run_with_input(&self, args: &[&str], input: &str) -> Run {
        sync(|| {
            let mut child = self
                .command(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
            Run::from(&child.wait_with_output().unwrap())
        })
    }

    /// Runs a command that must succeed.
    fn ok(&self, args: &[&str]) -> Run {
        let run = self.run(args);
        assert!(run.success, "pasejo {args:?} failed:\n{}", run.all());
        run
    }

    /// Runs a command that must fail, returning its output.
    fn fails(&self, args: &[&str]) -> String {
        let run = self.run(args);
        assert!(!run.success, "pasejo {args:?} succeeded:\n{}", run.all());
        run.all()
    }

    fn add_secret(&self, name: &str, value: &str) {
        let run = self.run_with_input(&["secret", "add", name], value);
        assert!(run.success, "secret add {name} failed:\n{}", run.all());
    }

    /// Signs in through the device grant, approving it in the browser as
    /// `subject`.
    async fn login(&self, harness: &Harness, subject: &str) {
        let mut child = self
            .command(&["server", "login", &self.url, "--label", subject])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (lines, received) = mpsc::channel();
        let stderr = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if lines.send(line).is_err() {
                    break;
                }
            }
        });
        let mut transcript = String::new();
        let url = sync(|| {
            loop {
                let line = received
                    .recv_timeout(Duration::from_secs(30))
                    .unwrap_or_else(|_| {
                        let _ = child.kill();
                        panic!("no sign-in link:\n{transcript}");
                    });
                transcript.push_str(&line);
                transcript.push('\n');
                if let Some(url) = between(&line, "Open ", " in a browser") {
                    break url;
                }
            }
        });
        harness.idp.set_user(
            subject,
            &format!("{subject}@example.com"),
            &capitalize(subject),
        );
        let page = harness.follow_to_callback(&url).await;
        assert_eq!(page.status, StatusCode::OK, "{}", page.text());
        let approval = between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
        assert_eq!(
            harness.approve(&approval, "approve").await.status,
            StatusCode::OK
        );
        let status = sync(|| wait(&mut child, Duration::from_secs(60)));
        let rest: Vec<String> = received.try_iter().collect();
        assert!(status, "sign-in failed:\n{transcript}{}", rest.join("\n"));
    }

    /// Creates an organization and returns its identifier and fingerprint.
    fn create_org(&self, name: &str) -> (String, String) {
        let run = self.ok(&["org", "create", name]);
        let text = run.all();
        let id = between(&text, "with identifier ", " and fingerprint").unwrap();
        let fingerprint = between(&text, "and fingerprint ", ".").unwrap();
        (id, fingerprint)
    }

    /// Asks to join `org` and returns this device's identifier and
    /// fingerprint.
    fn join(&self, org: &str, fingerprint: &str) -> (String, String) {
        let run = self.ok(&["org", "join", &self.url, org, "--fingerprint", fingerprint]);
        let text = run.all();
        let device = between(&text, "its identifier ", " and fingerprint").unwrap();
        let fingerprint = between(&text, "and fingerprint ", ".").unwrap();
        (device, fingerprint)
    }

    fn store_id(&self, name: &str) -> String {
        let config = std::fs::read_to_string(self.path().join("config.toml")).unwrap();
        let config: toml::Table = config.parse().unwrap();
        config["stores"]
            .as_array()
            .unwrap()
            .iter()
            .find(|store| store["name"].as_str() == Some(name))
            .and_then(|store| store["remote"]["store"].as_str())
            .unwrap()
            .to_owned()
    }
}

fn wait(child: &mut Child, limit: Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.success();
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Alice's installation with an organization and a default remote store
/// named `team`; returns the installation and the organization's
/// identifier and fingerprint.
async fn founder(harness: &Harness) -> (Installation, String, String) {
    let alice = Installation::new(harness);
    alice.login(harness, "alice").await;
    let (org, fingerprint) = alice.create_org("acme");
    alice.ok(&["store", "create-remote", "team", "--default"]);
    (alice, org, fingerprint)
}

/// Bob joins `org` as a member with an approved device.
async fn member(
    harness: &Harness,
    admin: &Installation,
    org: &str,
    fingerprint: &str,
) -> Installation {
    person(harness, admin, org, fingerprint, "bob").await
}

/// `name` joins `org` as a member with an approved device.
async fn person(
    harness: &Harness,
    admin: &Installation,
    org: &str,
    fingerprint: &str,
    name: &str,
) -> Installation {
    let installation = Installation::new(harness);
    installation.login(harness, name).await;
    let (device, device_fingerprint) = installation.join(org, fingerprint);
    admin.ok(&["org", "role", &format!("{name}@example.com"), "member"]);
    admin.ok(&[
        "org",
        "approve",
        &device,
        "--fingerprint",
        &device_fingerprint,
    ]);
    installation
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn server_login_registers_the_device_and_status_shows_the_session() {
    let harness = Harness::start().await;
    let alice = Installation::new(&harness);
    assert!(
        alice
            .ok(&["server", "status"])
            .all()
            .contains("No server is registered")
    );
    alice.login(&harness, "alice").await;
    let status = alice.ok(&["server", "status"]).all();
    assert!(status.contains(&harness.url), "{status}");
    assert!(status.contains("(alice)"), "{status}");
    assert!(status.contains(", signed in"), "{status}");

    let config = std::fs::read_to_string(alice.path().join("config.toml")).unwrap();
    assert!(
        config.contains(&alice.recipient()),
        "the device uses the global identity's recipient:\n{config}"
    );
    let keys: Vec<_> = std::fs::read_dir(alice.path().join("data/pasejo/servers"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("signing.key"))
        .collect();
    assert_eq!(keys.len(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&keys[0]).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    alice.ok(&["server", "logout"]);
    let status = alice.ok(&["server", "status"]).all();
    assert!(status.contains("not signed in"), "{status}");
    let refused = alice.fails(&["org", "create", "acme"]);
    assert!(refused.contains("not signed in"), "{refused}");

    alice.ok(&["server", "logout", "--forget"]);
    let status = alice.ok(&["server", "status"]).all();
    assert!(status.contains("No server is registered"), "{status}");
    assert!(!keys[0].exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_denied_sign_in_fails_without_a_session() {
    let harness = Harness::start().await;
    let alice = Installation::new(&harness);
    let mut child = alice
        .command(&["server", "login", &harness.url])
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let url = sync(|| {
        let mut line = String::new();
        loop {
            line.clear();
            assert!(stderr.read_line(&mut line).unwrap() > 0, "no sign-in link");
            if let Some(url) = between(&line, "Open ", " in a browser") {
                return url;
            }
        }
    });
    harness.idp.set_user("alice", "alice@example.com", "Alice");
    let page = harness.follow_to_callback(&url).await;
    let approval = between(&page.text(), "name=\"approval\" value=\"", "\"").unwrap();
    harness.approve(&approval, "deny").await;
    assert!(!sync(|| wait(&mut child, Duration::from_secs(60))));
    let mut rest = String::new();
    stderr.read_to_string(&mut rest).unwrap();
    assert!(rest.contains("denied in the browser"), "{rest}");
    assert!(
        alice
            .ok(&["server", "status"])
            .all()
            .contains("No server is registered")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn secrets_and_one_time_passwords_round_trip_through_a_remote_store() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    assert!(alice.ok(&["secret", "list"]).stdout.is_empty());

    alice.add_secret(
        "web/github",
        "hunter2\nurl: https://github.com\nusername: alice\n",
    );
    alice.add_secret("db/main", "s3cret\nuser: admin\n");
    let list = alice.ok(&["secret", "list"]).stdout;
    assert_eq!(list.lines().collect::<Vec<_>>(), ["db/main", "web/github"]);
    assert_eq!(
        alice.ok(&["secret", "show", "web/github"]).stdout,
        "hunter2\nurl: https://github.com\nusername: alice\n\n"
    );
    assert_eq!(
        alice
            .ok(&["secret", "show", "web/github", "--field", "username"])
            .stdout
            .trim(),
        "alice"
    );

    alice.ok(&["secret", "move", "web/github", "web/gh"]);
    alice.ok(&["secret", "copy", "web/gh", "web/gh-copy"]);
    alice.ok(&["secret", "remove", "--force", "db/main"]);
    let list = alice.ok(&["secret", "list"]).stdout;
    assert_eq!(list.lines().collect::<Vec<_>>(), ["web/gh", "web/gh-copy"]);
    assert!(
        alice
            .ok(&["secret", "show", "web/gh-copy"])
            .stdout
            .starts_with("hunter2\n")
    );
    let missing = alice.fails(&["secret", "show", "db/main"]);
    assert!(missing.contains("db/main"), "{missing}");

    alice.ok(&["otp", "add", "gh", "--secret", "JBSWY3DPEHPK3PXP"]);
    assert_eq!(alice.ok(&["otp", "list"]).stdout.trim(), "gh");
    let code = alice.ok(&["otp", "show", "gh"]).stdout;
    assert_eq!(code.trim().len(), 6, "{code}");
    alice.ok(&["otp", "move", "gh", "github"]);
    assert_eq!(alice.ok(&["otp", "list"]).stdout.trim(), "github");
    alice.ok(&["otp", "remove", "--force", "github"]);
    assert!(alice.ok(&["otp", "list"]).stdout.trim().is_empty());

    let run = alice.ok(&[
        "exec",
        "--env",
        "TOKEN=web/gh",
        "--",
        "sh",
        "-c",
        "printf %s \"$TOKEN\"",
    ]);
    assert_eq!(run.stdout, "hunter2");
    let resolved = alice.run_with_input(
        &["secret", "resolve"],
        r#"{"token": {"secret": "web/gh"}, "user": {"secret": "web/gh", "field": "username"}}"#,
    );
    assert!(resolved.success, "{}", resolved.all());
    let resolved: Value = serde_json::from_str(&resolved.stdout).unwrap();
    assert_eq!(resolved, json!({"token": "hunter2", "user": "alice"}));

    let query = alice.ok(&["browser", "query", "github.com"]).stdout;
    let query: Value = serde_json::from_str(&query).unwrap();
    let names: Vec<&str> = query["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["web/gh", "web/gh-copy"]);
    assert_eq!(query["results"][0]["username"], "alice");
    assert!(!query.to_string().contains("hunter2"));

    // A second process sees the same store through its own cache.
    alice.add_secret("later", "value\n");
    assert!(alice.ok(&["secret", "list"]).stdout.contains("later"));
    let pin = alice.path().join("data/pasejo/servers");
    assert!(std::fs::read_dir(pin).unwrap().next().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_store_names_never_reach_the_server_in_clear() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    alice.add_secret(
        "bank/very-unique-secret-name",
        "pw\nurl: https://very-unique-host.example\n",
    );
    alice.ok(&[
        "otp",
        "add",
        "very-unique-otp-name",
        "--secret",
        "JBSWY3DPEHPK3PXP",
    ]);
    let mut database = Vec::new();
    for entry in std::fs::read_dir(harness.dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("server.db")
        {
            database.extend(std::fs::read(path).unwrap());
        }
    }
    if database.is_empty() {
        // PostgreSQL holds the data elsewhere; the SQLite run covers this.
        return;
    }
    let text = String::from_utf8_lossy(&database);
    for name in [
        "very-unique-secret-name",
        "very-unique-host",
        "very-unique-otp-name",
        "team",
        "pw\n",
    ] {
        assert!(!text.contains(name), "{name} is in the database");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn remote_stores_refuse_commands_that_only_apply_to_files() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    for args in [
        vec!["recipient", "list"],
        vec!["hook", "set", "--pull", "true"],
        vec![
            "store",
            "merge",
            "--common-ancestor",
            "a",
            "--current-version",
            "b",
            "--other-version",
            "c",
        ],
    ] {
        let refused = alice.fails(&args);
        assert!(refused.contains("is a remote store"), "{args:?}: {refused}");
    }
    let not_remote = alice.fails(&["store", "rekey", "--store", "missing"]);
    assert!(not_remote.contains("missing"), "{not_remote}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_member_reads_after_a_grant_and_loses_access_when_it_ends() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    alice.add_secret("web/github", "hunter2\nurl: https://github.com\n");
    alice.add_secret("web/gitlab", "other\n");

    let bob = Installation::new(&harness);
    bob.login(&harness, "bob").await;
    let wrong = bob.fails(&[
        "org",
        "join",
        &harness.url,
        &org,
        "--fingerprint",
        "0000 0000",
    ]);
    assert!(wrong.contains("does not match"), "{wrong}");
    let missing = bob.fails(&["org", "join", &harness.url, &org]);
    assert!(missing.contains("--fingerprint"), "{missing}");
    let (device, device_fingerprint) = bob.join(&org, &fingerprint);
    let pending = bob.fails(&["org", "stores"]);
    assert!(pending.contains("waits for approval"), "{pending}");

    let devices = alice.ok(&["org", "devices"]).stdout;
    assert!(
        devices.contains(&device) && devices.contains("pending"),
        "{devices}"
    );
    let not_member = alice.fails(&[
        "org",
        "approve",
        &device,
        "--fingerprint",
        &device_fingerprint,
    ]);
    assert!(not_member.contains("is not a member"), "{not_member}");
    alice.ok(&["org", "role", "bob@example.com", "member"]);
    let wrong = alice.fails(&["org", "approve", &device, "--fingerprint", "1234"]);
    assert!(wrong.contains("does not match"), "{wrong}");
    alice.ok(&[
        "org",
        "approve",
        &device,
        "--fingerprint",
        &device_fingerprint,
    ]);
    let devices = alice.ok(&["org", "devices"]).stdout;
    assert!(
        devices
            .lines()
            .any(|line| line.contains(&device) && line.contains("active")),
        "{devices}"
    );
    assert!(
        bob.ok(&["org", "join", &harness.url, &org])
            .all()
            .contains("is active")
    );

    let store = alice.store_id("team");
    let stores = bob.ok(&["org", "stores"]).all();
    assert!(
        stores.contains("no store this device sees"),
        "bob has no grant yet: {stores}"
    );
    let no_keys = bob.fails(&["store", "add-remote", &store, "--name", "team"]);
    assert!(
        no_keys.contains("lacks") || no_keys.contains("permission"),
        "{no_keys}"
    );
    assert!(
        bob.ok(&["store", "list"]).stdout.trim().is_empty(),
        "a store that does not open stays unregistered"
    );

    let granted = alice
        .ok(&["store", "grant", "bob@example.com", "reader"])
        .all();
    assert!(
        granted.contains("done: give 1 devices the keys"),
        "{granted}"
    );
    let stores = bob.ok(&["org", "stores"]).stdout;
    assert!(
        stores
            .lines()
            .any(|line| line.contains(&store) && line.contains("team") && line.contains("reader")),
        "{stores}"
    );
    let added = bob
        .ok(&["store", "add-remote", &store, "--name", "team", "--default"])
        .all();
    assert!(
        added.contains("Registered the remote store team as 'team'"),
        "{added}"
    );
    assert!(
        bob.ok(&["secret", "show", "web/github"])
            .stdout
            .starts_with("hunter2\n")
    );
    let read_only = bob.run_with_input(&["secret", "add", "mine"], "x\n");
    assert!(!read_only.success);
    assert!(
        read_only.all().contains("permission"),
        "{}",
        read_only.all()
    );

    alice.ok(&["store", "grant", "bob@example.com", "writer"]);
    bob.add_secret("from-bob", "b\n");
    assert!(alice.ok(&["secret", "list"]).stdout.contains("from-bob"));

    let continues = alice.ok(&["store", "rotation", "bob@example.com"]).all();
    assert!(continues.contains("still has access"), "{continues}");
    let ended = alice
        .ok(&["store", "grant", "bob@example.com", "none"])
        .all();
    assert!(ended.contains("done: rotate the keys of store"), "{ended}");
    let refused = bob.fails(&["secret", "show", "web/github"]);
    assert!(
        refused.contains("permission") || refused.contains("lacks"),
        "{refused}"
    );

    let rotation = alice.ok(&["store", "rotation", "bob@example.com"]).stdout;
    assert!(
        rotation.lines().any(|line| line == "web/github"),
        "bob read web/github: {rotation}"
    );
    assert!(
        !rotation.contains("web/gitlab"),
        "bob never read web/gitlab: {rotation}"
    );
    let rekeyed = alice.ok(&["store", "rekey"]).all();
    assert!(rekeyed.contains("Rewrote 3 entries"), "{rekeyed}");
    let rotation = alice.ok(&["store", "rotation", "bob@example.com"]).all();
    assert!(
        rotation.contains("fetched nothing that still needs rotating"),
        "{rotation}"
    );
    assert!(
        alice
            .ok(&["secret", "show", "web/github"])
            .stdout
            .starts_with("hunter2\n")
    );

    alice.ok(&["org", "revoke", &device]);
    let devices = alice.ok(&["org", "devices"]).stdout;
    assert!(!devices.contains(&device), "{devices}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn teams_grant_their_members_access() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    alice.add_secret("shared", "team-secret\n");
    let bob = member(&harness, &alice, &org, &fingerprint).await;

    alice.ok(&["org", "team", "create", "ops"]);
    let exists = alice.fails(&["org", "team", "create", "ops"]);
    assert!(exists.contains("already exists"), "{exists}");
    alice.ok(&["org", "team", "add", "ops", "bob@example.com"]);
    alice.ok(&["store", "grant", "ops", "reader"]);
    let store = alice.store_id("team");
    bob.ok(&["store", "add-remote", &store, "--name", "team"]);
    assert_eq!(
        bob.ok(&["secret", "show", "shared"]).stdout,
        "team-secret\n\n"
    );

    alice.ok(&["org", "team", "remove", "ops", "bob@example.com"]);
    let actions = alice.ok(&["org", "actions", "--run"]).all();
    assert!(actions.contains("rotate the keys"), "{actions}");
    assert!(!bob.run(&["secret", "show", "shared"]).success);
    let unknown = alice.fails(&["store", "grant", "nobody", "reader"]);
    assert!(unknown.contains("no team named 'nobody'"), "{unknown}");
    let unknown = alice.fails(&["store", "grant", "nobody@example.com", "reader"]);
    assert!(unknown.contains("nobody@example.com"), "{unknown}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn changes_need_a_second_admin_once_two_admin_is_on() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    let bob = member(&harness, &alice, &org, &fingerprint).await;
    alice.ok(&["org", "role", "bob@example.com", "admin"]);
    // Turning two-admin on is itself a change a second admin signs.
    let proposed = alice.ok(&["org", "settings", "--two-admin", "true"]).all();
    assert!(proposed.contains("needs a second admin"), "{proposed}");
    bob.ok(&[
        "org",
        "cosign",
        &between(&proposed, "Proposal ", " waits").unwrap(),
    ]);
    assert!(
        alice
            .ok(&["org", "settings", "--two-admin", "true"])
            .all()
            .contains("already have these values")
    );

    let proposed = alice
        .ok(&["org", "role", "bob@example.com", "auditor"])
        .all();
    assert!(proposed.contains("needs a second admin"), "{proposed}");
    let proposal = between(&proposed, "Proposal ", " waits").unwrap();
    let listed = bob.ok(&["org", "proposals"]).stdout;
    assert!(listed.contains(&proposal), "{listed}");
    let own = alice.fails(&["org", "cosign", &proposal]);
    assert!(!own.is_empty());
    bob.ok(&["org", "cosign", &proposal]);
    assert!(
        alice
            .ok(&["org", "proposals"])
            .all()
            .contains("No open proposals")
    );
    let unknown = alice.fails(&["org", "cosign", &proposal]);
    assert!(unknown.contains(&proposal), "{unknown}");
    // Bob is an auditor now and may no longer change roles.
    let refused = bob.fails(&["org", "role", "alice@example.com", "member"]);
    assert!(
        refused.contains("membership log refused the change"),
        "{refused}"
    );
    assert!(!refused.contains("does not verify"), "{refused}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn automatic_approval_warns_when_turned_on_and_for_every_device_it_approves() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    let warning = "without anyone comparing its fingerprint. A compromised server";
    let turned_on = alice
        .ok(&["org", "settings", "--device-approval", "automatic"])
        .all();
    assert!(turned_on.contains(warning), "{turned_on}");
    let unchanged = alice
        .ok(&["org", "settings", "--device-approval", "automatic"])
        .all();
    assert!(
        unchanged.contains("already have these values"),
        "{unchanged}"
    );
    assert!(!unchanged.contains(warning), "{unchanged}");
    let other = alice.ok(&["org", "settings", "--auto-join", "true"]).all();
    assert!(!other.contains(warning), "{other}");

    let bob = Installation::new(&harness);
    bob.login(&harness, "bob").await;
    let (device, _) = bob.join(&org, &fingerprint);
    alice.ok(&["org", "role", "bob@example.com", "member"]);
    let ran = alice.ok(&["org", "actions", "--run"]).all();
    assert!(
        ran.contains("Device 'bob' was approved without anyone comparing its fingerprint"),
        "{ran}"
    );
    assert!(alice.ok(&["org", "devices"]).all().contains(&device));

    // The warning is translated.
    alice.ok(&["org", "settings", "--device-approval", "manual"]);
    let german = sync(|| {
        Run::from(
            &alice
                .command(&["org", "settings", "--device-approval", "automatic"])
                .env("LANG", "de")
                .env("LC_ALL", "de")
                .output()
                .unwrap(),
        )
    });
    assert!(german.success, "{}", german.all());
    assert!(
        german
            .all()
            .contains("ohne dass jemand seinen Fingerabdruck vergleicht"),
        "{}",
        german.all()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_store_migrates_to_the_server() {
    let harness = Harness::start().await;
    let alice = Installation::new(&harness);
    let file = alice.path().join("personal.age");
    alice.ok(&[
        "store",
        "add",
        "--name",
        "personal",
        "--path",
        file.to_str().unwrap(),
    ]);
    alice.ok(&["recipient", "add", "--public-key", &alice.recipient()]);
    alice.add_secret("mail", "pw\nusername: alice\n");
    alice.ok(&["otp", "add", "mail-otp", "--secret", "JBSWY3DPEHPK3PXP"]);
    alice.login(&harness, "alice").await;
    let (org, _) = alice.create_org("acme");

    let migrated = alice.ok(&["store", "migrate", "--to-remote", &org]).all();
    assert!(
        migrated.contains("Copied 2 entries of 'personal'"),
        "{migrated}"
    );
    assert!(file.exists(), "the file stays in place");
    assert_eq!(
        alice.ok(&["secret", "show", "mail"]).stdout,
        "pw\nusername: alice\n\n"
    );
    assert_eq!(alice.ok(&["otp", "list"]).stdout.trim(), "mail-otp");
    let again = alice.fails(&["store", "migrate", "--to-remote", &org]);
    assert!(again.contains("is a remote store"), "{again}");
    let stores = alice.ok(&["org", "stores"]).stdout;
    assert!(stores.contains("personal"), "{stores}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_recovery_identity_decrypts_every_store() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    alice.add_secret("root", "toor\n");
    let recovery = age::x25519::Identity::generate();
    let recovery_file = alice.path().join("recovery-identity");
    std::fs::write(&recovery_file, recovery.to_string().expose_secret()).unwrap();
    let output = alice.path().join("recovered");

    let none = alice.fails(&[
        "org",
        "recover",
        "--identity",
        recovery_file.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(none.contains("no recovery recipient"), "{none}");
    let invalid = alice.fails(&["org", "recovery", "set", "not-a-recipient"]);
    assert!(invalid.contains("not-a-recipient"), "{invalid}");
    alice.ok(&["org", "recovery", "set", &recovery.to_public().to_string()]);
    alice.ok(&["org", "actions", "--run"]);
    let recovered = alice
        .ok(&[
            "org",
            "recover",
            "--identity",
            recovery_file.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .all();
    assert!(
        recovered.contains("Recovered 1 entries of team"),
        "{recovered}"
    );

    let store = alice.store_id("team");
    let file = output.join(format!("{store}.age"));
    assert!(file.exists(), "{recovered}");
    let rescuer = Installation::new(&harness);
    rescuer.ok(&[
        "store",
        "add",
        "--name",
        "rescued",
        "--path",
        file.to_str().unwrap(),
    ]);
    rescuer.ok(&[
        "identity",
        "add",
        "--store",
        "rescued",
        "--file",
        recovery_file.to_str().unwrap(),
    ]);
    assert_eq!(rescuer.ok(&["secret", "show", "root"]).stdout, "toor\n\n");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_audit_trail_records_reads_and_verifies() {
    let harness = Harness::start_with(Options {
        extra: "[audit]\ncheckpoint_every = 3\n".into(),
        ..Options::default()
    })
    .await;
    let (alice, _, _) = founder(&harness).await;
    alice.add_secret("web/github", "hunter2\n");
    alice.ok(&["secret", "show", "web/github"]);
    let log = alice.ok(&["audit", "log"]).stdout;
    assert!(log.lines().count() > 3, "{log}");
    assert!(log.contains("object.read"), "{log}");
    assert!(
        log.contains("org.create") || log.contains("log.append"),
        "{log}"
    );
    let filtered = alice.ok(&["audit", "log", "--store", "team"]).stdout;
    assert!(
        filtered
            .lines()
            .all(|line| line.contains(&alice.store_id("team"))),
        "{filtered}"
    );
    let last: u64 = log
        .lines()
        .last()
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    // Reading the log is itself audited, so later events are reads.
    let since = alice
        .ok(&["audit", "log", "--since", &last.to_string()])
        .stdout;
    assert!(!since.trim().is_empty());
    for line in since.lines() {
        let seq: u64 = line.split_whitespace().next().unwrap().parse().unwrap();
        assert!(seq > last && line.contains("audit.read"), "{since}");
    }

    let verified = alice.ok(&["audit", "verify"]).all();
    assert!(verified.contains("Verified"), "{verified}");
    let checkpoints: u64 = between(&verified, "events and ", " checkpoints")
        .unwrap()
        .parse()
        .unwrap();
    assert!(checkpoints >= 1, "{verified}");
    alice.add_secret("more", "x\n");
    let again = alice.ok(&["audit", "verify"]).all();
    assert!(again.contains("Verified"), "{again}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_machine_signs_in_with_a_workload_token() {
    let harness = Harness::start().await;
    let machine = Installation::new(&harness);
    let token = machine.path().join("token");
    std::fs::write(
        &token,
        harness.workload_jwt("system:serviceaccount:ci:builder", "pasejo-server", 600),
    )
    .unwrap();
    machine.ok(&[
        "server",
        "login",
        &harness.url,
        "--workload-token",
        token.to_str().unwrap(),
        "--label",
        "ci",
    ]);
    machine.create_org("machines");
    machine.ok(&["store", "create-remote", "deploy"]);
    machine.add_secret("token", "abc\n");
    assert_eq!(machine.ok(&["secret", "show", "token"]).stdout, "abc\n\n");

    // An expired session renews through the token file.
    let data = machine.path().join("data/pasejo/servers");
    for entry in std::fs::read_dir(&data).unwrap() {
        let session = entry.unwrap().path().join("session.json");
        if session.exists() {
            std::fs::remove_file(session).unwrap();
        }
    }
    assert_eq!(machine.ok(&["secret", "show", "token"]).stdout, "abc\n\n");

    let bad = machine.path().join("bad-token");
    std::fs::write(
        &bad,
        harness.workload_jwt("system:serviceaccount:other:x", "pasejo-server", 600),
    )
    .unwrap();
    let other = Installation::new(&harness);
    let refused = other.fails(&[
        "server",
        "login",
        &harness.url,
        "--workload-token",
        bad.to_str().unwrap(),
    ]);
    assert!(
        refused.contains("no longer accepts")
            || refused.contains("refused")
            || refused.contains("server"),
        "{refused}"
    );
    let missing = other.fails(&[
        "server",
        "login",
        &harness.url,
        "--workload-token",
        "/nonexistent/token",
    ]);
    assert!(missing.contains("/nonexistent/token"), "{missing}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_browser_host_queries_the_lookup_object_and_reveals_one_secret() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    alice.add_secret(
        "web/github",
        "hunter2\nurl: https://github.com\nusername: alice\nnotes: private\n",
    );
    alice.add_secret("web/other", "nope\nurl: https://other.example\n");

    let messages = [
        json!({"request_id": 1, "action": "query", "domain": "github.com"}),
        json!({"request_id": 2, "action": "reveal", "store": "team", "name": "web/github", "domain": "github.com"}),
        json!({"request_id": 3, "action": "reveal", "store": "team", "name": "web/other", "domain": "github.com"}),
    ];
    let mut replies = Vec::new();
    for message in messages {
        replies.push(sync(|| exchange(&alice, &message)));
    }
    let results = replies[0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1, "{}", replies[0]);
    assert_eq!(results[0]["name"], "web/github");
    assert_eq!(results[0]["username"], "alice");
    assert!(!replies[0].to_string().contains("hunter2"));
    assert!(
        !replies[0].to_string().contains("private"),
        "the lookup object carries lookup fields only"
    );
    assert_eq!(
        replies[1]["credential"]["password"], "hunter2",
        "{}",
        replies[1]
    );
    assert_eq!(replies[2]["error"], "not_matched", "{}", replies[2]);

    let log = alice.ok(&["audit", "log"]).stdout;
    assert!(
        log.contains("lookup.read"),
        "a query reads the lookup object: {log}"
    );
    let browser_log = alice.ok(&["browser", "log"]).stdout;
    assert!(browser_log.contains("web/github"), "{browser_log}");
}

/// Sends one native-messaging message to a fresh `browser host` and returns
/// its reply.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_access_review_revokes_grants_through_pending_actions() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    alice.add_secret("db", "s3cret");
    let bob = member(&harness, &alice, &org, &fingerprint).await;
    let carol = person(&harness, &alice, &org, &fingerprint, "carol").await;
    alice.ok(&["store", "grant", "bob@example.com", "reader"]);
    alice.ok(&["store", "grant", "carol@example.com", "reader"]);
    let store = alice.store_id("team");
    bob.ok(&["store", "add-remote", &store, "--name", "team", "--default"]);
    assert_eq!(bob.ok(&["secret", "show", "db"]).stdout, "s3cret\n");

    assert!(
        alice
            .ok(&["org", "review", "list"])
            .all()
            .contains("No access reviews")
    );
    let unknown = alice.fails(&[
        "org",
        "review",
        "open",
        "--reviewer",
        "nobody@example.com",
        "--due",
        "2099-01-31",
    ]);
    assert!(unknown.contains("nobody@example.com"), "{unknown}");
    let bad_date = alice.fails(&[
        "org",
        "review",
        "open",
        "--reviewer",
        "carol@example.com",
        "--due",
        "2099-02-30",
    ]);
    assert!(bad_date.contains("is not a date"), "{bad_date}");
    let opened = alice
        .ok(&[
            "org",
            "review",
            "open",
            "--reviewer",
            "carol@example.com",
            "--due",
            "2099-01-31",
        ])
        .all();
    assert!(
        opened.contains("for store team, due 2099-01-31"),
        "{opened}"
    );
    let review = between(&opened, "Opened access review ", " for store").unwrap();

    let listed = carol.ok(&["org", "review", "list"]).stdout;
    assert!(
        listed.lines().any(|line| line.starts_with(&review)
            && line.contains("2099-01-31")
            && line.ends_with("open")),
        "carol has not registered the store, so the identifier stands in: {listed}"
    );
    assert!(listed.contains(&store), "{listed}");
    assert!(
        listed.contains("- bob@example.com  reader  pending"),
        "{listed}"
    );
    assert!(
        listed.contains("- carol@example.com  reader  pending"),
        "{listed}"
    );

    // A reviewer never decides on their own access.
    let own = carol.fails(&[
        "org",
        "review",
        "decide",
        &review,
        "carol@example.com",
        "keep",
    ]);
    assert!(!own.is_empty());
    // Bob is no reviewer.
    let not_reviewer = bob.fails(&[
        "org",
        "review",
        "decide",
        &review,
        "carol@example.com",
        "keep",
    ]);
    assert!(!not_reviewer.is_empty());
    let not_under_review = carol.fails(&["org", "review", "decide", &review, "dave-team", "keep"]);
    assert!(not_under_review.contains("dave-team"), "{not_under_review}");
    let unknown_review = carol.fails(&[
        "org",
        "review",
        "decide",
        "0123456789abcdef0123456789abcdef",
        "bob@example.com",
        "keep",
    ]);
    assert!(
        unknown_review.contains("No access review"),
        "{unknown_review}"
    );

    let decided = carol
        .ok(&[
            "org",
            "review",
            "decide",
            &review,
            "bob@example.com",
            "revoke",
        ])
        .all();
    assert!(
        decided.contains("Recorded 'revoke' for bob@example.com")
            && decided.contains("pasejo org actions --run"),
        "{decided}"
    );
    let listed = alice.ok(&["org", "review", "list"]).stdout;
    assert!(
        listed.contains("- bob@example.com  reader  revoke"),
        "{listed}"
    );

    let pending = alice.ok(&["org", "actions"]).all();
    assert!(pending.contains("bob@example.com"), "{pending}");
    alice.ok(&["org", "actions", "--run"]);
    // The grant is gone and the store's keys rotated past it.
    alice.ok(&["org", "actions", "--run"]);
    let after = bob.fails(&["secret", "show", "db"]);
    assert!(!after.is_empty());
    assert!(
        !alice
            .ok(&["org", "actions"])
            .all()
            .contains("bob@example.com")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scim_tokens_are_shown_once_listed_and_revoked() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    assert!(
        alice
            .ok(&["org", "scim-token", "list"])
            .all()
            .contains("No live SCIM tokens")
    );
    let created = alice.ok(&["org", "scim-token", "create"]);
    let token = created.stdout.trim().to_owned();
    assert!(token.starts_with("pasejo-scim-"), "{}", created.all());
    assert_eq!(
        created.stdout.lines().count(),
        1,
        "only the token goes to stdout"
    );
    let id = between(&created.stderr, "Created SCIM token ", ".").unwrap();
    assert!(created.stderr.contains("cannot be shown again"));
    let listed = alice.ok(&["org", "scim-token", "list"]).stdout;
    assert!(
        listed.starts_with(&id) && !listed.contains(&token),
        "{listed}"
    );

    let users = harness
        .http
        .get(format!("{}/scim/v2/{org}/Users", harness.url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(users.status(), StatusCode::OK);

    let bob = member(&harness, &alice, &org, &fingerprint).await;
    assert!(!bob.fails(&["org", "scim-token", "create"]).is_empty());
    assert!(!bob.fails(&["org", "scim-token", "list"]).is_empty());

    assert!(
        alice
            .ok(&["org", "scim-token", "revoke", &id])
            .all()
            .contains(&format!("Revoked SCIM token {id}"))
    );
    assert!(
        alice
            .ok(&["org", "scim-token", "list"])
            .all()
            .contains("No live SCIM tokens")
    );
    let refused = harness
        .http
        .get(format!("{}/scim/v2/{org}/Users", harness.url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::UNAUTHORIZED);
    assert!(
        alice
            .fails(&["org", "scim-token", "revoke", "nonsense"])
            .contains("nonsense")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_device_signs_in_through_the_emergency_window_without_the_identity_provider() {
    let harness = Harness::start().await;
    let (alice, _, _) = founder(&harness).await;
    alice.add_secret("db", "s3cret");
    alice.ok(&["server", "logout"]);
    assert!(!alice.fails(&["secret", "show", "db"]).is_empty());

    let closed = alice.fails(&["server", "login", &harness.url, "--emergency"]);
    assert!(!closed.is_empty());
    let unknown = alice.fails(&[
        "server",
        "login",
        "https://elsewhere.example",
        "--emergency",
    ]);
    assert!(
        unknown.contains("Emergency sign-in needs a device that is already registered"),
        "{unknown}"
    );
    let conflicting = alice.fails(&[
        "server",
        "login",
        &harness.url,
        "--emergency",
        "--label",
        "laptop",
    ]);
    assert!(conflicting.contains("cannot be used with"), "{conflicting}");

    let until = harness.now() + 600_000;
    sqlx::query(
        &harness
            .state
            .db
            .sql("INSERT INTO settings (name, value) VALUES ($1, $2)"),
    )
    .bind("emergency_until")
    .bind(until.to_string())
    .execute(harness.state.db.pool())
    .await
    .unwrap();
    let signed_in = alice
        .ok(&["server", "login", &harness.url, "--emergency"])
        .all();
    assert!(
        signed_in.contains("as alice@example.com through the emergency window"),
        "{signed_in}"
    );
    assert_eq!(alice.ok(&["secret", "show", "db"]).stdout, "s3cret\n");

    // The window admits only devices that already exist; it creates none.
    let mallory = Installation::new(&harness);
    let unregistered = mallory.fails(&["server", "login", &harness.url, "--emergency"]);
    assert!(
        unregistered.contains("already registered"),
        "{unregistered}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn machines_are_named_with_their_rule_name() {
    let harness = Harness::start().await;
    let (alice, org, fingerprint) = founder(&harness).await;
    alice.add_secret("deploy", "t0ken");
    let machine = Installation::new(&harness);
    let token = machine.path().join("token");
    std::fs::write(
        &token,
        harness.workload_jwt("system:serviceaccount:ci:builder", "pasejo-server", 600),
    )
    .unwrap();
    machine.ok(&[
        "server",
        "login",
        &harness.url,
        "--workload-token",
        token.to_str().unwrap(),
        "--label",
        "runner",
    ]);
    let (device, device_fingerprint) = machine.join(&org, &fingerprint);
    let devices = alice.ok(&["org", "devices"]).stdout;
    assert!(
        devices
            .lines()
            .any(|line| line.contains(&device) && line.contains("machine:ci")),
        "{devices}"
    );
    let unknown = alice.fails(&["org", "role", "machine:deploy", "member"]);
    assert!(unknown.contains("machine:deploy"), "{unknown}");
    alice.ok(&["org", "role", "machine:ci", "member"]);
    alice.ok(&[
        "org",
        "approve",
        &device,
        "--fingerprint",
        &device_fingerprint,
    ]);
    alice.ok(&["store", "grant", "machine:ci", "reader"]);
    let store = alice.store_id("team");
    machine.ok(&["store", "add-remote", &store, "--name", "team", "--default"]);
    assert_eq!(machine.ok(&["secret", "show", "deploy"]).stdout, "t0ken\n");
    let devices = alice.ok(&["org", "devices"]).stdout;
    assert!(
        devices.lines().any(|line| line.contains(&device)
            && line.contains("machine:ci")
            && line.contains("active")),
        "{devices}"
    );
    let log = alice.ok(&["audit", "log"]).stdout;
    assert!(log.contains("machine:ci"), "{log}");
}

fn exchange(installation: &Installation, message: &Value) -> Value {
    let mut child = installation
        .command(&["browser", "host"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let body = message.to_string();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(&u32::try_from(body.len()).unwrap().to_le_bytes())
        .unwrap();
    stdin.write_all(body.as_bytes()).unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut length = [0; 4];
    stdout.read_exact(&mut length).unwrap_or_else(|_| {
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        panic!("no reply: {stderr}");
    });
    let mut reply = vec![0; u32::from_le_bytes(length) as usize];
    stdout.read_exact(&mut reply).unwrap();
    drop(stdin);
    wait(&mut child, Duration::from_secs(20));
    serde_json::from_slice(&reply).unwrap()
}
