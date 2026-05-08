// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! The English snapshot suite is split into one `#[test]` per
//! `tests/cmd/<area>/` directory so that cargo test's "test ... has been
//! running for over 60 seconds" message names the slow area. A single big
//! `cli_tests` swallowed that signal — every regression looked the same.
//!
//! To pinpoint a slow individual case within an area, run:
//!
//! ```text
//! cargo test --test trycmd_tests -- --test-threads=1 --nocapture
//! ```
//!
//! and, if you need to see what each case actually produced:
//!
//! ```text
//! TRYCMD=dump cargo test --test trycmd_tests
//! ```
//!
//! which writes each case's stdout/stderr/files under `target/` for
//! inspection. `RUST_LOG=trycmd=debug,snapbox=debug` adds trycmd-internal
//! tracing when paired with `--nocapture`.
//!
//! When adding a new `tests/cmd/<area>/` directory, add a matching
//! `cli_tests_<area>` function below.

/// Pre-populates a `TestCases` with the env every English-locale suite
/// needs. trycmd's `TestCases::env` is `&self` (interior mutability), so
/// we can apply settings here and hand the value back by move.
fn english_test_cases() -> trycmd::TestCases {
    let cases = trycmd::TestCases::new();
    cases
        // Point to a test-local configuration file. Without this, all
        // tests would use the same configuration file which would make
        // the test suite flaky.
        .env("PASEJO_CONFIG", "config.toml")
        // Disable hook-throttle marker tracking entirely. Without this,
        // markers persisted in the user's data dir would carry over
        // between tests (and between `cargo test` runs), making the
        // suite flaky.
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        // Pin the locale so the binary's i18n loader always selects
        // English. Without this, snapshots would be checked against the
        // developer's system locale (e.g. de_DE) and break on machines
        // other than the author's. LC_ALL takes precedence over LANG /
        // LC_MESSAGES, so setting it is sufficient on its own, but we
        // set both for belt-and-braces.
        .env("LANG", "en")
        .env("LC_ALL", "en");
    cases
}

#[test]
#[cfg(not(windows))]
fn cli_tests_completion() {
    english_test_cases().case("tests/cmd/completion/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_config() {
    english_test_cases().case("tests/cmd/config/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_config_migrations() {
    english_test_cases().case("tests/cmd/config-migrations/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_export() {
    english_test_cases().case("tests/cmd/export/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_help() {
    english_test_cases().case("tests/cmd/help/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_hook() {
    english_test_cases().case("tests/cmd/hook/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_identity() {
    english_test_cases().case("tests/cmd/identity/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_otp() {
    english_test_cases().case("tests/cmd/otp/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_recipient() {
    english_test_cases().case("tests/cmd/recipient/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_secret() {
    english_test_cases().case("tests/cmd/secret/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_store() {
    english_test_cases().case("tests/cmd/store/*/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_docs() {
    english_test_cases().case("docs/commands/pasejo-cmd-*.md");
}

/// Translation gates: small, targeted suites that run the binary under a
/// non-default locale and assert the translated output. These are *not*
/// mirrors of the English coverage — each only covers a representative
/// slice of commands that exercise i18n message families, enough to
/// catch broken `.ftl` files, missing message ids, or argument-name
/// mismatches between Rust and Fluent. Add a new case to each suite
/// whenever you add a new message family to `cli/i18n.rs`; the English
/// `cli_tests_*` functions stay comprehensive.
#[test]
#[cfg(not(windows))]
fn cli_tests_de() {
    trycmd::TestCases::new()
        .case("tests/cmd-de/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "de")
        .env("LC_ALL", "de");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_es() {
    trycmd::TestCases::new()
        .case("tests/cmd-es/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "es")
        .env("LC_ALL", "es");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_fr() {
    trycmd::TestCases::new()
        .case("tests/cmd-fr/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "fr")
        .env("LC_ALL", "fr");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_it() {
    trycmd::TestCases::new()
        .case("tests/cmd-it/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "it")
        .env("LC_ALL", "it");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_ja() {
    trycmd::TestCases::new()
        .case("tests/cmd-ja/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "ja")
        .env("LC_ALL", "ja");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_ko() {
    trycmd::TestCases::new()
        .case("tests/cmd-ko/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "ko")
        .env("LC_ALL", "ko");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_zh() {
    trycmd::TestCases::new()
        .case("tests/cmd-zh/**/*.md")
        .env("PASEJO_CONFIG", "config.toml")
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        .env("LANG", "zh")
        .env("LC_ALL", "zh");
}
