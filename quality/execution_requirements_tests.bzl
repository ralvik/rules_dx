"""Unit tests for the execution-requirements helper."""

load("//libs/starlark:defs.bzl", "expect_equal", "expect_false", "expect_true", "starlark_test")
load(":execution_requirements.bzl", "DX_FORBIDDEN_NO_REMOTE", "DX_NO_REMOTE_EXEC", "DX_REMOTE_QUALIFIED", "dx_execution_requirements", "dx_has_forbidden_marker", "dx_is_local_only")

def execution_requirements_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_false(
                "remote stays unqualified so the helper stays local-only",
                DX_REMOTE_QUALIFIED,
            ),
            expect_equal(
                "helper returns the local-only marker",
                dx_execution_requirements(),
                {"no-remote-exec": "1"},
            ),
            expect_equal(
                "marker key stays the frozen constant",
                DX_NO_REMOTE_EXEC,
                "no-remote-exec",
            ),
            expect_equal(
                "forbidden marker stays the cache-disabling key",
                DX_FORBIDDEN_NO_REMOTE,
                "no-remote",
            ),
            expect_true(
                "helper output reads as local-only",
                dx_is_local_only(dx_execution_requirements()),
            ),
            expect_false(
                "helper output carries no cache-disabling marker",
                dx_has_forbidden_marker(dx_execution_requirements()),
            ),
            expect_false(
                "empty requirements read as remote-capable, never local-only",
                dx_is_local_only({}),
            ),
            expect_true(
                "bare no-remote reads as forbidden",
                dx_has_forbidden_marker({"no-remote": "1"}),
            ),
        ],
    )
