"""Aspect evidence tests (M03 WP2c).

Pins merged dx_results presence per fixture: exact capability subsets,
no generic fallback, no empty actions, tags honored. Per-stage exact
source inputs are proven via aquery action inputs (see completion report);
these analysis pins prove the capability-level shape that rests on the
WP2a pure pipeline unit tests.
"""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //quality/testdata:fixture_mixed_clean_subject
field dx_count=2
field dx_results=fixture_mixed_clean-format.pb,fixture_mixed_clean-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_mixed_clean
subject //quality/testdata:fixture_mixed_subject
field dx_count=2
field dx_results=fixture_mixed-format.pb,fixture_mixed-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_mixed
subject //quality/testdata:fixture_no_lint_subject
field dx_count=1
field dx_results=fixture_no_lint-format.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_no_lint
subject //quality/testdata:fixture_python_subject
field dx_count=1
field dx_results=fixture_python-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_python
subject //quality/testdata:fixture_rust_subject
field dx_count=2
field dx_results=fixture_rust-format.pb,fixture_rust-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_rust
subject //quality/testdata:plain_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=False
field label=//quality/testdata:plain"""

def aspect_fixture_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_OBSERVATIONS,
    )
