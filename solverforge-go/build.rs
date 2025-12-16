use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = PathBuf::from(&crate_dir).join("go/solverforge");

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Generate C header file using cbindgen
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("SOLVERFORGE_H")
        .with_no_includes()
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(output_dir.join("solverforge.h"));

    println!("cargo:rerun-if-changed=src/");
}
