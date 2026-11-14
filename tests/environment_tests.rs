// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! `secret show --field`, `secret resolve` and `exec` against real stores.
//! The stores hold multi-line secrets, which only `secret add`'s piped
//! stdin can create, so these cases live here rather than in trycmd.

mod common;

use common::{StoreScaffold, TEST_PUBLIC_KEY, pasejo};
use predicates::prelude::*;

const DATABASE: &str = "hunter2\nusername:   alice\nhost: db.example.com";

fn scaffold() -> StoreScaffold {
    StoreScaffold::new()
        .store("primary")
        .identity()
        .recipient()
        .secret("database", DATABASE)
        .secret("api/token", "tok-123456")
}

/// `sh -c script` under the C locale. The test locale `en` exists only for
/// pasejo's own catalogs, and a shell would warn that it cannot set it.
#[cfg(unix)]
fn sh(script: &str) -> [&str; 5] {
    ["env", "LC_ALL=C", "sh", "-c", script]
}

/// Registers a second store named `other` holding `other-secret`.
fn add_other_store(scaffold: &StoreScaffold) {
    pasejo(&scaffold.config)
        .args(["store", "add", "--name", "other", "--path"])
        .arg(scaffold.path().join("other-store"))
        .assert()
        .success();
    pasejo(&scaffold.config)
        .args(["identity", "add", "--store", "other", "--file"])
        .arg(scaffold.path().join("identity"))
        .assert()
        .success();
    pasejo(&scaffold.config)
        .args(["recipient", "add", "--store", "other", "--public-key"])
        .arg(TEST_PUBLIC_KEY)
        .assert()
        .success();
    pasejo(&scaffold.config)
        .args(["secret", "add", "--store", "other", "other-secret"])
        .write_stdin("from-the-other-store")
        .assert()
        .success();
}

#[test]
fn secret_show_field_prints_the_trimmed_value_of_the_field() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args(["secret", "show", "--field", "username", "database"])
        .assert()
        .success()
        .stdout("alice\n");
}

#[test]
fn secret_show_field_fails_for_a_missing_field() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args(["secret", "show", "--field", "port", "database"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "Secret 'database' has no field 'port'",
        ));
}

#[test]
fn secret_show_field_reports_a_missing_field_in_german() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .env("LANG", "de")
        .env("LC_ALL", "de")
        .args(["secret", "show", "--field", "port", "database"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Das Geheimnis 'database' hat kein Feld 'port'",
        ));
}

#[test]
fn secret_resolve_prints_every_requested_value_across_stores() {
    let scaffold = scaffold();
    add_other_store(&scaffold);
    pasejo(&scaffold.config)
        .args(["secret", "resolve"])
        .write_stdin(
            r#"{
                "PASSWORD": {"secret": "database"},
                "USER": {"secret": "database", "field": "username"},
                "HOST": {"secret": "database", "line": -1},
                "TOKEN": {"secret": "api/token"},
                "OTHER": {"secret": "other-secret", "store": "other"}
            }"#,
        )
        .assert()
        .success()
        .stdout(
            "{\"HOST\":\"host: db.example.com\",\"OTHER\":\"from-the-other-store\",\
             \"PASSWORD\":\"hunter2\",\"TOKEN\":\"tok-123456\",\"USER\":\"alice\"}\n",
        );
}

#[test]
fn secret_resolve_prints_nothing_when_one_reference_fails() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args(["secret", "resolve"])
        .write_stdin(r#"{"TOKEN": {"secret": "api/token"}, "PORT": {"secret": "database", "field": "port"}}"#)
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "Cannot resolve the value of 'PORT'",
        ))
        .stderr(predicate::str::contains(
            "Secret 'database' has no field 'port'",
        ));
}

#[test]
fn secret_resolve_refuses_malformed_input() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args(["secret", "resolve"])
        .write_stdin(r#"{"TOKEN": {"secret": "api/token", "line": 1, "field": "x"}}"#)
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "Cannot read the secret references from stdin",
        ));
}

#[test]
fn secret_resolve_names_an_unknown_store() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args(["secret", "resolve"])
        .write_stdin(r#"{"TOKEN": {"secret": "api/token", "store": "missing"}}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing"));
}

#[cfg(unix)]
#[test]
fn exec_runs_the_command_with_the_values_in_its_environment() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args([
            "exec",
            "--env",
            "TOKEN=api/token",
            "--env-field",
            "DB_USER=database=username",
            "--",
        ])
        .args(sh(r#"printf '%s|%s' "$TOKEN" "$DB_USER""#))
        .assert()
        .success()
        .stdout("tok-123456|alice");
}

#[cfg(unix)]
#[test]
fn exec_takes_values_from_the_selected_store() {
    let scaffold = scaffold();
    add_other_store(&scaffold);
    pasejo(&scaffold.config)
        .args([
            "exec",
            "--store",
            "other",
            "--env",
            "VALUE=other-secret",
            "--",
        ])
        .args(sh(r#"printf '%s' "$VALUE""#))
        .assert()
        .success()
        .stdout("from-the-other-store");
}

#[cfg(unix)]
#[test]
fn exec_exits_with_the_exit_code_of_the_command() {
    let scaffold = scaffold();
    for redact in [false, true] {
        let mut command = pasejo(&scaffold.config);
        command.args(["exec", "--env", "TOKEN=api/token"]);
        if redact {
            command.arg("--redact");
        }
        command
            .arg("--")
            .args(sh("exit 7"))
            .assert()
            .code(7)
            .stderr("");
    }
}

#[cfg(unix)]
#[test]
fn exec_does_not_run_the_command_when_a_value_is_missing() {
    let scaffold = scaffold();
    let marker = scaffold.path().join("ran");
    pasejo(&scaffold.config)
        .args(["exec", "--env", "TOKEN=missing/secret", "--", "touch"])
        .arg(&marker)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot resolve the value of 'TOKEN'",
        ))
        .stderr(predicate::str::contains(
            "No secret found at 'missing/secret'",
        ));
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn exec_redact_replaces_the_values_in_stdout_and_stderr() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args([
            "exec",
            "--redact",
            "--env",
            "TOKEN=api/token",
            "--env-field",
            "DB_USER=database=username",
            "--",
        ])
        .args(sh(
            r#"echo "token=$TOKEN user=$DB_USER"; echo "err $TOKEN" >&2"#,
        ))
        .assert()
        .success()
        .stdout("token=${TOKEN} user=${DB_USER}\n")
        .stderr("err ${TOKEN}\n");
}

#[cfg(unix)]
#[test]
fn exec_redact_warns_about_a_value_too_short_to_redact() {
    let scaffold = scaffold().secret("short", "ab");
    pasejo(&scaffold.config)
        .args(["exec", "--redact", "--env", "SHORT=short", "--"])
        .args(sh(r#"printf '%s' "$SHORT""#))
        .assert()
        .success()
        .stdout("ab")
        .stderr(predicate::str::contains(
            "The value of SHORT is shorter than 3 characters and is not redacted",
        ));
}

#[test]
fn exec_reports_a_command_that_cannot_be_started() {
    let scaffold = scaffold();
    pasejo(&scaffold.config)
        .args([
            "exec",
            "--env",
            "TOKEN=api/token",
            "--",
            "pasejo-no-such-command",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to run command pasejo-no-such-command",
        ));
}
