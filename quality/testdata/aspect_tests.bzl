"""Aspect evidence tests."""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //quality/testdata:fixture_mixed_clean_subject
field dx_count=2
field dx_results=fixture_mixed_clean-format.pb,fixture_mixed_clean-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_mixed_clean
aspect_field aspect_seen=True
aspect_field field_count=4
aspect_field has_subject=True
aspect_field subject_label=//quality/testdata:fixture_mixed_clean_subject
aspect_field transitive_count=0
subject //quality/testdata:fixture_no_lint_subject
field dx_count=1
field dx_results=fixture_no_lint-format.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_no_lint
aspect_field aspect_seen=True
aspect_field field_count=4
aspect_field has_subject=True
aspect_field subject_label=//quality/testdata:fixture_no_lint_subject
aspect_field transitive_count=0
subject //quality/testdata:fixture_python_subject
field dx_count=1
field dx_results=fixture_python-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_python
aspect_field aspect_seen=True
aspect_field field_count=4
aspect_field has_subject=True
aspect_field subject_label=//quality/testdata:fixture_python_subject
aspect_field transitive_count=0
subject //quality/testdata:fixture_rust_subject
field dx_count=2
field dx_results=fixture_rust-format.pb,fixture_rust-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_rust
aspect_field aspect_seen=True
aspect_field field_count=4
aspect_field has_subject=True
aspect_field subject_label=//quality/testdata:fixture_rust_subject
aspect_field transitive_count=0
subject //quality/testdata:plain_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=False
field label=//quality/testdata:plain
aspect_field aspect_seen=True
aspect_field field_count=4
aspect_field has_subject=True
aspect_field subject_label=//quality/testdata:plain_subject
aspect_field transitive_count=0"""

def aspect_fixture_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_OBSERVATIONS,
    )
