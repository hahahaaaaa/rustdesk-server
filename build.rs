use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let version = env!("CARGO_PKG_VERSION");
    let contents = format!("pub const VERSION: &str = \"{version}\";\n");
    if let Err(err) = fs::write("src/version.rs", contents) {
        panic!("failed to write src/version.rs: {err}");
    }
}
