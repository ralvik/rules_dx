"""Aspect-lint golden harness (issue #406).
"""

load("//libs/starlark:defs.bzl", "starlark_test")
load("//quality:fixtures.bzl", "real_source_target")
load("//quality/testdata:real_aspect_subject.bzl", "real_aspect_subject")

def lint_test(name, srcs, expected_observations, **kwargs):
    """Proves one lint/format subject's `dx_results` shape as data.

    Creates a private `real_source_target` (`<name>_fixture`) from `srcs`,
    a `real_aspect_subject` (`<name>_subject`) over it, and a passing
    `starlark_test` (`<name>`) diffing the subject's `DxSubjectInfo`
    against `expected_observations`. Only golden-mismatch fails; the
    fixture itself never fails the build.

    For dirty (expected-failure) coverage, prefer the matrix
    (`runner_matrix_cases.bzl`): provider-less bytes with `print_result`
    goldens, both `*_pass`/`*_fail` green. This macro is for clean
    presence pins; dirty live subjects break `dx <check> --check //...`
    and belong in the matrix, not here."""
    real_source_target(
        name = name + "_fixture",
        **srcs
    )
    real_aspect_subject(
        name = name + "_subject",
        target = ":" + name + "_fixture",
    )
    kwargs.setdefault("size", "small")
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":" + name + "_subject"],
        expected_observations = expected_observations,
        **kwargs
    )
