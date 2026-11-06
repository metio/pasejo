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
    use serial_test::serial;
    use ssh_key::rand_core::OsRng;
    use ssh_key::{Algorithm, LineEnding, PrivateKey};

    #[test]
    #[serial]
    fn insert_secret_with_age_key() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        insert_secret_with_key(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;
        Ok(())
    }

    #[test]
    #[serial]
    fn insert_secret_with_ssh_ed25519_key() -> anyhow::Result<()> {
        let key = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
        insert_secret_with_key(
            &key.public_key().to_string(),
            &key.to_openssh(LineEnding::default())?,
        )?;
        Ok(())
    }

    #[test]
    #[serial]
    #[ignore = "slow: RSA key generation; run with: cargo test -- --ignored"]
    fn insert_secret_with_ssh_rsa_key() -> anyhow::Result<()> {
        let key = PrivateKey::random(&mut OsRng, Algorithm::Rsa { hash: None })?;
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
        let mut spawn_cmd = std::process::Command::new(cargo_bin(env!("CARGO_PKG_NAME")));
        spawn_cmd.args(command.split_whitespace());
        let process = rexpect::session::spawn_command(spawn_cmd, Some(30_000))?;
        test_case(process, temp)?;
        Ok(())
    }

    fn prepare_environment(public_key: &str, private_key: &str) -> anyhow::Result<TempDir> {
        // Pin the locale so every child process picks up English from the
        // i18n loader, regardless of the developer's shell locale. A host
        // `LANGUAGE` outranks `LC_ALL`, so it is removed. These tests are
        // #[serial] so the global env mutation is race-free.
        unsafe {
            env::remove_var("LANGUAGE");
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
        let mut spawn_cmd = std::process::Command::new(cmd_path);
        spawn_cmd.args(["secret", "add", name]);
        let mut process = rexpect::session::spawn_command(spawn_cmd, Some(30_000))?;
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
            key.to_string().expose_secret(),
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
            key.to_string().expose_secret(),
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
            key.to_string().expose_secret(),
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
            key.to_string().expose_secret(),
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
    fn browser_query_finds_url_field_match() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;

        // The non-TTY path of `secret add` reads the entire stdin verbatim
        // as the secret value, which is the only way to inject a multi-line
        // secret with a `url:` field through the CLI — the trycmd suite
        // cannot do this because its `$ command` syntax has no stdin
        // mechanism, and `--multiline` opens an editor that would require
        // PTY scripting.
        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("add")
            .arg("alice")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .write_stdin("hunter2\nurl: https://example.com\nlogin: alice@example.com\n")
            .assert()
            .success()
            .code(0);

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("browser")
            .arg("query")
            .arg("example.com")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(
                predicate::str::contains(r#""name":"alice""#)
                    .and(predicate::str::contains(r#""url":"https://example.com""#))
                    .and(predicate::str::contains(
                        r#""username":"alice@example.com""#,
                    ))
                    .and(predicate::str::contains(r#""password":"hunter2""#))
                    .and(predicate::str::contains(r#""match_kind":"url_field""#))
                    .and(predicate::str::contains(r#""score":1.0"#)),
            )
            .success()
            .code(0);

        Ok(())
    }

    #[test]
    #[serial]
    fn browser_query_matches_subdomain_via_registrable_domain() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;

        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("secret")
            .arg("add")
            .arg("alice")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .write_stdin("hunter2\nurl: https://www.example.com\n")
            .assert()
            .success();

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .arg("browser")
            .arg("query")
            .arg("mail.example.com")
            .env("PASEJO_CONFIG", temp.path().join("config.toml"))
            .assert()
            .stdout(
                predicate::str::contains(r#""match_kind":"url_field""#)
                    .and(predicate::str::contains(r#""score":0.9"#)),
            )
            .success();

        Ok(())
    }

    #[test]
    #[serial]
    fn browser_host_refuses_to_start_with_tty_stdin() -> anyhow::Result<()> {
        // `rexpect::session::spawn_command` attaches a PTY, so the host
        // child sees `stdin().is_terminal() == true` and must refuse to
        // start. The trycmd suite cannot exercise this because trycmd
        // always pipes stdin.
        let temp = assert_fs::TempDir::new()?;
        let config = temp.path().join("config.toml");

        let mut spawn_cmd = std::process::Command::new(cargo_bin(env!("CARGO_PKG_NAME")));
        spawn_cmd
            .args(["browser", "host"])
            .env("PASEJO_CONFIG", &config)
            .env_remove("LANGUAGE")
            .env("LANG", "en")
            .env("LC_ALL", "en");
        let mut process = rexpect::session::spawn_command(spawn_cmd, Some(5_000))?;
        process.exp_string("Browser host requires a piped stdin")?;
        process.exp_eof()?;
        Ok(())
    }

    #[test]
    #[serial]
    fn grep_with_invalid_regex_returns_error() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
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

    /// Writes an `$EDITOR` stand-in that records where it was asked to edit,
    /// keeps a hard link to the edited inode so the cleanup can be observed
    /// after the file is unlinked, replaces the content and exits with
    /// `exit_code`.
    fn fake_editor(temp: &TempDir, exit_code: i32) -> anyhow::Result<std::path::PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        let markers = temp.path().join("markers");
        std::fs::create_dir_all(&markers)?;
        let script = temp.path().join("fake-editor");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\n\
                 dirname \"$1\" > '{markers}/dir'\n\
                 cat \"$1\" > '{markers}/before'\n\
                 ln \"$1\" '{markers}/link'\n\
                 printf 'edited-password\\nurl: https://example.com\\n\\n' > \"$1\"\n\
                 exit {exit_code}\n",
                markers = markers.display(),
            ),
        )?;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))?;
        Ok(script)
    }

    fn runtime_dir(temp: &TempDir) -> anyhow::Result<std::path::PathBuf> {
        let runtime = temp.path().join("runtime");
        std::fs::create_dir_all(&runtime)?;
        Ok(runtime)
    }

    fn assert_editor_file_scrubbed(
        temp: &TempDir,
        runtime: &std::path::Path,
    ) -> anyhow::Result<()> {
        assert_eq!(
            std::fs::read_dir(runtime)?.count(),
            0,
            "editor file left in {}",
            runtime.display()
        );
        let leftover = std::fs::read(temp.path().join("markers/link"))?;
        assert!(!leftover.is_empty());
        assert!(
            leftover.iter().all(|byte| *byte == 0),
            "editor file was not overwritten before removal: {leftover:?}"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn secret_edit_uses_private_runtime_dir_and_scrubs_the_file() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;
        let editor = fake_editor(&temp, 0)?;
        let runtime = runtime_dir(&temp)?;

        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "add", "alice"])
            .write_stdin("original-password\nlogin: alice")
            .assert()
            .success();

        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "edit", "alice"])
            .env("EDITOR", &editor)
            .env_remove("VISUAL")
            .env("XDG_RUNTIME_DIR", &runtime)
            .assert()
            .success();

        assert_eq!(
            std::fs::read_to_string(temp.path().join("markers/dir"))?.trim_end(),
            runtime.display().to_string()
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("markers/before"))?,
            "original-password\nlogin: alice"
        );
        assert_editor_file_scrubbed(&temp, &runtime)?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "show", "alice"])
            .assert()
            .stdout(predicate::eq("edited-password\nurl: https://example.com\n"))
            .success();
        Ok(())
    }

    #[test]
    #[serial]
    fn secret_edit_keeps_the_secret_and_scrubs_the_file_when_the_editor_fails() -> anyhow::Result<()>
    {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;
        let editor = fake_editor(&temp, 3)?;
        let runtime = runtime_dir(&temp)?;

        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "add", "alice"])
            .write_stdin("original-password")
            .assert()
            .success();

        assert_cmd::Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "edit", "alice"])
            .env("EDITOR", &editor)
            .env_remove("VISUAL")
            .env("XDG_RUNTIME_DIR", &runtime)
            .assert()
            .failure()
            .stderr(predicate::str::contains("did not exit successfully"));

        assert_editor_file_scrubbed(&temp, &runtime)?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "show", "alice"])
            .assert()
            .stdout(predicate::eq("original-password\n"))
            .success();
        Ok(())
    }

    #[test]
    #[serial]
    fn secret_add_multiline_uses_private_runtime_dir_and_scrubs_the_file() -> anyhow::Result<()> {
        let key = age::x25519::Identity::generate();
        let temp = prepare_environment(
            &key.to_public().to_string(),
            key.to_string().expose_secret(),
        )?;
        let editor = fake_editor(&temp, 0)?;
        let runtime = runtime_dir(&temp)?;

        let mut spawn_cmd = std::process::Command::new(cargo_bin(env!("CARGO_PKG_NAME")));
        spawn_cmd
            .args(["secret", "add", "--multiline", "alice"])
            .env("EDITOR", &editor)
            .env_remove("VISUAL")
            .env("XDG_RUNTIME_DIR", &runtime);
        let mut process = rexpect::session::spawn_command(spawn_cmd, Some(10_000))?;
        process.exp_eof()?;

        assert_eq!(
            std::fs::read_to_string(temp.path().join("markers/dir"))?.trim_end(),
            runtime.display().to_string()
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("markers/before"))?,
            ""
        );
        assert_editor_file_scrubbed(&temp, &runtime)?;

        Command::cargo_bin(env!("CARGO_PKG_NAME"))?
            .args(["secret", "show", "alice"])
            .assert()
            .stdout(predicate::eq("edited-password\nurl: https://example.com\n"))
            .success();
        Ok(())
    }
}
