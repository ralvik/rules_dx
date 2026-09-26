"""Wrapper conformance tests for Kotlin."""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load("//libs/starlark:wrapper.bzl", "dx_effective_visibility", "dx_forwarded_test_kwargs")
load("//quality:sources.bzl", "KNOWN_SEMANTIC_FILE_CLASSES")
load(":defs.bzl", "kotlin_kotlinc_opts_with_werror")

def kotlin_wrapper_contract_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("kotlin class is known", "kotlin" in KNOWN_SEMANTIC_FILE_CLASSES, True),
            expect_equal("forwarder defaults to private", dx_effective_visibility(None), ["//visibility:private"]),
            expect_equal("explicit visibility wins", dx_effective_visibility(["//visibility:public"]), ["//visibility:public"]),
            expect_equal("test kwargs strip manual", dx_forwarded_test_kwargs({"tags": ["manual", "cpu:4"]}), {"tags": ["cpu:4"]}),
            expect_equal("test kwargs forward timeout, flaky stays upstream", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short"}),
            expect_equal("test kwargs keep flaky out of forwarder", dx_forwarded_test_kwargs({"flaky": True}), {}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_equal(
                "empty gains warnings-as-errors",
                kotlin_kotlinc_opts_with_werror({})["kotlinc_opts"],
                "//kotlin/rules:warnings_as_errors",
            ),
            expect_equal(
                "explicit opts win",
                kotlin_kotlinc_opts_with_werror({"kotlinc_opts": "//custom:opts"})["kotlinc_opts"],
                "//custom:opts",
            ),
            expect_equal(
                "other kwargs survive",
                kotlin_kotlinc_opts_with_werror({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
