// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::env;
use std::io::Write;
use std::process::Command;

use age::cli_common::file_io;
use age::secrecy::ExposeSecret;
use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;
use rexpect::session::PtySession;
use rexpect::spawn;
use serial_test::serial;
use ssh_key::rand_core::OsRng;
use ssh_key::{Algorithm, LineEnding, PrivateKey};

#[test]
#[serial]
fn insert_secret_with_age_key() -> anyhow::Result<()> {
    let key = age::x25519::Identity::generate();
    insert_secret_with_key(
        &key.to_public().to_string(),
        &key.to_string().expose_secret(),
    )?;
    Ok(())
}

#[test]
#[serial]
fn insert_secret_with_ssh_ed25519_key() -> anyhow::Result<()> {
    let key = PrivateKey::random(&mut OsRng::default(), Algorithm::Ed25519)?;
    insert_secret_with_key(
        &key.public_key().to_string(),
        &key.to_openssh(LineEnding::default())?,
    )?;
    Ok(())
}

#[test]
#[serial]
#[ignore] // it takes too long to run - execute with: cargo test -- --ignored
fn insert_secret_with_ssh_rsa_key() -> anyhow::Result<()> {
    let key = PrivateKey::random(&mut OsRng::default(), Algorithm::Rsa { hash: None })?;
    insert_secret_with_key(
        &key.public_key().to_string(),
        &key.to_openssh(LineEnding::default())?,
    )?;
    Ok(())
}

fn insert_secret_with_key(public_key: &str, private_key: &str) -> anyhow::Result<()> {
    let secret_name = "some/secret";
    let secret_text = "here is a very secret text";
    let qrcode = qr2term::generate_qr_string(secret_text)?;
    run_command(
        &format!("secret insert {secret_name}"),
        public_key,
        private_key,
        |mut process, temp| {
            process.exp_string(&format!("Enter secret for {secret_name}:"))?;
            process.send_line(secret_text)?;
            process.exp_string("Confirmation:")?;
            process.send_line(secret_text)?;
            process.exp_eof()?;

            temp.child(format!("{secret_name}.age"))
                .assert(predicate::path::is_file());

            Command::cargo_bin(env!("CARGO_PKG_NAME"))?
                .arg("secret")
                .arg("show")
                .arg(secret_name)
                .env("PASEJO_CONFIG", temp.path().join("config.toml"))
                .assert()
                .stdout(predicate::eq(format!("{secret_text}\n")))
                .success()
                .code(0);

            Command::cargo_bin(env!("CARGO_PKG_NAME"))?
                .arg("secret")
                .arg("show")
                .arg("--qrcode")
                .arg(secret_name)
                .env("PASEJO_CONFIG", temp.path().join("config.toml"))
                .assert()
                .stdout(predicate::eq(qrcode))
                .success()
                .code(0);

            Ok(())
        },
    )?;
    Ok(())
}

fn run_command<T>(
    command: &str,
    public_key: &str,
    private_key: &str,
    test_case: T,
) -> anyhow::Result<()>
where
    T: FnOnce(PtySession, TempDir) -> anyhow::Result<()>,
{
    let cargo_package_name = env!("CARGO_PKG_NAME");
    let temp = TempDir::new()?;
    let identity_file = temp.path().join("private-key");
    let mut output = file_io::OutputWriter::new(
        identity_file.to_str().map(String::from),
        false,
        file_io::OutputFormat::Text,
        0o600,
        false,
    )?;
    writeln!(output, "{private_key}")?;

    Command::cargo_bin(cargo_package_name)?
        .arg("store")
        .arg("init")
        .arg("--path")
        .arg(temp.path())
        .arg("--name")
        .arg("test")
        .arg("--vcs")
        .arg("none")
        .env("PASEJO_CONFIG", temp.path().join("config.toml"))
        .assert()
        .success()
        .code(0);

    Command::cargo_bin(cargo_package_name)?
        .arg("identity")
        .arg("add")
        .arg("--file")
        .arg(&identity_file)
        .env("PASEJO_CONFIG", temp.path().join("config.toml"))
        .assert()
        .success()
        .code(0);

    Command::cargo_bin(cargo_package_name)?
        .arg("recipient")
        .arg("add")
        .arg("--public-key")
        .arg(public_key)
        .env("PASEJO_CONFIG", temp.path().join("config.toml"))
        .assert()
        .success()
        .code(0);

    let cmd_path = cargo_bin(cargo_package_name).into_os_string();
    unsafe {
        env::set_var("PASEJO_CONFIG", temp.path().join("config.toml"));
    }

    let process = spawn(&format!("{cmd_path:?} {command}"), Some(30_000))?;
    test_case(process, temp)?;

    Ok(())
}
