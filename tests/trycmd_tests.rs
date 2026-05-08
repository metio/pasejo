// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

#[test]
#[cfg(not(windows))]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/completion/*.md")
        .case("tests/cmd/config/*/*.md")
        .case("tests/cmd/config-migrations/*/*.md")
        .case("tests/cmd/export/*/*.md")
        .case("tests/cmd/help/*.md")
        .case("tests/cmd/hook/*/*.md")
        .case("tests/cmd/identity/*/*.md")
        .case("tests/cmd/otp/*/*.md")
        .case("tests/cmd/recipient/*/*.md")
        .case("tests/cmd/secret/*/*.md")
        .case("tests/cmd/store/*/*.md")
        .case("docs/commands/pasejo-cmd-*.md")
        // Point to a test-local configuration file. Without this,
        // all tests would use the same configuration file which would
        // make the test suite flaky
        .env("PASEJO_CONFIG", "config.toml")
        // Disable hook-throttle marker tracking entirely. Without this,
        // markers persisted in the user's data dir would carry over between
        // tests (and between `cargo test` runs), making the suite flaky.
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1")
        // Pin the locale so the binary's i18n loader always selects English.
        // Without this, snapshots would be checked against the developer's
        // system locale (e.g. de_DE) and break on machines other than the
        // author's. LC_ALL takes precedence over LANG / LC_MESSAGES, so
        // setting it is sufficient on its own, but we set both for
        // belt-and-braces.
        .env("LANG", "en")
        .env("LC_ALL", "en");
}

/// Translation gates: small, targeted suites that run the binary under a
/// non-default locale and assert the translated output. These are *not*
/// mirrors of `cli_tests` — each only covers a representative slice of
/// commands that exercise i18n message families, enough to catch broken
/// `.ftl` files, missing message ids, or argument-name mismatches between
/// Rust and Fluent. Add a new case to each suite whenever you add a new
/// message family to `cli/i18n.rs`; the English coverage in `cli_tests`
/// stays comprehensive.
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
