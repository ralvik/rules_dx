"""Matrix data cases (split from runner_matrix_cases.bzl)."""

DATA_CASES = [
    {
        "name": "matrix_starlark_lint_pass",
        "srcs": [":real_clean.bzl"],
        "capability": "lint",
        "stages": ["buildifier;starlark;quality/testdata/real_clean.bzl"],
        "tool_names": ["buildifier"],
        "tool_binaries": ["@dx_tools//:buildifier"],
        "expected": """producer //quality/testdata:matrix_starlark_lint_pass
capability LINT
stages 1
stage buildifier classes=starlark sources=quality/testdata/real_clean.bzl
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_starlark_lint_fail",
        "generated": {
            "matrix/starlark_dirty.bzl": "x=1\n",
        },
        "capability": "lint",
        "stages": ["buildifier;starlark;matrix/starlark_dirty.bzl"],
        "tool_names": ["buildifier"],
        "tool_binaries": ["@dx_tools//:buildifier"],
        "expected": """producer //quality/testdata:matrix_starlark_lint_fail
capability LINT
stages 1
stage buildifier classes=starlark sources=matrix/starlark_dirty.bzl
completed_rounds 2
convergence STABLE
initial 1
initial WARNING buildifier module-docstring matrix/starlark_dirty.bzl 0 1 fixable=false "The file has no module docstring.\\nA module docstring is a string literal (not a comment) which should be the first statement of a file (it may follow comment lines)."
terminal 1
terminal WARNING buildifier module-docstring matrix/starlark_dirty.bzl 0 1 fixable=false "The file has no module docstring.\\nA module docstring is a string literal (not a comment) which should be the first statement of a file (it may follow comment lines)."
replacements 1
replacement matrix/starlark_dirty.bzl 1 2 " = "
""",
    },
    {
        "name": "matrix_starlark_format_pass",
        "srcs": [":real_clean.bzl"],
        "capability": "format",
        "stages": ["buildifier;starlark;quality/testdata/real_clean.bzl"],
        "tool_names": ["buildifier"],
        "tool_binaries": ["@dx_tools//:buildifier"],
        "expected": """producer //quality/testdata:matrix_starlark_format_pass
capability FORMAT
stages 1
stage buildifier classes=starlark sources=quality/testdata/real_clean.bzl
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_starlark_format_fail",
        "generated": {
            "matrix/starlark_dirty.bzl": "x=1\n",
        },
        "capability": "format",
        "stages": ["buildifier;starlark;matrix/starlark_dirty.bzl"],
        "tool_names": ["buildifier"],
        "tool_binaries": ["@dx_tools//:buildifier"],
        "expected": """producer //quality/testdata:matrix_starlark_format_fail
capability FORMAT
stages 1
stage buildifier classes=starlark sources=matrix/starlark_dirty.bzl
completed_rounds 2
convergence STABLE
initial 1
initial WARNING buildifier - matrix/starlark_dirty.bzl 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/starlark_dirty.bzl 1 2 " = "
""",
    },
    {
        "name": "matrix_toml_lint_pass",
        "srcs": [":real_clean.toml"],
        "capability": "lint",
        "stages": ["taplo;toml;quality/testdata/real_clean.toml"],
        "tool_names": ["taplo"],
        "tool_binaries": ["@dx_tools//:taplo"],
        "expected": """producer //quality/testdata:matrix_toml_lint_pass
capability LINT
stages 1
stage taplo classes=toml sources=quality/testdata/real_clean.toml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_toml_lint_fail",
        "generated": {
            "matrix/toml_dirty.toml": "a=1\n",
        },
        "capability": "lint",
        "stages": ["taplo;toml;matrix/toml_dirty.toml"],
        "tool_names": ["taplo"],
        "tool_binaries": ["@dx_tools//:taplo"],
        "expected": """producer //quality/testdata:matrix_toml_lint_fail
capability LINT
stages 1
stage taplo classes=toml sources=matrix/toml_dirty.toml
completed_rounds 2
convergence STABLE
initial 0
terminal 0
replacements 1
replacement matrix/toml_dirty.toml 1 2 " = "
""",
    },
    {
        "name": "matrix_toml_format_pass",
        "srcs": [":real_clean.toml"],
        "capability": "format",
        "stages": ["taplo;toml;quality/testdata/real_clean.toml"],
        "tool_names": ["taplo"],
        "tool_binaries": ["@dx_tools//:taplo"],
        "expected": """producer //quality/testdata:matrix_toml_format_pass
capability FORMAT
stages 1
stage taplo classes=toml sources=quality/testdata/real_clean.toml
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_toml_format_fail",
        "generated": {
            "matrix/toml_dirty.toml": "a=1\n",
        },
        "capability": "format",
        "stages": ["taplo;toml;matrix/toml_dirty.toml"],
        "tool_names": ["taplo"],
        "tool_binaries": ["@dx_tools//:taplo"],
        "expected": """producer //quality/testdata:matrix_toml_format_fail
capability FORMAT
stages 1
stage taplo classes=toml sources=matrix/toml_dirty.toml
completed_rounds 2
convergence STABLE
initial 1
initial WARNING taplo - matrix/toml_dirty.toml 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/toml_dirty.toml 1 2 " = "
""",
    },
]
