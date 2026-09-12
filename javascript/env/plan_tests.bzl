"""Focused JavaScript environment-plan tests (M16 WP3)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //javascript/env:*_plan` JSON outputs (M16 WP3).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //javascript/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.js
field has_npm=False
field npm_source_count=0
field target=//javascript/hello:hello_lib
field transitive_sources=hello.js
subject //javascript/env:helper_plan
file helper_plan.json
field direct_sources=helper.js
field has_npm=False
field npm_source_count=0
field target=//javascript/entries:helper
field transitive_sources=helper.js
subject //javascript/env:main_plan
file main_plan.json
field direct_sources=main.js
field has_npm=False
field npm_source_count=0
field target=//javascript/entries:main
field transitive_sources=helper.js,main.js"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
