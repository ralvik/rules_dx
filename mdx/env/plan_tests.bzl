"""Focused MDX environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //mdx/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //mdx/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.mdx
field has_npm=False
field npm_source_count=0
field target=//mdx/tests/fixtures/hello:hello_lib
field transitive_sources=Hello.mdx
aspect_field aspect_seen=True
aspect_field field_count=5
aspect_field has_subject=True
aspect_field subject_label=//mdx/env:hello_lib_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
