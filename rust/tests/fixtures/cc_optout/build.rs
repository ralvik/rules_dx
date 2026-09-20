//! Pure-Rust build script for the CC opt-out fixture.
//!
//! Emits a cfg without touching any C/C++ tool. Must succeed with
//! `use_cc_toolchain = 0` (kept opt-out): the script execution action
//! carries no C++ toolchain inputs, while the script binary itself still
//! compiles through the normal Rust toolchain.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(has_cc_optout_stamp)");
    println!("cargo:rustc-cfg=has_cc_optout_stamp");
}
