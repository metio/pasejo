#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/help/*.toml")
        .case("tests/cmd/store/*-none/*.toml");

    if which::which("git").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-git/*.toml");
    }
    if which::which("hg").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-hg/*.toml");
    }
}
