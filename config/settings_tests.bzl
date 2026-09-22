"""Analysis tests pinning the frozen quality settings defaults (WP1)."""

load("//libs/starlark:defs.bzl", "starlark_test")

# buildifier: disable=canonical-repository  # issue #914: expected data pins Bazel's actual canonical rendering (not a probe; helper lives in //libs/starlark:canonical.bzl)
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
