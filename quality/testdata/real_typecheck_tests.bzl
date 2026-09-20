"""Real typecheck aspect evidence tests (WP3).
"""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_TYPECHECK_OBSERVATIONS = """subject //quality/testdata:fixture_real_python_no_typecheck_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=True
field label=//quality/testdata:fixture_real_python_no_typecheck
subject //quality/testdata:fixture_real_python_typecheck_subject
field dx_count=1
field dx_results=fixture_real_python-real-typecheck.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_python
subject //quality/testdata:fixture_real_rust_typecheck_clean_subject
field dx_count=1
field dx_results=fixture_real_rust_typecheck_clean-real-typecheck-rust.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_typecheck_clean
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
