// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;

mod common;
use common::{StoreScaffold, pasejo};

const SECOND_PUBLIC_KEY: &str = "age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";

fn spawn(scaffold: &StoreScaffold, args: &[&str]) -> Child {
    StdCommand::new(cargo_bin("pasejo"))
        .args(args)
        .env("PASEJO_CONFIG", &scaffold.config)
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env_remove("LANGUAGE")
        .env("LANG", "en")
        .env("LC_ALL", "en")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn has_exited_within(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn store_lock_path(scaffold: &StoreScaffold) -> PathBuf {
    scaffold.path().join("store.lock")
}

fn hold_store_lock(scaffold: &StoreScaffold) -> File {
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(store_lock_path(scaffold))
        .unwrap();
    lock.lock().unwrap();
    lock
}

fn set_pull_hook_touching(scaffold: &StoreScaffold, marker: &Path) {
    pasejo(&scaffold.config)
        .args(["hook", "set", "--pull"])
        .arg(format!("touch {}", marker.display()))
        .assert()
        .success();
}

#[test]
fn recipient_add_waits_for_the_store_lock() {
    let scaffold = StoreScaffold::new().store("test").identity().recipient();

    let lock = hold_store_lock(&scaffold);

    let mut child = spawn(
        &scaffold,
        &["recipient", "add", "--public-key", SECOND_PUBLIC_KEY],
    );
    let exited_under_lock = has_exited_within(&mut child, Duration::from_millis(500));
    drop(lock);
    let status = child.wait().unwrap();

    assert!(
        !exited_under_lock,
        "recipient add finished while another process held the store lock"
    );
    assert!(status.success());
    pasejo(&scaffold.config)
        .args(["recipient", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains(SECOND_PUBLIC_KEY));
}

#[test]
fn recipient_add_on_a_new_store_waits_for_the_store_lock() {
    let scaffold = StoreScaffold::new().store("test").identity();

    let lock = hold_store_lock(&scaffold);

    let mut child = spawn(
        &scaffold,
        &["recipient", "add", "--public-key", SECOND_PUBLIC_KEY],
    );
    let exited_under_lock = has_exited_within(&mut child, Duration::from_millis(500));
    drop(lock);
    let status = child.wait().unwrap();

    assert!(
        !exited_under_lock,
        "recipient add created the store while another process held the store lock"
    );
    assert!(status.success());
}

#[cfg(unix)]
fn assert_pull_hook_waits_for_the_store_lock(args: &[&str]) {
    let scaffold = StoreScaffold::new()
        .store("test")
        .identity()
        .recipient()
        .secret("alice", "hunter2\n");
    let marker = scaffold.path().join("pulled");
    set_pull_hook_touching(&scaffold, &marker);

    let lock = hold_store_lock(&scaffold);

    let mut child = spawn(&scaffold, args);
    let exited_under_lock = has_exited_within(&mut child, Duration::from_millis(500));
    let pulled_under_lock = marker.exists();
    drop(lock);
    let status = child.wait().unwrap();

    assert!(
        !pulled_under_lock,
        "{args:?} ran its pull hook while another process held the store lock"
    );
    assert!(
        !exited_under_lock,
        "{args:?} finished while another process held the store lock"
    );
    assert!(status.success(), "{args:?} exited with {status:?}");
    assert!(marker.exists(), "{args:?} never ran its pull hook");
}

#[cfg(unix)]
#[test]
fn secret_list_runs_its_pull_hook_under_the_store_lock() {
    assert_pull_hook_waits_for_the_store_lock(&["secret", "list"]);
}

#[cfg(unix)]
#[test]
fn store_decrypt_runs_its_pull_hook_under_the_store_lock() {
    assert_pull_hook_waits_for_the_store_lock(&["store", "decrypt", "--yes-i-know"]);
}

#[cfg(unix)]
#[test]
fn export_runs_its_pull_hook_under_the_store_lock() {
    assert_pull_hook_waits_for_the_store_lock(&["export", "bitwarden"]);
}

#[cfg(unix)]
#[test]
fn hook_run_pull_runs_under_the_store_lock() {
    assert_pull_hook_waits_for_the_store_lock(&["hook", "run", "--pull"]);
}

fn spawn_merge(scaffold: &StoreScaffold, current_version: &Path) -> Child {
    let store = scaffold.store_path();
    let store = store.to_str().unwrap();
    spawn(
        scaffold,
        &[
            "store",
            "merge",
            "--common-ancestor",
            store,
            "--current-version",
            current_version.to_str().unwrap(),
            "--other-version",
            store,
        ],
    )
}

#[test]
fn store_merge_into_the_registered_store_waits_for_the_store_lock() {
    let scaffold = StoreScaffold::new()
        .store("test")
        .identity()
        .recipient()
        .secret("alice", "hunter2\n");

    let lock = hold_store_lock(&scaffold);

    let mut child = spawn_merge(&scaffold, &scaffold.store_path());
    let exited_under_lock = has_exited_within(&mut child, Duration::from_millis(500));
    drop(lock);
    let status = child.wait().unwrap();

    assert!(
        !exited_under_lock,
        "store merge wrote the registered store while another process held the store lock"
    );
    assert!(status.success());
}

#[test]
fn store_merge_into_a_private_copy_does_not_wait_for_the_store_lock() {
    let scaffold = StoreScaffold::new()
        .store("test")
        .identity()
        .recipient()
        .secret("alice", "hunter2\n");
    let copy = scaffold.path().join(".merge_file_current");
    std::fs::copy(scaffold.store_path(), &copy).unwrap();

    let lock = hold_store_lock(&scaffold);

    let mut child = spawn_merge(&scaffold, &copy);
    let exited_under_lock = has_exited_within(&mut child, Duration::from_secs(5));
    drop(lock);
    let status = child.wait().unwrap();

    assert!(
        exited_under_lock,
        "store merge into a git temp file waited for the store lock"
    );
    assert!(status.success());
    assert!(!scaffold.path().join(".merge_file_current.lock").exists());
}
