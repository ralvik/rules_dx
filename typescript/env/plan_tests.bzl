"""Focused TypeScript environment-plan tests (M16 WP3)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //typescript/env:*_plan` JSON outputs (M16 WP3).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //typescript/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.ts
field has_npm=False
field has_tsconfig=True
field npm_source_count=0
field target=//typescript/hello:hello_lib
field transitive_sources=hello.js
subject //typescript/env:helper_plan
file helper_plan.json
field direct_sources=helper.ts
field has_npm=False
field has_tsconfig=True
field npm_source_count=0
field target=//typescript/entries:helper
field transitive_sources=helper.js
subject //typescript/env:main_plan
file main_plan.json
field direct_sources=main.ts
field has_npm=False
field has_tsconfig=True
field npm_source_count=0
field target=//typescript/entries:main
field transitive_sources=helper.js,main.js"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
