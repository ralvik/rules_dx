load("//libs/starlark:defs.bzl", "expect_equal", "expect_match", "expect_true", "starlark_test")
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
            expect_equal("test kwargs forward timeout, flaky stays upstream", dx_forwarded_test_kwargs({"timeout": "short", "flaky": True}), {"timeout": "short"}),
            expect_equal("test kwargs keep flaky out of forwarder", dx_forwarded_test_kwargs({"flaky": True}), {}),
            expect_equal("test kwargs empty stays empty", dx_forwarded_test_kwargs({}), {}),
            expect_true(
                "empty gains platform werror select",
                type(cc_copts_with_werror({})["copts"]) == "select",
            ),
            expect_match(
                "empty select covers windows WX and default Werror",
                str(cc_copts_with_werror({})["copts"]),
                "windows",
            ),
            expect_equal(
                "existing werror is kept without duplication",
                cc_copts_with_werror({"copts": ["-Werror"]})["copts"],
                ["-Werror"],
            ),
            expect_equal(
                "existing msvc werror is kept without duplication",
                cc_copts_with_werror({"copts": ["/WX"]})["copts"],
                ["/WX"],
            ),
            expect_true(
                "other flags stay under the platform select",
                type(cc_copts_with_werror({"copts": ["-Wall"]})["copts"]) == "select",
            ),
            expect_match(
                "other flags keep -Wall in the select",
                str(cc_copts_with_werror({"copts": ["-Wall"]})["copts"]),
                "-Wall",
            ),
            expect_match(
                "windows std rewrite carries /Zc:__cplusplus",
                str(cc_copts_with_werror({"copts": ["-std=c++17"]})["copts"]),
                "/Zc:__cplusplus",
            ),
            expect_equal(
                "other kwargs survive",
                cc_copts_with_werror({"deps": [":hello_lib"]})["deps"],
                [":hello_lib"],
            ),
        ],
    )
