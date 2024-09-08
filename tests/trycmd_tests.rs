#[test]
fn cli_tests() {
    trycmd::TestCases::new()
        .case("tests/cmd/completion/*.md")
        .case("tests/cmd/help/*.md")
        .case("tests/cmd/identity/*/*.md")
        .case("tests/cmd/recipient/*/*.md")
        .case("tests/cmd/store/*-none/*.md")
        .case("tests/cmd/store/remove/*.md")
        .case("tests/cmd/store/set-default/*.md");

    if let (Ok(touch), Ok(mkdir)) = (which::which("touch"), which::which("mkdir")) {
        trycmd::TestCases::new()
            .case("tests/cmd/secret/*/*.md")
            .register_bin("touch", touch)
            .register_bin("mkdir", mkdir);
    }

    if which::which("git").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-git/*.md");
    }
    if which::which("hg").is_ok() {
        trycmd::TestCases::new().case("tests/cmd/store/*-hg/*.md");
    }
}
