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
        ),
    },
)

def failure_test(name, target, expected_failure_substring, **kwargs):
    kwargs.setdefault("size", "small")
    _starlark_failure_test(
        name = name,
        target_under_test = target,
        expected_failure_substring = expected_failure_substring,
        **kwargs
    )
