"""Identity tests pinning the authoritative toolchain binding (M04 WP1).

Both executables must resolve inside the selected Rust toolchain
repositories. Any owner or path change (toolchain update, rules_rust layout
change, accidental vendored copy) fails this test until re-pinned by review.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":bindings.bzl", "RUSTFMT_TOOLCHAIN_TYPE", "RUST_TOOLCHAIN_TYPE")

# Buildifier's canonical-repository lint forbids a literal "@@" in source,
# so the pin spells it as "@" + "@..."; the observed value keeps "@@".
_AT = "@"

# Frozen pin: executed owners/paths for Rust 1.98.0 on linux_x86_64.
# Both executables live in authoritative toolchain repositories; any change
# (toolchain update, rules_rust layout change, vendored copy) fails review.
EXPECTED_IDENTITY_OBSERVATIONS = """subject //rust/toolchains:identity_under_test
field clippy.owner=""" + _AT + """@rules_rust++rust+rust_linux_x86_64__x86_64-unknown-linux-gnu__stable_tools//:rust_toolchain
field clippy.path=bazel-out/k8-fastbuild/bin/external/rules_rust++rust+rust_linux_x86_64__x86_64-unknown-linux-gnu__stable_tools/rust_toolchain/bin/clippy-driver
field rustfmt.owner=""" + _AT + """@rules_rust++rust+rustfmt_1.98.0__x86_64-unknown-linux-gnu_tools//:bin/rustfmt
field rustfmt.path=external/rules_rust++rust+rustfmt_1.98.0__x86_64-unknown-linux-gnu_tools/bin/rustfmt"""

def toolchain_identity_tests(name):
    starlark_test(
        name = name + "_binding",
        mode = "load",
        checks = [
            expect_equal(
                "clippy binds to the main Rust toolchain type",
                RUST_TOOLCHAIN_TYPE,
                "@rules_rust//rust:toolchain_type",
            ),
            expect_equal(
                "rustfmt binds to the registered rustfmt toolchain type",
                RUSTFMT_TOOLCHAIN_TYPE,
                "@rules_rust//rust/rustfmt:toolchain_type",
            ),
        ],
    )
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":identity_under_test"],
        expected_observations = EXPECTED_IDENTITY_OBSERVATIONS,
    )
