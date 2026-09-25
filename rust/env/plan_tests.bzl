"""Focused Rust environment-plan tests (WP3).
"""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //rust/env:hello_cdylib_plan
file hello_cdylib_plan.json
field crate_name=hello_cdylib
field crate_type=cdylib
field direct_dep_count=0
field direct_sources=cdylib.rs
field edition=2021
field root=cdylib.rs
field target=//rust/tests/fixtures/hello:hello_cdylib
field via_test_crate=True
aspect_field aspect_seen=True
aspect_field field_count=8
aspect_field has_subject=True
aspect_field subject_label=//rust/env:hello_cdylib_plan
aspect_field transitive_count=0
subject //rust/env:hello_lib_plan
file hello_lib_plan.json
field crate_name=hello
field crate_type=rlib
field direct_dep_count=1
field direct_sources=lib.rs
field edition=2021
field root=lib.rs
field target=//rust/tests/fixtures/hello:hello_lib
field via_test_crate=False
aspect_field aspect_seen=True
aspect_field field_count=8
aspect_field has_subject=True
aspect_field subject_label=//rust/env:hello_lib_plan
aspect_field transitive_count=0
subject //rust/env:hello_plan
file hello_plan.json
field crate_name=hello
field crate_type=bin
field direct_dep_count=2
field direct_sources=main.rs
field edition=2021
field root=main.rs
field target=//rust/tests/fixtures/hello:hello
field via_test_crate=False
aspect_field aspect_seen=True
aspect_field field_count=8
aspect_field has_subject=True
aspect_field subject_label=//rust/env:hello_plan
aspect_field transitive_count=0
subject //rust/env:hello_test_plan
file hello_test_plan.json
field crate_name=hello
field crate_type=bin
field direct_dep_count=1
field direct_sources=
field edition=2021
field root=lib.rs
field target=//rust/tests/fixtures/hello:hello_test
field via_test_crate=False
aspect_field aspect_seen=True
aspect_field field_count=8
aspect_field has_subject=True
aspect_field subject_label=//rust/env:hello_test_plan
aspect_field transitive_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
