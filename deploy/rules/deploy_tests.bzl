"""Unit and analysis tests for the deploy boundary.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":defs.bzl", "VALID_DEPLOY_PROFILES", "deploy_profile_error")

def deploy_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "profile vocabulary pins debug, dev, release",
                VALID_DEPLOY_PROFILES,
                ["debug", "dev", "release"],
            ),
            expect_equal(
                "deploy_profile_error accepts every valid profile",
                [
                    deploy_profile_error("debug"),
                    deploy_profile_error("dev"),
                    deploy_profile_error("release"),
                ],
                ["", "", ""],
            ),
            expect_equal(
                "deploy_profile_error accepts None as command default",
                deploy_profile_error(None),
                "",
            ),
            expect_equal(
                "deploy_profile_error rejects an unknown profile",
                deploy_profile_error("staging"),
                "dx_deployment: invalid profile 'staging': want one of debug, dev, release",
            ),
            expect_equal(
                "deploy_profile_error rejects an empty profile",
                deploy_profile_error(""),
                "dx_deployment: invalid profile '': want one of debug, dev, release",
            ),
        ],
    )

EXPECTED_DEPLOY_DEFAULT_OBSERVATIONS = """subject //deploy/rules:deploy_default
file deploy_default
field app=
field profile=release"""

EXPECTED_DEPLOY_DEBUG_OBSERVATIONS = """subject //deploy/rules:deploy_debug
file deploy_debug
field app=
field profile=debug"""

EXPECTED_DEPLOY_WITH_APP_OBSERVATIONS = """subject //deploy/rules:deploy_with_app
file deploy_with_app
field app=//deploy/rules:deploy_program
field profile=release"""

def deploy_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":deploy_default"],
        expected_observations = EXPECTED_DEPLOY_DEFAULT_OBSERVATIONS,
    )

def deploy_debug_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":deploy_debug"],
        expected_observations = EXPECTED_DEPLOY_DEBUG_OBSERVATIONS,
    )

def deploy_app_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":deploy_with_app"],
        expected_observations = EXPECTED_DEPLOY_WITH_APP_OBSERVATIONS,
    )
