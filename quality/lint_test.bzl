"""Aspect-lint golden harness."""

load("//libs/starlark:defs.bzl", "starlark_test")
load("//quality:fixtures.bzl", "real_source_target")
load("//quality/testdata:real_aspect_subject.bzl", "real_aspect_subject")

def lint_test(name, srcs, expected_observations, **kwargs):
    """Proves one lint/format subject's dx_results shape as data."""
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
