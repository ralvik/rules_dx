"""Focused Rust environment-plan tests (M12 WP3).

Pins the provider-derived plan surface for each wrapper shape: library and
binary plans read authoritative CrateInfo; the test plan reads the same
crate through TestCrateInfo. Expected observations pin crate identity,
edition, root, direct sources, and direct dependency counts.
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
field target=//rust/hello:hello_cdylib
field via_test_crate=True
subject //rust/env:hello_lib_plan
file hello_lib_plan.json
field crate_name=hello
field crate_type=rlib
field direct_dep_count=1
field direct_sources=lib.rs
field edition=2021
field root=lib.rs
field target=//rust/hello:hello_lib
field via_test_crate=False
subject //rust/env:hello_plan
file hello_plan.json
field crate_name=hello
field crate_type=bin
field direct_dep_count=2
field direct_sources=main.rs
field edition=2021
field root=main.rs
field target=//rust/hello:hello
field via_test_crate=False
subject //rust/env:hello_test_plan
file hello_test_plan.json
field crate_name=hello
field crate_type=bin
field direct_dep_count=1
field direct_sources=
field edition=2021
field root=lib.rs
field target=//rust/hello:hello_test
field via_test_crate=False"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
