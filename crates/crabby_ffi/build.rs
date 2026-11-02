fn main() {
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=cbindgen.toml");
    let config = cbindgen::Config::from_file(root.join("cbindgen.toml")).unwrap();
    let bindings = cbindgen::Builder::new()
        .with_crate(root)
        .with_config(config)
        .generate()
        .expect("generate C ABI header");
    bindings.write_to_file(std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("crabby.h"));
}
