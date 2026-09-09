"""Execution tests over runfiles fixtures (M01 WP1)."""

load("//tools/starlark:defs.bzl", "expect_equal", "starlark_test")

def fixture_execution_tests(name):
    starlark_test(
        name = name,
        mode = "execution",
        checks = [
            expect_equal("record encoding is deterministic", expect_equal("x", 1, 2), "{\"actual\":1,\"expected\":2,\"name\":\"x\"}"),
        ],
        file_checks = {
            ":answer_fixture.txt": "answer=42",
            ":shape_fixture.txt": "shape=circle",
        },
    )
