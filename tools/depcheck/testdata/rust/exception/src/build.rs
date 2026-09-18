fn main() {
    // build-plugin runs here at build time (non-import use).
    println!("cargo:rerun-if-changed=build.rs");
}
