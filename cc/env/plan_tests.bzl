"""Focused C/C++ environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //cc/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //cc/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.cc,hello.h
field has_sources=True
field source_count=2
field target=//cc/tests/fixtures/hello:hello_lib"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
