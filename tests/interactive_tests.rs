use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use predicates::prelude::*;
use rexpect::session::PtySession;
use rexpect::spawn;
use ssh_key::rand_core::OsRng;
use ssh_key::{Algorithm, PrivateKey};
use std::env;
use std::process::Command;

#[test]
fn insert_secret_with_age_key() -> anyhow::Result<()> {
    let secret_name = "some/secret";
    let key = age::x25519::Identity::generate();
    run_command(
        &format!("secret insert {secret_name}"),
        &key.to_public().to_string(),
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

#[test]
fn insert_secret_with_ssh_key() -> anyhow::Result<()> {
    let secret_name = "some/secret";
    let key = PrivateKey::random(&mut OsRng::default(), Algorithm::Ed25519)?;
    run_command(
        &format!("secret insert {secret_name}"),
        &key.public_key().to_string(),
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
