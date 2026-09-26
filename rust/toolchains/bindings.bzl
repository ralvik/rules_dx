
RUST_TOOLCHAIN_TYPE = "@rules_rust//rust:toolchain_type"
RUSTFMT_TOOLCHAIN_TYPE = "@rules_rust//rust/rustfmt:toolchain_type"

def rust_toolchain_tools(ctx):
    rust = ctx.toolchains[RUST_TOOLCHAIN_TYPE]
    rustfmt = ctx.toolchains[RUSTFMT_TOOLCHAIN_TYPE]
    return rust.clippy_driver, rustfmt.rustfmt

def rust_toolchain_rustc(ctx):
    return ctx.toolchains[RUST_TOOLCHAIN_TYPE].rustc

def rust_toolchain_toolchains():
    return [RUST_TOOLCHAIN_TYPE, RUSTFMT_TOOLCHAIN_TYPE]
