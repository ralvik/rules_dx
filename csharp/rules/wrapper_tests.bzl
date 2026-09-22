"""Wrapper conformance tests for C#.

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "csharp_tfm_with_defaults")

def csharp_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("csharp class is known", "csharp" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout/flaky", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short", "flaky": True}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "empty gains net10 tfm",
                csharp_tfm_with_defaults({})["target_frameworks"],
                ["net10.0"],
            ),
            expect_equal(
                "empty gains warnings as errors",
                csharp_tfm_with_defaults({})["treat_warnings_as_errors"],
                True,
            ),
            expect_equal(
                "explicit tfm wins",
                csharp_tfm_with_defaults({"target_frameworks": ["net9.0"]})["target_frameworks"],
                ["net9.0"],
            ),
            expect_equal(
                "explicit warnings setting wins",
                csharp_tfm_with_defaults({"treat_warnings_as_errors": False})["treat_warnings_as_errors"],
                False,
            ),
            expect_equal(
                "other kwargs survive",
                csharp_tfm_with_defaults({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
