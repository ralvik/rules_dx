"""Focused JavaScript environment-plan tests (M16 WP3)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //javascript/env:*_plan` JSON outputs (M16 WP3).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //javascript/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.js
field has_npm=False
field npm_source_count=0
field target=//javascript/hello:hello_lib
field transitive_sources=hello.js"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
