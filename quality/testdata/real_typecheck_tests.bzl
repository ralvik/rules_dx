"""Real typecheck aspect evidence tests (M12 WP3).

Pins merged real typecheck dx_results presence per fixture: rust
fixtures gain exactly one typecheck action, the `no-typecheck` tag
opts out, and non-rust fixtures gain none. Per-stage exact source
inputs are proven via aquery action inputs; these analysis pins prove
the capability-level shape resting on the real pipeline unit tests.
"""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_TYPECHECK_OBSERVATIONS = """subject //quality/testdata:fixture_real_rust_typecheck_clean_subject
field dx_count=1
field dx_results=fixture_real_rust_typecheck_clean-real-typecheck.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_typecheck_clean
subject //quality/testdata:fixture_real_rust_typecheck_dirty_subject
field dx_count=1
field dx_results=fixture_real_rust_typecheck_dirty-real-typecheck.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_typecheck_dirty
subject //quality/testdata:fixture_real_rust_typecheck_no_typecheck_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_typecheck_no_typecheck
subject //quality/testdata:fixture_real_starlark_typecheck_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=True
field label=//quality/testdata:fixture_real_starlark"""

def real_typecheck_fixture_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_TYPECHECK_OBSERVATIONS,
    )
