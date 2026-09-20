"""Focused F# environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //fsharp/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //fsharp/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Library.fs
field has_sources=True
field source_count=1
field target=//fsharp/tests/fixtures/hello:hello_lib"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
