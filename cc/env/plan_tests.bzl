"""Focused C/C++ environment-plan tests (WP2)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //cc/env:*_plan` JSON outputs (WP2).
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //cc/env:hello_c_lib_plan
file hello_c_lib_plan.json
field direct_sources=hello.c,hello_c.h
field has_sources=True
field has_tests=False
field source_count=2
field target=//cc/tests/fixtures/hello:hello_c_lib
field test_source_count=0
field test_sources=
field transitive_source_count=1
field transitive_sources=hello_c.h
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//cc/env:hello_c_lib_plan
aspect_field transitive_count=0
subject //cc/env:hello_c_test_plan
file hello_c_test_plan.json
field direct_sources=hello_c_test.c
field has_sources=True
field has_tests=True
field source_count=1
field target=//cc/tests/fixtures/hello:hello_c_test
field test_source_count=1
field test_sources=hello_c_test.c
field transitive_source_count=1
field transitive_sources=hello_c.h
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//cc/env:hello_c_test_plan
aspect_field transitive_count=0
subject //cc/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.cc,hello.h
field has_sources=True
field has_tests=False
field source_count=2
field target=//cc/tests/fixtures/hello:hello_lib
field test_source_count=0
field test_sources=
field transitive_source_count=1
field transitive_sources=hello.h
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//cc/env:hello_lib_plan
aspect_field transitive_count=0
subject //cc/env:hello_plan
file hello_plan.json
field direct_sources=main.cc
field has_sources=True
field has_tests=False
field source_count=1
field target=//cc/tests/fixtures/hello:hello
field test_source_count=0
field test_sources=
field transitive_source_count=1
field transitive_sources=hello.h
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//cc/env:hello_plan
aspect_field transitive_count=0
subject //cc/env:hello_test_plan
file hello_test_plan.json
field direct_sources=hello_test.cc
field has_sources=True
field has_tests=True
field source_count=1
field target=//cc/tests/fixtures/hello:hello_test
field test_source_count=1
field test_sources=hello_test.cc
field transitive_source_count=1
field transitive_sources=hello.h
aspect_field aspect_seen=True
aspect_field field_count=9
aspect_field has_subject=True
aspect_field subject_label=//cc/env:hello_test_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
