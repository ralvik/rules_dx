use cc::Build;

fn main() {
    let _ = Build::new();
    println!("cargo:rerun-if-changed=build/script.rs");
}
