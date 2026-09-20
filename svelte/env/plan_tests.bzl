"""Focused Svelte environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //svelte/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //svelte/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.svelte
field has_npm=False
field npm_source_count=0
field target=//svelte/tests/fixtures/hello:hello_lib
field transitive_sources=Hello.svelte"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
