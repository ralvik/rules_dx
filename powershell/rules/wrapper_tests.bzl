"""Wrapper conformance tests for PowerShell.

Contract: `docs/decisions/0032-ruby-powershell-bandit-swift.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "powershell_effective_srcs")

def powershell_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("powershell class is known", "powershell" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout/flaky", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short", "flaky": True}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "none srcs become empty thin shape",
                powershell_effective_srcs(None),
                [],
            ),
            expect_equal(
                "empty srcs stay empty",
                powershell_effective_srcs([]),
                [],
            ),
            expect_equal(
                "sources are kept",
                powershell_effective_srcs(["Hello.ps1"]),
                ["Hello.ps1"],
            ),
        ],
    )
