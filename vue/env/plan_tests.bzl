"""Focused Vue environment-plan tests (M18 WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //vue/env:*_plan` JSON outputs (M18 WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //vue/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.vue
field has_npm=False
field npm_source_count=0
field target=//vue/hello:hello_lib
field transitive_sources=Hello.vue"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
