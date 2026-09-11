//! M12 conformance fixture: minimal staticlib crate for the
//! dx_rust_static_library wrapper subject.

/// C-compatible greeting entry point proving the static-library wrapper
/// forwards a compilable staticlib crate.
#[no_mangle]
pub extern "C" fn hello_staticlib() {}
