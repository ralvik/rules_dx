"""Hermetic expected-failure proofs (issue #406).

Wraps `analysistest` from `@bazel_skylib//lib:unittest.bzl` with
`expect_failure = True` for analysis-time `fail()`s: `wrong_phase_demo`
(`libs/starlark/defs.bzl`) and Vale-no-config
(`quality/real_aspects.bzl`). Failure lives inside a passing test body,
never as a failing target: the red subject stays `manual` (so
`bazel build //...` skips it per the analysistest contract), the
expect-failure test is non-manual and passes by asserting the exact
user-visible diagnostic. If the subject stops failing, the test fails.

Hermeticity: no nested Bazel, no `manual`/`local`/`exclusive`/`no-sandbox`
on the test, sandboxed and cacheable via the standard analysis-test
transition (`allow_analysis_failures`).
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
    this test.

    Args:
      name: passing test target name.
      target: failing subject label (manual).
      expected_failure_substring: required failure-message substring.
      **kwargs: extra attributes forwarded to the test rule.
    """
    kwargs.setdefault("size", "small")
    _starlark_failure_test(
        name = name,
        target_under_test = target,
        expected_failure_substring = expected_failure_substring,
        **kwargs
    )
