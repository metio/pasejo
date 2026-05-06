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
        .env("PASEJO_DISABLE_HOOK_THROTTLING", "1");
}
