use std::path::Path;

#[test]
fn macros_compile_and_fail_as_expected() {
    for dir in ["tests/ui/pass", "tests/ui/fail"] {
        assert_every_case_directory_has_an_entry(Path::new(dir));
    }
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/*.rs");
    cases.compile_fail("tests/ui/fail/*.rs");
}

/// trybuild compiles only the top-level `*.rs` files, so a directory without
/// a sibling `<name>.rs` entry would never be compiled. Every case directory
/// must belong to an entry, and shared fixtures live inside it.
fn assert_every_case_directory_has_an_entry(dir: &Path) {
    let orphans: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.is_dir() && !path.with_extension("rs").is_file())
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        orphans.is_empty(),
        "trybuild never compiles these directories because they have no sibling `<name>.rs` entry: {orphans:?}"
    );
}
