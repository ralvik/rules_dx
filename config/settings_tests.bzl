"""Analysis tests pinning the frozen quality settings defaults (M03 WP1)."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //config:settings_under_test
field fail_on=warning
field validate=False
field workspace=@@//dx:config"""

def settings_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":settings_under_test"],
        expected_observations = EXPECTED_OBSERVATIONS,
    )
