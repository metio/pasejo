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

/// Pre-populates a `TestCases` with the env every snapshot suite needs,
/// running the binary under `lang`. trycmd's `TestCases::env` is `&self`
/// (interior mutability), so we can apply settings here and hand the value
/// back by move.
fn test_cases(lang: &str) -> trycmd::TestCases {
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
        // Keep local state such as the browser release log inside the
        // case's sandbox instead of the developer's data directory.
        .env("PASEJO_DATA_DIR", "data")
        // Make every editor session fail immediately, so `secret edit` and
        // `secret add --multiline` never open the developer's editor. An
        // empty `VISUAL` is ignored, which leaves `EDITOR` in charge.
        .env("VISUAL", "")
        .env("EDITOR", "false")
        // Pin the locale so snapshots never depend on the developer's
        // environment. The i18n loader follows gettext precedence
        // (`LANGUAGE` > `LC_ALL` > `LC_MESSAGES` > `LANG`), so a host
        // `LANGUAGE` would win over the other two unless it is set here
        // as well. trycmd cannot remove an inherited variable, so all
        // three are overwritten.
        .env("LANGUAGE", lang)
        .env("LC_ALL", lang)
        .env("LANG", lang);
    cases
}

fn english_test_cases() -> trycmd::TestCases {
    test_cases("en")
}

#[test]
#[cfg(not(windows))]
fn cli_tests_browser() {
    english_test_cases().case("tests/cmd/browser/*/*.md");
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
    test_cases("de").case("tests/cmd-de/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_es() {
    test_cases("es").case("tests/cmd-es/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_fr() {
    test_cases("fr").case("tests/cmd-fr/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_it() {
    test_cases("it").case("tests/cmd-it/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_ja() {
    test_cases("ja").case("tests/cmd-ja/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_ko() {
    test_cases("ko").case("tests/cmd-ko/**/*.md");
}

#[test]
#[cfg(not(windows))]
fn cli_tests_zh() {
    test_cases("zh").case("tests/cmd-zh/**/*.md");
}
