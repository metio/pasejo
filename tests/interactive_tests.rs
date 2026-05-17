// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

#[cfg(unix)]
mod interactive_tests {
    use std::env;
    use std::io::Write;
    use std::process::Command;

    use age::cli_common::file_io;
    use age::secrecy::ExposeSecret;
    use assert_cmd::cargo::cargo_bin;
    use assert_cmd::prelude::*;
    use assert_fs::TempDir;
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
            &format!("secret add {secret_name}"),
            public_key,
            private_key,
            |mut process, temp| {
                process.exp_string(&format!("Enter secret for {secret_name}:"))?;
                process.send_line(secret_text)?;
                process.exp_string("Confirmation:")?;
                process.send_line(secret_text)?;
                process.exp_eof()?;

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
        let temp = prepare_environment(public_key, private_key)?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();
        let process = spawn(&format!("{cmd_path:?} {command}"), Some(30_000))?;
        test_case(process, temp)?;
        Ok(())
    }

    fn prepare_environment(public_key: &str, private_key: &str) -> anyhow::Result<TempDir> {
        // Pin the locale so every child process picks up English from the
        // i18n loader, regardless of the developer's shell locale. These
        // tests are #[serial] so the global env mutation is race-free.
        unsafe {
            env::set_var("LANG", "en");
            env::set_var("LC_ALL", "en");
        }

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
            .arg("add")
            .arg("--path")
            .arg(temp.path().join("pasejo-store"))
            .arg("--name")
            .arg("test")
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

        unsafe {
            env::set_var("PASEJO_CONFIG", temp.path().join("config.toml"));
        }

        Ok(temp)
    }

    fn add_secret_via_pty(
        cmd_path: &std::ffi::OsString,
        name: &str,
        value: &str,
    ) -> anyhow::Result<()> {
        let mut process = spawn(&format!("{cmd_path:?} secret add {name}"), Some(30_000))?;
        process.exp_string(&format!("Enter secret for {name}:"))?;
        process.send_line(value)?;
        process.exp_string("Confirmation:")?;
        process.send_line(value)?;
        process.exp_eof()?;
        Ok(())
    }

    #[test]
    #[serial]
    fn grep_prints_only_matching_keys_by_default() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            &key.to_string().expose_secret(),
        )?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();

        add_secret_via_pty(&cmd_path, "bar", "alpha-charlie")?;
        add_secret_via_pty(&cmd_path, "baz", "delta")?;
        add_secret_via_pty(&cmd_path, "foo", "alpha-bravo")?;

        // BTreeMap iteration order is lexicographic — bar then foo match.
        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("alpha")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq("bar\nfoo\n"))
            .success()
            .code(0);

        Ok(())
    }

    #[test]
    #[serial]
    fn grep_with_show_values_prints_keys_and_values() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            &key.to_string().expose_secret(),
        )?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();

        add_secret_via_pty(&cmd_path, "bar", "alpha-charlie")?;
        add_secret_via_pty(&cmd_path, "baz", "delta")?;
        add_secret_via_pty(&cmd_path, "foo", "alpha-bravo")?;

        let expected = "bar:\nalpha-charlie\nfoo:\nalpha-bravo\n";

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("--show-values")
            .arg("alpha")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq(expected))
            .success()
            .code(0);

        // -V is the short form of --show-values.
        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("-V")
            .arg("alpha")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq(expected))
            .success()
            .code(0);

        Ok(())
    }

    #[test]
    #[serial]
    fn grep_regex_mode_respects_show_values() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            &key.to_string().expose_secret(),
        )?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();

        add_secret_via_pty(&cmd_path, "bar", "alpha-charlie")?;
        add_secret_via_pty(&cmd_path, "baz", "delta")?;
        add_secret_via_pty(&cmd_path, "foo", "alpha-bravo")?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("--regex")
            .arg(r"^alpha-.+$")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq("bar\nfoo\n"))
            .success()
            .code(0);

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("--regex")
            .arg("--show-values")
            .arg(r"^alpha-.+$")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq("bar:\nalpha-charlie\nfoo:\nalpha-bravo\n"))
            .success()
            .code(0);

        Ok(())
    }

    #[test]
    #[serial]
    fn grep_with_no_matches_prints_nothing() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            &key.to_string().expose_secret(),
        )?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();

        add_secret_via_pty(&cmd_path, "foo", "alpha-bravo")?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("nothing-here")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(predicate::eq(""))
            .success()
            .code(0);

        Ok(())
    }

    #[test]
    #[serial]
    fn grep_with_invalid_regex_returns_error() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            &key.to_string().expose_secret(),
        )?;
        let cmd_path = cargo_bin(env!("CARGO_PKG_NAME")).into_os_string();

        add_secret_via_pty(&cmd_path, "foo", "alpha")?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("grep")
            .arg("--regex")
            .arg("[unclosed")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .failure();

        Ok(())
    }
}
