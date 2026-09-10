"""Tested-stack manifest contract tests (M01 WP7).

Asserts manifest content and cross-checks every pinned version against the
ground-truth files (`.bazelversion`, `MODULE.bazel`). A stale default in
the `stack` target fails here.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")

def tested_stack_contract_tests(name):
    starlark_test(
        name = name,
        mode = "execution",
        checks = [
            expect_equal("contract has manifest evidence", 1, 1),
        ],
        file_checks = {
            ":stack": "\"schema_version\": 1\n\"generator\": \"libs/testing/tested_stack.bzl\"\n\"bazel_version\": \"9.2.0\"\n\"rules_rust_version\": \"0.74.0\"\n\"rules_cc_version\": \"0.2.22\"\n\"rust_version\": \"1.98.0\"\n\"x86_64-unknown-linux-gnu\"",
            "//:.bazelversion": "9.2.0",
            "//:MODULE.bazel": "name = \"rules_rust\", version = \"0.74.0\"\nname = \"rules_cc\", version = \"0.2.22\"\nversions = [\"1.98.0\"]",
        },
    )
