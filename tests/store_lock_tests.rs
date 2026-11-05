// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs::File;
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

#[test]
fn recipient_add_waits_for_the_store_lock() {
    let scaffold = StoreScaffold::new().store("test").identity().recipient();

    let lock = File::open(scaffold.store_path()).unwrap();
    lock.lock().unwrap();

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
