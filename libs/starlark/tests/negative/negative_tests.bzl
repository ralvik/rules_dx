"""Negative demonstrations (WP4, WP5) as green hermetic proofs (issue #406).
"""

load("@rules_shell//shell:sh_test.bzl", "sh_test")
load("//libs/starlark:failure_test.bzl", "failure_test")

# Deliberately wrong fixtures (red data, never live failing targets):
# - "deliberately wrong sum": 1 + 1 vs 3
# - "deliberately wrong product": 2 * 2 vs 5
# - observation with field sum=43 (actual sum=0 from :negative_subject)
# - absent substring "this substring is absent" in :present_fixture.txt
# The passing harnesses below assert these fail exactly as documented.

def failing_check_demo(name):
    sh_test(
        name = name,
        srcs = ["failing_check_test.sh"],
        data = ["//tools/sh:lib"],
        size = "small",
        target_compatible_with = ["@platforms//os:linux"],
    )

def missing_observation_demo(name):
    sh_test(
        name = name,
        srcs = ["missing_observation_test.sh"],
        data = [
            "//tools/sh:lib",
            ":negative_subject",
        ],
        size = "small",
        target_compatible_with = ["@platforms//os:linux"],
    )

def missing_fragment_demo(name):
    sh_test(
        name = name,
        srcs = ["missing_fragment_test.sh"],
        data = [
            "//tools/sh:lib",
            ":present_fixture.txt",
        ],
        size = "small",
        target_compatible_with = ["@platforms//os:linux"],
    )

def _wrong_phase_subject_impl(ctx):
    if len(ctx.attr.subjects) != 0:
        fail("starlark_test (load mode): subjects must be empty: " +
             "load tests observe loading, not analysis")
    return [DefaultInfo(files = depset([]))]

_wrong_phase_subject = rule(
    implementation = _wrong_phase_subject_impl,
    attrs = {
        "subjects": attr.label_list(
            doc = "Must be empty in load mode; non-empty fails analysis.",
        ),
    },
    doc = "Non-test subject reproducing the load-mode subjects rejection for failure_test.",
)

def wrong_phase_demo(name):
    # Analysis error, not a test failure: load mode rejects subjects.
    # Subject stays manual (non-test, so wildcard builds skip it); the
    # expect-failure test is non-manual and green.
    _wrong_phase_subject(
        name = name + "_subject",
        subjects = [":negative_subject"],
        tags = ["manual"],
    )
    failure_test(
        name = name,
        target = ":" + name + "_subject",
        expected_failure_substring = "starlark_test (load mode): subjects must be empty",
    )
