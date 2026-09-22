"""Structured cohort matrix cells (issue #799).

Contract: `docs/quality/runner-matrix.md`.
Seed-only wiring proof with fake doubles plus recorded diagnostics.
"""

# Recorded Buf lint JSONL for the lint-fail cell, shaped like the
# parser unit samples (`quality/adapter/src/parsers/buf.rs`):
# workspace-relative path, type plus positions. Injected verbatim.
BUF_LINT = """{"path": "matrix/buf_lint_dirty.proto", "start_line": 3, "start_column": 9, "end_line": 3, "end_column": 18, "type": "PACKAGE_DIRECTORY_MATCH", "message": "Files with package fixtures.buf must be in a directory fixtures/buf."}"""

# Recorded qmllint JSON for the lint-fail cell, shaped like the parser
# unit samples (`quality/adapter/src/parsers/qmllint.rs`).
QMLLINT_LINT = """{"diagnostics": [{"file": "matrix/qmllint_dirty.qml", "line": 4, "column": 5, "rule": "unqualified", "message": "Unqualified access to `foo`.", "severity": "warning"}]}"""

STRUCTURED_CASES = [
    {
        "name": "matrix_protobuf_format_pass",
        "generated": {
            "matrix/buf_clean.proto": "syntax = \"proto3\";\n\npackage fixtures.buf;\n\nmessage Sample {\n  string greet = 1;\n}\n",
        },
        "capability": "format",
        "stages": ["buf;protobuf;matrix/buf_clean.proto"],
        "tool_names": ["buf"],
        "tool_binaries": ["//quality/testdata:fake_buf_format"],
        "expected": """producer //quality/testdata:matrix_protobuf_format_pass
capability FORMAT
stages 1
stage buf classes=protobuf sources=matrix/buf_clean.proto
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_protobuf_format_fail",
        "generated": {
            "matrix/buf_dirty.proto": "syntax = \"proto3\";\n\npackage fixtures.buf; BADFMT\n",
        },
        "capability": "format",
        "stages": ["buf;protobuf;matrix/buf_dirty.proto"],
        "tool_names": ["buf"],
        "tool_binaries": ["//quality/testdata:fake_buf_format"],
        "expected": """producer //quality/testdata:matrix_protobuf_format_fail
capability FORMAT
stages 1
stage buf classes=protobuf sources=matrix/buf_dirty.proto
completed_rounds 2
convergence STABLE
initial 1
initial WARNING buf - matrix/buf_dirty.proto 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/buf_dirty.proto 0 49 "syntax = \\"proto3\\";\\n\\npackage fixtures.buf; fixed\\n"
""",
    },
    {
        "name": "matrix_protobuf_lint_pass",
        "generated": {
            "matrix/buf_lint_clean.proto": "syntax = \"proto3\";\n\npackage fixtures.buf;\n\nmessage Sample {\n  string greet = 1;\n}\n",
        },
        "capability": "lint",
        "stages": ["buf;protobuf;matrix/buf_lint_clean.proto"],
        "upstream_tools": ["buf"],
        "upstream_generated": {"buf": ""},
        "expected": """producer //quality/testdata:matrix_protobuf_lint_pass
capability LINT
stages 1
stage buf classes=protobuf sources=matrix/buf_lint_clean.proto
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_protobuf_lint_fail",
        "generated": {
            "matrix/buf_lint_dirty.proto": "syntax = \"proto3\";\n\npackage fixtures.buf;\n\nmessage Sample {\n  string greet = 1;\n}\n",
        },
        "capability": "lint",
        "stages": ["buf;protobuf;matrix/buf_lint_dirty.proto"],
        "upstream_tools": ["buf"],
        "upstream_generated": {"buf": BUF_LINT},
        "expected": """producer //quality/testdata:matrix_protobuf_lint_fail
capability LINT
stages 1
stage buf classes=protobuf sources=matrix/buf_lint_dirty.proto
completed_rounds 1
convergence STABLE
initial 1
initial ERROR buf PACKAGE_DIRECTORY_MATCH matrix/buf_lint_dirty.proto 28 37 fixable=false "Files with package fixtures.buf must be in a directory fixtures/buf."
terminal 1
terminal ERROR buf PACKAGE_DIRECTORY_MATCH matrix/buf_lint_dirty.proto 28 37 fixable=false "Files with package fixtures.buf must be in a directory fixtures/buf."
replacements 0
""",
    },
    {
        "name": "matrix_qml_format_pass",
        "generated": {
            "matrix/qmlformat_clean.qml": "import QtQuick\n\nItem {\n    property string greet: \"hello\"\n}\n",
        },
        "capability": "format",
        "stages": ["qmlformat;qml;matrix/qmlformat_clean.qml"],
        "tool_names": ["qmlformat"],
        "tool_binaries": ["//quality/testdata:fake_qmlformat"],
        "expected": """producer //quality/testdata:matrix_qml_format_pass
capability FORMAT
stages 1
stage qmlformat classes=qml sources=matrix/qmlformat_clean.qml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_qml_format_fail",
        "generated": {
            "matrix/qmlformat_dirty.qml": "import QtQuick\n\nItem { BADFMT }\n",
        },
        "capability": "format",
        "stages": ["qmlformat;qml;matrix/qmlformat_dirty.qml"],
        "tool_names": ["qmlformat"],
        "tool_binaries": ["//quality/testdata:fake_qmlformat"],
        "expected": """producer //quality/testdata:matrix_qml_format_fail
capability FORMAT
stages 1
stage qmlformat classes=qml sources=matrix/qmlformat_dirty.qml
completed_rounds 2
convergence STABLE
initial 1
initial WARNING qmlformat - matrix/qmlformat_dirty.qml 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/qmlformat_dirty.qml 0 32 "import QtQuick\\n\\nItem { fixed }\\n"
""",
    },
    {
        "name": "matrix_qml_lint_pass",
        "generated": {
            "matrix/qmllint_clean.qml": "import QtQuick\n\nItem {\n    property string greet: \"hello\"\n}\n",
        },
        "capability": "lint",
        "stages": ["qmllint;qml;matrix/qmllint_clean.qml"],
        "upstream_tools": ["qmllint"],
        "upstream_generated": {"qmllint": """{"diagnostics": []}"""},
        "expected": """producer //quality/testdata:matrix_qml_lint_pass
capability LINT
stages 1
stage qmllint classes=qml sources=matrix/qmllint_clean.qml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_qml_lint_fail",
        "generated": {
            "matrix/qmllint_dirty.qml": "import QtQuick\n\nItem {\n    property string greet: foo\n}\n",
        },
        "capability": "lint",
        "stages": ["qmllint;qml;matrix/qmllint_dirty.qml"],
        "upstream_tools": ["qmllint"],
        "upstream_generated": {"qmllint": QMLLINT_LINT},
        "expected": """producer //quality/testdata:matrix_qml_lint_fail
capability LINT
stages 1
stage qmllint classes=qml sources=matrix/qmllint_dirty.qml
completed_rounds 1
convergence STABLE
initial 1
initial WARNING qmllint unqualified matrix/qmllint_dirty.qml 27 27 fixable=false "Unqualified access to `foo`."
terminal 1
terminal WARNING qmllint unqualified matrix/qmllint_dirty.qml 27 27 fixable=false "Unqualified access to `foo`."
replacements 0
""",
    },
]
