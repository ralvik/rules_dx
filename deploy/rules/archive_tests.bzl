"""Unit and analysis tests for the archive releaser.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":archive.bzl", "archive_filenames")

def archive_unit_tests(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal(
                "archive_filenames pins the tarball and checksum names",
                archive_filenames("release_demo"),
                ("release_demo.tar.gz", "release_demo.tar.gz.sha256"),
            ),
            expect_equal(
                "archive_filenames derives names per instance",
                archive_filenames("release"),
                ("release.tar.gz", "release.tar.gz.sha256"),
            ),
        ],
    )

EXPECTED_ARCHIVE_DEFAULT_OBSERVATIONS = """subject //deploy/rules:release_demo
file release_demo
field app=//deploy/rules:deploy_program
field profile=release"""

EXPECTED_ARCHIVE_DEBUG_OBSERVATIONS = """subject //deploy/rules:release_demo_debug
file release_demo_debug
field app=//deploy/rules:deploy_program
field profile=debug"""

def archive_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":release_demo"],
        expected_observations = EXPECTED_ARCHIVE_DEFAULT_OBSERVATIONS,
    )

def archive_debug_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":release_demo_debug"],
        expected_observations = EXPECTED_ARCHIVE_DEBUG_OBSERVATIONS,
    )
