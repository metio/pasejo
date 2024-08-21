#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/help/*.toml")
        .case("tests/cmd/store/init/*.toml");

    if which::which("git").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/init-git/*.toml");
    }
    if which::which("hg").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/init-hg/*.toml");
    }
}
