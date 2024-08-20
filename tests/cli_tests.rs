#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/help/*.toml")
        .case("tests/cmd/store/*/*.toml");
}
