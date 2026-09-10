"""Behavioral-matrix mapping validation (M01 WP3).

Every inventory item in `behavioral_matrix.md` must be present there (via
its `matrix-item:` anchor) and must name a proof source file that mentions
the item. A removed anchor or a renamed proof file fails this test, so
missing mappings cannot pass silently. Assertion quality beyond presence is
review-based; see the matrix file.
"""

load("//libs/starlark:defs.bzl", "starlark_test")

_MATRIX = "//libs/starlark:behavioral_matrix.md"

def matrix_validation_tests(name):
    starlark_test(
        name = name,
        mode = "execution",
        file_checks = {
            _MATRIX: "matrix-item: expect_equal\nmatrix-item: starlark_test-facade\nmatrix-item: load-mode\nmatrix-item: unit-mode\nmatrix-item: analysis-mode\nmatrix-item: execution-mode\nmatrix-item: dx-subject-info\nmatrix-item: failure-rendering\nmatrix-item: mode-validation\nmatrix-item: tested-stack",
            "//libs/starlark:defs.bzl": "def starlark_test\ndef expect_equal\ndef _load_test_impl\ndef _unit_test_impl\ndef _analysis_test_impl\ndef _execution_test_impl\nmust be empty",
            "//libs/starlark/tests:arithmetic_tests.bzl": "expect_equal",
            "//libs/starlark/tests:analysis_tests.bzl": "expected_observations",
            "//libs/starlark/tests:execution_tests.bzl": "file_checks",
            "//libs/starlark/tests:subject.bzl": "DxSubjectInfo",
            "//libs/starlark/tests/negative:negative_tests.bzl": "deliberately wrong",
            "//libs/testing:tested_stack.bzl": "schema_version",
        },
    )
