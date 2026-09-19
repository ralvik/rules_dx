"""Focused Kotlin environment-plan tests (M23 WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //kotlin/env:*_plan` JSON outputs (M23 WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //kotlin/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.kt
field has_sources=True
field source_count=1
field target=//kotlin/tests/fixtures/hello:hello_lib"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
