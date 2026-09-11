//! M12 conformance fixture: minimal proc-macro crate for the
//! dx_rust_proc_macro wrapper subject.

use proc_macro::TokenStream;

/// Identity function-like macro used only to prove the proc-macro
/// wrapper forwards a compilable proc-macro crate.
#[proc_macro]
pub fn hello_identity(input: TokenStream) -> TokenStream {
    input
}
