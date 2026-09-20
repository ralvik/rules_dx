"""Authoritative Rust toolchain bindings for quality adapters (WP1, WP3).

Contract: `docs/quality/tool-integrations.md`.
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
    """Return the `rustc` File for the selected toolchain.

 Declared for rust stages; the delegated typecheck pipeline
    never spawns it — findings parse from the upstream `rustc_output`
    diagnostics file, so the compiler follows the selected toolchain
    with no adapter edit.
    """
    return ctx.toolchains[RUST_TOOLCHAIN_TYPE].rustc

def rust_toolchain_toolchains():
    """Toolchain types an adapter rule must declare for `rust_toolchain_tools`."""
    return [RUST_TOOLCHAIN_TYPE, RUSTFMT_TOOLCHAIN_TYPE]
