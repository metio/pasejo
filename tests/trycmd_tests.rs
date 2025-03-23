// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

use std::fs;

use duct::cmd;
use regex::Regex;

#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/completion/*.md")
        .case("tests/cmd/help/*.md")
        .case("tests/cmd/identity/*/*.md")
        .case("tests/cmd/recipient/*/*.md")
        .case("tests/cmd/store/init-none/*.md")
        .case("tests/cmd/store/remove/*.md")
        .case("tests/cmd/store/set-default/*.md");

    if let (Ok(echo), Ok(age_keygen)) = (which::which("echo"), which::which("age-keygen")) {
        let key = cmd!("age-keygen")
            .read()
            .ok()
            .expect("to see a new age key");
        let regex = Regex::new("# public key: (?P<recipient>.*)").expect("regex to compile");
        let captures = regex.captures(key.as_str()).expect("to find matches");
        let recipient = &captures["recipient"];

        fs::write(
            "tests/cmd/secret/insert/insert.in/recipient",
            recipient.to_owned(),
        )
        .expect("Unable to write file");

        trycmd::TestCases::new()
            .case("tests/cmd/secret/insert/*.md")
            .register_bin("echo", echo)
            .register_bin("age-keygen", age_keygen)
            .insert_var("[RECIPIENT]", recipient.to_owned())
            .expect("this to execute");
    }

    if let (Ok(touch), Ok(mkdir)) = (which::which("touch"), which::which("mkdir")) {
        trycmd::TestCases::new()
            .case("tests/cmd/secret/list/*.md")
            .case("tests/cmd/secret/move/*.md")
            .register_bin("touch", touch)
            .register_bin("mkdir", mkdir);
    }

    if which::which("git").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-git/*.md");
    }
    if which::which("hg").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-hg/*.md");
    }
    if which::which("pijul").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-pijul/*.md");
    }
}
