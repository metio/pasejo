// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Shared scaffolding for the binary-spawning integration tests.
//!
//! Lives under `tests/common/` (a `mod.rs`, so cargo does not treat it as a
//! test target of its own) and is pulled into each integration crate with
//! `mod common;`. Each crate uses only the parts it needs, so unused items
//! are expected — hence the crate-wide `dead_code` allowance.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use assert_cmd::Command as AssertCommand;
use assert_fs::TempDir;

/// A fixed age identity (private key + its public key), so the tests do not
/// depend on the `age` crate's key-generation API — the encrypt/decrypt path
/// is exercised identically with a known value. The same pair backs the
/// `tests/cmd/export/bitwarden` fixtures.
pub const TEST_PRIVATE_KEY: &str = "# public key: age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd\n\
     AGE-SECRET-KEY-14LGKWZMHDDWMLCL5EUU2FSQNEX33DZYR3F8VEPZSA670QRC3DA3SNSD535";
pub const TEST_PUBLIC_KEY: &str = "age1fdalwkzzv0vztxm08gmh6geddk095x0ww9ztfjdtlf673ynwugqssxx8qd";

/// A `pasejo` command pre-wired with the test config path and a pinned C
/// locale, so assertions see English output regardless of the host
/// environment.
pub fn pasejo(config: &Path) -> AssertCommand {
    let mut command = AssertCommand::cargo_bin("pasejo").unwrap();
    command
        .env("PASEJO_CONFIG", config)
        .env("LANG", "en")
        .env("LC_ALL", "en");
    command
}

/// Builds a throwaway store world in a temp directory by driving the real
/// CLI, so the on-disk state matches what a user would create. Each step is
/// chainable; the world is torn down when the returned value is dropped.
pub struct StoreScaffold {
    pub temp: TempDir,
    pub config: PathBuf,
}

impl StoreScaffold {
    /// A fresh temp dir with a test-local `config.toml` path (nothing
    /// registered yet).
    #[must_use]
    pub fn new() -> Self {
        let temp = TempDir::new().unwrap();
        let config = temp.path().join("config.toml");
        Self { temp, config }
    }

    /// The sandbox directory.
    #[must_use]
    pub fn path(&self) -> &Path {
        self.temp.path()
    }

    /// The encrypted store file's path (`<temp>/store`), the location every
    /// `store(...)` call registers.
    #[must_use]
    pub fn store_path(&self) -> PathBuf {
        self.temp.path().join("store")
    }

    /// Register a store named `name` at `<temp>/store`.
    #[must_use]
    pub fn store(self, name: &str) -> Self {
        pasejo(&self.config)
            .args(["store", "add", "--name", name, "--path"])
            .arg(self.store_path())
            .assert()
            .success();
        self
    }

    /// Write the fixed identity key file (mode `0600` on Unix) and register
    /// it on the (default) store.
    #[must_use]
    pub fn identity(self) -> Self {
        let identity_file = self.temp.path().join("identity");
        std::fs::write(&identity_file, TEST_PRIVATE_KEY).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&identity_file, std::fs::Permissions::from_mode(0o600))
                .unwrap();
        }
        pasejo(&self.config)
            .args(["identity", "add", "--file"])
            .arg(&identity_file)
            .assert()
            .success();
        self
    }

    /// Register the fixed public key as a recipient on the (default) store.
    #[must_use]
    pub fn recipient(self) -> Self {
        pasejo(&self.config)
            .args(["recipient", "add", "--public-key", TEST_PUBLIC_KEY])
            .assert()
            .success();
        self
    }

    /// Add a secret named `name` with `value` piped on stdin.
    #[must_use]
    pub fn secret(self, name: &str, value: &str) -> Self {
        pasejo(&self.config)
            .args(["secret", "add", name])
            .write_stdin(value.to_owned())
            .assert()
            .success();
        self
    }
}

impl Default for StoreScaffold {
    fn default() -> Self {
        Self::new()
    }
}
