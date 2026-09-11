"""Focused Python environment-plan tests (M14 WP3)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# Pinned from `bazel build //python/env:*_plan` JSON outputs (M14 WP3).
# Source-only targets project no wheels; the pytest target projects the
# six uv wheel records (coverage, iniconfig, packaging, pluggy, pygments,
# pytest) its venv assembles.
EXPECTED_ENV_PLAN_OBSERVATIONS = """subject //python/env:hello_lib_plan
file hello_lib_plan.json
field direct_sources=hello.py
field has_wheels=False
field imports=_main,_main/python/hello
field target=//python/hello:hello_lib
field transitive_sources=hello.py
field wheel_count=0
subject //python/env:hello_plan
file hello_plan.json
field direct_sources=main.py
field has_wheels=False
field imports=_main,_main/python/hello
field target=//python/hello:hello
field transitive_sources=hello.py,main.py
field wheel_count=0
subject //python/env:hello_test_plan
file hello_test_plan.json
field direct_sources=hello_test.py
field has_wheels=True
field imports=_main,_main/python/hello,aspect_rules_py+,aspect_rules_py++uv+project__hello,aspect_rules_py++uv+whl_install__hello__coverage__7_16_0/actual_install.install/lib/python3.12/site-packages,aspect_rules_py++uv+whl_install__hello__iniconfig__2_3_0/actual_install.install/lib/python3.12/site-packages,aspect_rules_py++uv+whl_install__hello__packaging__26_3/actual_install.install/lib/python3.12/site-packages,aspect_rules_py++uv+whl_install__hello__pluggy__1_6_0/actual_install.install/lib/python3.12/site-packages,aspect_rules_py++uv+whl_install__hello__pygments__2_21_0/actual_install.install/lib/python3.12/site-packages,aspect_rules_py++uv+whl_install__hello__pytest__9_1_1/actual_install.install/lib/python3.12/site-packages,aspect_rules_py+/py/private,aspect_rules_py+/py/private/launcher_env,aspect_rules_py+/py/private/pytest_shard
field target=//python/hello:hello_test
field transitive_sources=aspect_rules_py_launcher_env.py,hello.py,hello_test.py,pytest_main.py,pytest_shard.py
field wheel_count=6
subject //python/env:main_bin_plan
file main_bin_plan.json
field direct_sources=
field has_wheels=False
field imports=_main,_main/python/entries
field target=//python/entries:main_bin
field transitive_sources=helper.py,main.py
field wheel_count=0
subject //python/env:main_plan
file main_plan.json
field direct_sources=main.py
field has_wheels=False
field imports=_main,_main/python/entries
field target=//python/entries:main
field transitive_sources=helper.py,main.py
field wheel_count=0"""

def env_plan_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_ENV_PLAN_OBSERVATIONS,
    )
