//! Prints the C header generated from this crate. Regenerate it with
//! `cargo run -p ponko_ffi --example generate_header > include/crabby.h`;
//! CI checks the committed header matches.
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = cbindgen::Config::from_file(root.join("cbindgen.toml")).expect("cbindgen.toml");
    cbindgen::Builder::new()
        .with_crate(root)
        .with_config(config)
        .generate()
        .expect("generate C ABI header")
        .write(std::io::stdout());
}
