"""Wrapper conformance tests for C/C++.

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "cc_copts_with_werror")

def cc_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("c class is known", "c" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("cpp class is known", "cpp" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("cuda class is known", "cuda" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout/flaky", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short", "flaky": True}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "empty gains werror",
                cc_copts_with_werror({})["copts"],
                ["-Werror"],
            ),
            expect_equal(
                "existing werror is kept without duplication",
                cc_copts_with_werror({"copts": ["-Werror"]})["copts"],
                ["-Werror"],
            ),
            expect_equal(
                "other flags are kept",
                cc_copts_with_werror({"copts": ["-Wall"]})["copts"],
                ["-Wall", "-Werror"],
            ),
            expect_equal(
                "other kwargs survive",
                cc_copts_with_werror({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
