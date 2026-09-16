"""Wrapper-contract tests for the JavaScript wrappers (#87 item 2).

Pins the jest-pipeline enforcement: `javascript_test` always routes
through `jest_test` with the standard auto-configured reporters, so
disabling them (a project-specific result protocol) is rejected with a
contract diagnostic. Filtering support is structural: the effective
runtime environment always carries `TESTBRIDGE_TEST_ONLY` no matter
what the caller passes. The `fail()` in the `javascript_test` macro is
load-time, so both behaviors are proven here through the pure
`javascript_test_rejection` and `javascript_test_env` helpers.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":defs.bzl", "javascript_test_env", "javascript_test_rejection")

def javascript_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "disabling standard reporters is rejected",
                javascript_test_rejection({"auto_configure_reporters": False}) != None,
                True,
            ),
            expect_equal(
                "reporter rejection names the standard logs",
                javascript_test_rejection({"auto_configure_reporters": False}),
                "javascript_test always uses jest with the standard " +
                "auto-configured reporters (Bazel test logs); " +
                "`auto_configure_reporters = False` is not supported " +
                "(project-specific result protocols are rejected per " +
                "docs/testing/generation.md).",
            ),
            expect_equal(
                "empty kwargs are clean",
                javascript_test_rejection({}),
                None,
            ),
            expect_equal(
                "explicit standard reporters are clean",
                javascript_test_rejection({"auto_configure_reporters": True}),
                None,
            ),
            expect_equal(
                "ordinary jest kwargs are clean",
                javascript_test_rejection({"config": "jest.config.cjs", "snapshots": False}),
                None,
            ),
            expect_equal(
                "missing env gains the filter channel",
                javascript_test_env(None),
                ["TESTBRIDGE_TEST_ONLY"],
            ),
            expect_equal(
                "caller env keeps entries and gains the filter channel",
                javascript_test_env(["FOO"]),
                ["FOO", "TESTBRIDGE_TEST_ONLY"],
            ),
            expect_equal(
                "present filter channel is not duplicated",
                javascript_test_env(["FOO", "TESTBRIDGE_TEST_ONLY"]),
                ["FOO", "TESTBRIDGE_TEST_ONLY"],
            ),
        ],
    )
