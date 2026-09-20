"""Layer-2 matrix cases (snapshot workflow): every supported language x capability cell.

Contract: `docs/quality/runner-matrix.md`.
Split into per-language files with no behavior change.
"""

load(":runner_matrix_tests.bzl", "runner_matrix_suite")
load(":runner_matrix_rust.bzl", "RUST_CASES")
load(":runner_matrix_python.bzl", "PYTHON_CASES")
load(":runner_matrix_js.bzl", "JS_CASES")
load(":runner_matrix_ts.bzl", "TS_CASES")
load(":runner_matrix_json.bzl", "JSON_CASES")
load(":runner_matrix_data.bzl", "DATA_CASES")
load(":runner_matrix_markdown.bzl", "MARKDOWN_CASES")

def runner_matrix_cases(name):
    runner_matrix_suite(name, RUST_CASES + PYTHON_CASES + JS_CASES + TS_CASES + JSON_CASES + DATA_CASES + MARKDOWN_CASES)
