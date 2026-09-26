"""Analysis tests pinning the frozen quality settings defaults."""

load("//libs/starlark:defs.bzl", "starlark_test")

_CANONICAL_PREFIX = "@@"  # buildifier: disable=canonical-repository

EXPECTED_OBSERVATIONS = """subject //config:settings_under_test
field fail_on=warning
field validate=False
field workspace={prefix}//dx:config
aspect_field aspect_seen=True
aspect_field field_count=3
aspect_field has_subject=True
aspect_field subject_label=//config:settings_under_test
aspect_field transitive_count=0""".format(prefix = _CANONICAL_PREFIX)

def settings_tests(name):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = [":settings_under_test"],
        expected_observations = EXPECTED_OBSERVATIONS,
    )
