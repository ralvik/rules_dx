"""Hermetic expected-failure proofs (issue #406).
"""

load("@bazel_skylib//lib:unittest.bzl", "analysistest", "asserts")

def _starlark_failure_impl(ctx):
    env = analysistest.begin(ctx)
    asserts.expect_failure(env, ctx.attr.expected_failure_substring)
    return analysistest.end(env)

_starlark_failure_test = analysistest.make(
    _starlark_failure_impl,
    expect_failure = True,
    attrs = {
        "expected_failure_substring": attr.string(
            mandatory = True,
            doc = "Required substring of the analysis failure message.",
        ),
    },
)

def failure_test(name, target, expected_failure_substring, **kwargs):
    """Asserts one analysis-time `fail()` fires with the expected diagnostic.

    The target under test must carry `tags = ["manual"]` so wildcard builds
    skip it; this test (non-manual) depends on it via the
    `allow_analysis_failures` transition and passes only when analysis
    fails with the expected substring. A fixed subject (no failure) fails
    this test."""
    kwargs.setdefault("size", "small")
    _starlark_failure_test(
        name = name,
        target_under_test = target,
        expected_failure_substring = expected_failure_substring,
        **kwargs
    )
