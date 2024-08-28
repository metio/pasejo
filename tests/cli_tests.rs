#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/help/*.md")
        .case("tests/cmd/identity/*/*.md")
        .case("tests/cmd/recipient/*/*.md")
        .case("tests/cmd/store/*-none/*.md");

    if which::which("git").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-git/*.md");
    }
    if which::which("hg").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-hg/*.md");
    }
}
