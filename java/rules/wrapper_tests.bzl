"""Wrapper conformance tests for Java.

Contract: `docs/decisions/0019-first-release-additional-foundations.md`.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "java_javacopts_with_werror")

def java_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("java class is known", "java" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout, flaky stays upstream", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short"}),
            expect_equal("test kwargs keep flaky out of forwarder", dx_forwarded_test_kwargs({"flaky": True}), {}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "empty gains werror and lint",
                java_javacopts_with_werror({})["javacopts"],
                ["-Werror", "-Xlint:all"],
            ),
            expect_equal(
                "existing werror is kept without duplication",
                java_javacopts_with_werror({"javacopts": ["-Werror"]})["javacopts"],
                ["-Werror", "-Xlint:all"],
            ),
            expect_equal(
                "existing lint gains werror",
                java_javacopts_with_werror({"javacopts": ["-Xlint:all"]})["javacopts"],
                ["-Xlint:all", "-Werror"],
            ),
            expect_equal(
                "both flags stay unchanged",
                java_javacopts_with_werror({"javacopts": ["-Werror", "-Xlint:all"]})["javacopts"],
                ["-Werror", "-Xlint:all"],
            ),
            expect_equal(
                "other kwargs survive",
                java_javacopts_with_werror({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
