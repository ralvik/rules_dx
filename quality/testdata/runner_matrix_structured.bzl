
BUF_LINT = """{"path": "matrix/buf_lint_dirty.proto", "start_line": 3, "start_column": 9, "end_line": 3, "end_column": 18, "type": "PACKAGE_DIRECTORY_MATCH", "message": "Files with package fixtures.buf must be in a directory fixtures/buf."}"""

QMLLINT_LINT = """{"diagnostics": [{"file": "matrix/qmllint_dirty.qml", "line": 4, "column": 5, "rule": "unqualified", "message": "Unqualified access to foo.", "severity": "warning"}]}"""

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
        "expected": """producer //quality/testdata:matrix_protobuf_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_protobuf_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_protobuf_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_protobuf_lint_fail""",
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
        "expected": """producer //quality/testdata:matrix_qml_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_qml_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_qml_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_qml_lint_fail""",
    },
]
