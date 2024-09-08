use std::env;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use rexpect::session::PtySession;
use rexpect::spawn;
use serial_test::serial;
use ssh_key::rand_core::OsRng;
use ssh_key::{Algorithm, PrivateKey};

#[test]
#[serial]
fn insert_secret_with_age_key() -> anyhow::Result<()> {
    let key = age::x25519::Identity::generate();
    insert_secret_with_key(&key.to_public().to_string())?;
    Ok(())
}

#[test]
#[serial]
fn insert_secret_with_ssh_ed25519_key() -> anyhow::Result<()> {
    let key = PrivateKey::random(&mut OsRng::default(), Algorithm::Ed25519)?;
    insert_secret_with_key(&key.public_key().to_string())?;
    Ok(())
}

#[test]
#[serial]
#[ignore] // it takes too long to run - execute with: cargo test -- --ignored
fn insert_secret_with_ssh_rsa_key() -> anyhow::Result<()> {
    let key = PrivateKey::random(&mut OsRng::default(), Algorithm::Rsa { hash: None })?;
    insert_secret_with_key(&key.public_key().to_string())?;
    Ok(())
}

fn insert_secret_with_key(public_key: &str) -> anyhow::Result<()> {
    let secret_name = "some/secret";
    run_command(
        &format!("secret insert {secret_name}"),
        public_key,
        |mut process, temp| {
            process.exp_string(&format!("Enter secret for {secret_name}:"))?;
            process.send_line("some-secret")?;
            process.exp_string("Confirmation:")?;
            process.send_line("some-secret")?;
            process.exp_eof()?;

            temp.child(format!("{secret_name}.age"))
                .assert(predicate::path::is_file());
            Ok(())
        },
    )?;
    Ok(())
}

fn run_command<T>(command: &str, public_key: &str, test_case: T) -> anyhow::Result<()>
where
    T: FnOnce(PtySession, TempDir) -> anyhow::Result<()>,
{
    let cargo_package_name = env!("CARGO_PKG_NAME");
    let temp = TempDir::new()?;

    Command::cargo_bin(cargo_package_name)?
        .arg("store")
        .arg("init")
        .arg("--path")
        .arg(temp.path())
        .arg("--name")
        .arg("test")
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
    env::set_var("PASEJO_CONFIG", temp.path().join("config.toml"));

    let process = spawn(&format!("{cmd_path:?} {command}"), Some(30_000))?;
    test_case(process, temp)?;

    Ok(())
}
