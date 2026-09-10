"""Analysis subject proving rustfmt/Clippy bind to the toolchain (M04 WP1).

Renders the owning repository and exec path of each bound executable, so the
identity test pins that both resolve inside the authoritative toolchain
repositories selected for Rust 1.98.0 rather than a `rules_dx`-owned copy.
"""

load("//libs/starlark:defs.bzl", "DxSubjectInfo")
load(":bindings.bzl", "rust_toolchain_toolchains", "rust_toolchain_tools")

def _toolchain_identity_subject_impl(ctx):
    clippy_driver, rustfmt = rust_toolchain_tools(ctx)
    return [
        DefaultInfo(files = depset([])),
        DxSubjectInfo(fields = {
            "clippy.owner": str(clippy_driver.owner),
            "clippy.path": clippy_driver.path,
            "rustfmt.owner": str(rustfmt.owner),
            "rustfmt.path": rustfmt.path,
        }),
    ]

toolchain_identity_subject = rule(
    implementation = _toolchain_identity_subject_impl,
    toolchains = rust_toolchain_toolchains(),
)
