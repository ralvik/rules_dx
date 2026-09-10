"""Analysis tests for the example subject rule (M01 WP1)."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //libs/starlark/tests:subject_under_test
file subject_under_test.txt
field left=40
field right=2
field sum=42"""

def subject_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":subject_under_test"],
        expected_observations = EXPECTED_OBSERVATIONS,
    )
