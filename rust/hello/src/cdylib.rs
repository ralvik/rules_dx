//! M12 conformance fixture: minimal cdylib crate for the
//! dx_rust_shared_library wrapper subject.

/// C-compatible greeting entry point proving the shared-library wrapper
/// forwards a compilable cdylib crate.
#[no_mangle]
pub extern "C" fn hello_cdylib() {}
