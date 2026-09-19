"""Focused Astro environment-plan tests (M20 WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //astro/env:*_plan` JSON outputs (M20 WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //astro/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.astro
field has_npm=False
field npm_source_count=0
field target=//astro/tests/fixtures/hello:hello_lib
field transitive_sources=Hello.astro"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
