"""Authoritative Rust toolchain bindings for quality adapters (M04 WP1, M12 WP3).

rustfmt, Clippy, and rustc typechecking always follow the selected Rust
toolchain: no independent quality-tool copy exists. rustfmt resolves from
the registered `rustfmt_toolchain` for the selected version; Clippy and
rustc resolve from the main Rust toolchain's `clippy_driver` and `rustc`.
Changing the toolchain version changes all action inputs and keys with no
adapter edit.
"""

RUST_TOOLCHAIN_TYPE = "@rules_rust//rust:toolchain_type"
RUSTFMT_TOOLCHAIN_TYPE = "@rules_rust//rust/rustfmt:toolchain_type"

def rust_toolchain_tools(ctx):
    """Return the `(clippy_driver, rustfmt)` Files for the selected toolchain.

    Both files live in the authoritative toolchain repositories, never in a
    `rules_dx`-owned copy. Adapters declare both toolchain types and pass
    these files as action tools/inputs.
    """
    rust = ctx.toolchains[RUST_TOOLCHAIN_TYPE]
    rustfmt = ctx.toolchains[RUSTFMT_TOOLCHAIN_TYPE]
    return rust.clippy_driver, rustfmt.rustfmt

def rust_toolchain_rustc(ctx):
    """Return the `rustc` File for the selected toolchain (M12 WP3).

    The typecheck pipeline invokes the toolchain compiler directly as a
    lib-root check (`commands::rustc_check`); like Clippy it follows the
    selected toolchain with no adapter edit.
    """
    return ctx.toolchains[RUST_TOOLCHAIN_TYPE].rustc

def rust_toolchain_toolchains():
    """Toolchain types an adapter rule must declare for `rust_toolchain_tools`."""
    return [RUST_TOOLCHAIN_TYPE, RUSTFMT_TOOLCHAIN_TYPE]
