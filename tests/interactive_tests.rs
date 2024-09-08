use assert_cmd::cargo::cargo_bin;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use rexpect::spawn;
use std::env;
use std::process::Command;

#[test]
fn insert_secret_with_age_key() -> anyhow::Result<()> {
    let cargo_package_name = env!("CARGO_PKG_NAME");
    let temp = assert_fs::TempDir::new()?;

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

    let key = age::x25519::Identity::generate();
    let pubkey = key.to_public();

    Command::cargo_bin(cargo_package_name)?
        .arg("recipient")
        .arg("add")
        .arg("--public-key")
        .arg(pubkey.to_string())
        .env("PASEJO_CONFIG", temp.path().join("config.toml"))
        .assert()
        .success()
        .code(0);

    let cmd_path = cargo_bin(cargo_package_name).into_os_string();
    let secret_name = "some/secret";
    env::set_var("PASEJO_CONFIG", temp.path().join("config.toml"));
    let mut process = spawn(
        &format!("{cmd_path:?} secret insert {secret_name}"),
        Some(30_000),
    )?;
    process.exp_string(&format!("Enter secret for {secret_name}:"))?;
    process.send_line("some-secret")?;
    process.exp_string("Confirmation:")?;
    process.send_line("some-secret")?;
    process.exp_eof()?;

    temp.child(format!("{secret_name}.age"))
        .assert(predicate::path::is_file());
    temp.close()?;
    Ok(())
}
