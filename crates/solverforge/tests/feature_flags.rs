//! The facade's feature table: `verbose-logging` turns on console output, and
//! no feature exists that gates nothing.

fn feature_lines(manifest: &str) -> Vec<(String, String)> {
    let mut in_features = false;
    let mut lines = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_features = line == "[features]";
            continue;
        }
        if !in_features || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').expect("feature line has `=`");
        lines.push((name.trim().to_string(), value.trim().to_string()));
    }
    lines
}

#[test]
fn verbose_logging_implies_console() {
    let manifest = include_str!("../Cargo.toml");
    let features = feature_lines(manifest);
    let (_, verbose) = features
        .iter()
        .find(|(name, _)| name == "verbose-logging")
        .expect("facade declares verbose-logging");
    assert!(verbose.contains("\"console\""), "{verbose}");
}

#[test]
fn no_decimal_feature_or_dependency_remains() {
    let facade = include_str!("../Cargo.toml");
    let core = include_str!("../../solverforge-core/Cargo.toml");
    let workspace = include_str!("../../../Cargo.toml");
    for manifest in [facade, core] {
        assert!(feature_lines(manifest)
            .iter()
            .all(|(name, _)| name != "decimal"));
    }
    assert!(!workspace.contains("rust_decimal"));
}

#[cfg(feature = "verbose-logging")]
#[test]
fn verbose_logging_builds_with_the_console_module() {
    // `solverforge::console` exists only with the `console` feature, so this
    // compiles only when `verbose-logging` enables it.
    solverforge::console::init();
}
