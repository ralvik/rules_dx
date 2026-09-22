"""Focused Kotlin environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //kotlin/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //kotlin/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=Hello.kt
field has_sources=True
field has_tests=False
field source_count=1
field target=//kotlin/tests/fixtures/hello:hello_lib
field test_source_count=0
field test_sources=
field transitive_source_count=1
field transitive_sources=hello_lib_upstream-sources.jar
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//kotlin/env:hello_lib_plan
aspect_field transitive_count=0
subject //kotlin/env:hello_plan
file hello_plan.json
field direct_sources=Main.kt
field has_sources=True
field has_tests=False
field source_count=1
field target=//kotlin/tests/fixtures/hello:hello
field test_source_count=0
field test_sources=
field transitive_source_count=2
field transitive_sources=hello_lib_upstream-sources.jar,hello_upstream-sources.jar
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//kotlin/env:hello_plan
aspect_field transitive_count=0
subject //kotlin/env:hello_test_plan
file hello_test_plan.json
field direct_sources=HelloTest.kt
field has_sources=True
field has_tests=True
field source_count=1
field target=//kotlin/tests/fixtures/hello:hello_test
field test_source_count=2
field test_sources=HelloTest.kt,hello_test_upstream-sources.jar
field transitive_source_count=2
field transitive_sources=hello_lib_upstream-sources.jar,hello_test_upstream-sources.jar
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//kotlin/env:hello_test_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
