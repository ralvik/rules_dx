"""Focused Vue environment-plan tests."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //vue/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.vue
field has_npm=False
field npm_source_count=0
field target=//vue/tests/fixtures/hello:hello_lib
field transitive_sources=Hello.vue
aspect_field aspect_seen=True
aspect_field field_count=5
aspect_field has_subject=True
aspect_field subject_label=//vue/env:hello_lib_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
