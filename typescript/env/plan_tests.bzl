"""Focused TypeScript environment-plan tests."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //typescript/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.ts,main.ts
field has_npm=False
field has_store=False
field has_tsconfig=True
field npm_source_count=0
field store_count=0
field target=//typescript/tests/fixtures/hello:hello_lib
field transitive_sources=hello.js,main.js
field tsconfig=tsconfig.json
field tsconfig_count=1
aspect_field aspect_seen=True
aspect_field field_count=10
aspect_field has_subject=True
aspect_field subject_label=//typescript/env:hello_lib_plan
aspect_field transitive_count=0
subject //typescript/env:helper_plan
file helper_plan.json
field direct_sources=helper.ts
field has_npm=False
field has_store=False
field has_tsconfig=True
field npm_source_count=0
field store_count=0
field target=//typescript/tests/fixtures/entries:helper
field transitive_sources=helper.js
field tsconfig=tsconfig.json
field tsconfig_count=1
aspect_field aspect_seen=True
aspect_field field_count=10
aspect_field has_subject=True
aspect_field subject_label=//typescript/env:helper_plan
aspect_field transitive_count=0
subject //typescript/env:main_plan
file main_plan.json
field direct_sources=main.ts
field has_npm=False
field has_store=False
field has_tsconfig=True
field npm_source_count=0
field store_count=0
field target=//typescript/tests/fixtures/entries:main
field transitive_sources=helper.js,main.js
field tsconfig=tsconfig.json
field tsconfig_count=1
aspect_field aspect_seen=True
aspect_field field_count=10
aspect_field has_subject=True
aspect_field subject_label=//typescript/env:main_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
