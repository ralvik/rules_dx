"""Analysis tests for the example subject rule."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //libs/starlark/tests:subject_under_test
file subject_under_test.txt
field left=40
field right=2
field sum=42
aspect_field aspect_seen=True
aspect_field field_count=3
aspect_field has_subject=True
aspect_field subject_label=//libs/starlark/tests:subject_under_test
aspect_field transitive_count=0"""

def subject_analysis_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":subject_under_test"],
        expected_observations = EXPECTED_OBSERVATIONS,
    )
