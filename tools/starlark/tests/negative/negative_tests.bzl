"""Negative demonstrations (M01 WP4, WP5). All manual: they must fail.

Run explicitly; never part of `//...` suites or CI:

  bazel test //tools/starlark/tests/negative/... --nocache_test_results
  bazel build //tools/starlark/tests/negative:wrong_phase_demo \
    --nobuild 2>&1 | tail -5

Each failure mode below maps to a `failure-rendering` or `mode-validation`
matrix item, with captured output in the M01 completion report.
"""

load("//tools/starlark:defs.bzl", "expect_equal", "starlark_test")

def failing_check_demo(name):
    starlark_test(
        name = name,
        mode = "unit",
        checks = [
            expect_equal("deliberately wrong sum", 1 + 1, 3),
            expect_equal("deliberately wrong product", 2 * 2, 5),
            expect_equal("control that still passes", 1 + 1, 2),
        ],
        tags = ["manual"],
    )

def missing_observation_demo(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":negative_subject"],
        expected_observations = "subject //tools/starlark/tests/negative:negative_subject\nfile negative_subject.txt\nfield left=0\nfield right=0\nfield sum=43",
        tags = ["manual"],
    )

def missing_fragment_demo(name):
    starlark_test(
        name = name,
        mode = "execution",
        file_checks = {
            ":present_fixture.txt": "this substring is absent",
        },
        tags = ["manual"],
    )

def wrong_phase_demo(name):
    # Analysis error, not a test failure: load mode rejects subjects.
    starlark_test(
        name = name,
        mode = "load",
        checks = [expect_equal("fine on its own", 1, 1)],
        subjects = [":negative_subject"],
        tags = ["manual"],
    )
