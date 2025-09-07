// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/completion/*.md")
        .case("tests/cmd/config/*/*.md")
        .case("tests/cmd/config-migrations/*/*.md")
        .case("tests/cmd/help/*.md")
        .case("tests/cmd/hook/*/*.md")
        .case("tests/cmd/identity/*/*.md")
        .case("tests/cmd/otp/*/*.md")
        .case("tests/cmd/recipient/*/*.md")
        .case("tests/cmd/secret/*/*.md")
        .case("tests/cmd/store/*/*.md")
        .case("docs/commands/pasejo-cmd-*.md")
        .env("PASEJO_CONFIG", "config.toml");
}
