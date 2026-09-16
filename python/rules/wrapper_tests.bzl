"""Wrapper-contract tests for the Python wrappers (#87 item 2).

Pins the pytest-driver enforcement: `python_test` always runs pytest
through `py_pytest_test`, so a generic `main` (or any alternate test
driver) is rejected with a contract diagnostic while pytest-native
kwargs (`pytest_args`, `chdir`, `deps`, `tags`) pass through. The
`fail()` in the `python_test` macro is load-time, so the rejection
itself is proven here through the pure `python_test_rejection`
helper: a `None` return means "no rejection".
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":defs.bzl", "python_test_rejection")

def python_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "generic main is rejected",
                python_test_rejection({"main": "main.py"}) != None,
                True,
            ),
            expect_equal(
                "main rejection names pytest",
                python_test_rejection({"main": "main.py"}),
                "python_test always runs pytest and provides its own " +
                "entrypoint; `main` is not supported (generic mains and " +
                "alternate test drivers are rejected per " +
                "docs/testing/generation.md). Use py_pytest_main + py_test " +
                "directly for a custom main.",
            ),
            expect_equal(
                "empty kwargs are clean",
                python_test_rejection({}),
                None,
            ),
            expect_equal(
                "ordinary test kwargs are clean",
                python_test_rejection({"deps": ["@pypi//pytest"], "tags": ["small"]}),
                None,
            ),
            expect_equal(
                "pytest-native entrypoint kwargs stay pytest",
                python_test_rejection({"pytest_args": ["-k foo"], "chdir": "pkg"}),
                None,
            ),
        ],
    )
