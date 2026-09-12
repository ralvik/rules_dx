"""Real aspect evidence tests (M04 WP2).

Pins merged real dx_results presence per fixture: exact capability subsets,
no generic fallback, no empty actions, tags honored, Vale hinted. Per-stage
exact source inputs are proven via aquery action inputs; these analysis pins
prove the capability-level shape resting on the real pipeline unit tests.
"""

load("//libs/starlark:defs.bzl", "starlark_test")

EXPECTED_OBSERVATIONS = """subject //quality/testdata:fixture_real_javascript_dirty_subject
field dx_count=2
field dx_results=fixture_real_javascript_dirty-real-format.pb,fixture_real_javascript_dirty-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_javascript_dirty
subject //quality/testdata:fixture_real_javascript_hinted_subject
field dx_count=2
field dx_results=fixture_real_javascript_hinted-real-format.pb,fixture_real_javascript_hinted-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_javascript_hinted
subject //quality/testdata:fixture_real_javascript_subject
field dx_count=2
field dx_results=fixture_real_javascript-real-format.pb,fixture_real_javascript-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_javascript
subject //quality/testdata:fixture_real_markdown_sibling_subject
field dx_count=1
field dx_results=fixture_real_markdown_sibling-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_markdown_sibling
subject //quality/testdata:fixture_real_markdown_subject
field dx_count=1
field dx_results=fixture_real_markdown-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_markdown
subject //quality/testdata:fixture_real_mixed_subject
field dx_count=2
field dx_results=fixture_real_mixed-real-format.pb,fixture_real_mixed-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_mixed
subject //quality/testdata:fixture_real_no_lint_subject
field dx_count=1
field dx_results=fixture_real_no_lint-real-format.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_no_lint
subject //quality/testdata:fixture_real_python_dirty_subject
field dx_count=2
field dx_results=fixture_real_python_dirty-real-format.pb,fixture_real_python_dirty-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_python_dirty
subject //quality/testdata:fixture_real_python_hinted_subject
field dx_count=2
field dx_results=fixture_real_python_hinted-real-format.pb,fixture_real_python_hinted-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_python_hinted
subject //quality/testdata:fixture_real_python_subject
field dx_count=2
field dx_results=fixture_real_python-real-format.pb,fixture_real_python-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_python
subject //quality/testdata:fixture_real_rust_generated_shape_subject
field dx_count=2
field dx_results=fixture_real_rust_generated_shape-real-format.pb,fixture_real_rust_generated_shape-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_generated_shape
subject //quality/testdata:fixture_real_rust_hinted_subject
field dx_count=2
field dx_results=fixture_real_rust_hinted-real-format.pb,fixture_real_rust_hinted-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust_hinted
subject //quality/testdata:fixture_real_rust_subject
field dx_count=2
field dx_results=fixture_real_rust-real-format.pb,fixture_real_rust-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_rust
subject //quality/testdata:fixture_real_starlark_subject
field dx_count=2
field dx_results=fixture_real_starlark-real-format.pb,fixture_real_starlark-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_starlark
subject //quality/testdata:fixture_real_toml_subject
field dx_count=2
field dx_results=fixture_real_toml-real-format.pb,fixture_real_toml-real-lint.pb
field has_quality_sources=True
field label=//quality/testdata:fixture_real_toml
subject //quality/testdata:real_plain_subject
field dx_count=0
field dx_results=(none)
field has_quality_sources=False
field label=//quality/testdata:plain"""

def real_aspect_fixture_tests(name, subjects):
    starlark_test(
        name = name,
        mode = "analysis",
        subjects = subjects,
        expected_observations = EXPECTED_OBSERVATIONS,
    )
