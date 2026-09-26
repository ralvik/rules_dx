"""Wrapper conformance tests for Scala.

"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "scala_scalacopts_with_werror")

def scala_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("scala class is known", "scala" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout, flaky stays upstream", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short"}),
            expect_equal("test kwargs keep flaky out of forwarder", dx_forwarded_test_kwargs({"flaky": True}), {}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "empty gains fatal warnings",
                scala_scalacopts_with_werror({})["scalacopts"],
                ["-Xfatal-warnings"],
            ),
            expect_equal(
                "existing flag is kept without duplication",
                scala_scalacopts_with_werror({"scalacopts": ["-Xfatal-warnings"]})["scalacopts"],
                ["-Xfatal-warnings"],
            ),
            expect_equal(
                "other flags are kept",
                scala_scalacopts_with_werror({"scalacopts": ["-deprecation"]})["scalacopts"],
                ["-deprecation", "-Xfatal-warnings"],
            ),
            expect_equal(
                "other kwargs survive",
                scala_scalacopts_with_werror({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
