"""Split from `runner_matrix_cases.bzl`. No behavior change."""

PYTHON_CASES = [
    {
        "name": "matrix_python_lint_pass",
        "srcs": [":real_clean.py"],
        "capability": "lint",
        "stages": [
            "pydoclint;python;quality/testdata/real_clean.py",
            "ruff;python;quality/testdata/real_clean.py",
        ],
        "tool_names": ["pydoclint", "ruff"],
        "tool_binaries": ["//quality/tools/python:pydoclint", "@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_lint_pass
capability LINT
stages 2
stage pydoclint classes=python sources=quality/testdata/real_clean.py
stage ruff classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_lint_fail",
        "srcs": [":real_dirty.py"],
        "capability": "lint",
        "stages": [
            "pydoclint;python;quality/testdata/real_dirty.py",
            "ruff;python;quality/testdata/real_dirty.py",
        ],
        "tool_names": ["pydoclint", "ruff"],
        "tool_binaries": ["//quality/tools/python:pydoclint", "@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_lint_fail
capability LINT
stages 2
stage pydoclint classes=python sources=quality/testdata/real_dirty.py
stage ruff classes=python sources=quality/testdata/real_dirty.py
completed_rounds 2
convergence STABLE
initial 3
initial ERROR ruff F401 quality/testdata/real_dirty.py 49 51 fixable=true "`os` imported but unused"
initial ERROR pydoclint DOC103 quality/testdata/real_dirty.py 54 54 fixable=false "Function `add`: Docstring arguments are different from function arguments. (Or could be other formatting issues: https://jsh9.github.io/pydoclint/violation_codes.html#notes-on-doc103 ). Arguments in the function signature but not in the docstring: [second: int]."
initial ERROR pydoclint DOC101 quality/testdata/real_dirty.py 54 54 fixable=false "Function `add`: Docstring contains fewer arguments than in function signature."
terminal 2
terminal ERROR pydoclint DOC103 quality/testdata/real_dirty.py 44 44 fixable=false "Function `add`: Docstring arguments are different from function arguments. (Or could be other formatting issues: https://jsh9.github.io/pydoclint/violation_codes.html#notes-on-doc103 ). Arguments in the function signature but not in the docstring: [second: int]."
terminal ERROR pydoclint DOC101 quality/testdata/real_dirty.py 44 44 fixable=false "Function `add`: Docstring contains fewer arguments than in function signature."
replacements 1
replacement quality/testdata/real_dirty.py 0 300 "\\"\\"\\"Real-pipeline dirty fixture (WP2).\\"\\"\\"\\n\\n\\n\\ndef add(first: int, second: int) -> int:\\n    \\"\\"\\"Add two integers.\\n\\n    Parameters\\n    ----------\\n    first : int\\n        First operand.\\n\\n    Returns\\n    -------\\n    int\\n        The sum.\\n    \\"\\"\\"\\n    return first + second\\n\\n\\nTOTAL:int=add(1, \\"two\\")\\n"
""",
    },
    {
        "name": "matrix_python_format_pass",
        "srcs": [":real_clean.py"],
        "capability": "format",
        "stages": ["ruff;python;quality/testdata/real_clean.py"],
        "tool_names": ["ruff"],
        "tool_binaries": ["@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_format_pass
capability FORMAT
stages 1
stage ruff classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_format_fail",
        "srcs": [":real_dirty.py"],
        "capability": "format",
        "stages": ["ruff;python;quality/testdata/real_dirty.py"],
        "tool_names": ["ruff"],
        "tool_binaries": ["@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_format_fail
capability FORMAT
stages 1
stage ruff classes=python sources=quality/testdata/real_dirty.py
completed_rounds 2
convergence STABLE
initial 1
initial ERROR ruff unformatted quality/testdata/real_dirty.py 282 286 fixable=true "File would be reformatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.py 0 300 "\\"\\"\\"Real-pipeline dirty fixture (WP2).\\"\\"\\"\\n\\nimport os\\n\\n\\ndef add(first: int, second: int) -> int:\\n    \\"\\"\\"Add two integers.\\n\\n    Parameters\\n    ----------\\n    first : int\\n        First operand.\\n\\n    Returns\\n    -------\\n    int\\n        The sum.\\n    \\"\\"\\"\\n    return first + second\\n\\n\\nTOTAL: int = add(1, \\"two\\")\\n"
""",
    },
    {
        "name": "matrix_python_typecheck_pass",
        "srcs": [":real_clean.py"],
        "capability": "typecheck",
        "stages": ["ty;python;quality/testdata/real_clean.py"],
        "tool_names": ["ty"],
        "tool_binaries": ["@dx_tools//:ty"],
        "expected": """producer //quality/testdata:matrix_python_typecheck_pass
capability TYPECHECK
stages 1
stage ty classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_typecheck_fail",
        "srcs": [":real_dirty.py"],
        "capability": "typecheck",
        "stages": ["ty;python;quality/testdata/real_dirty.py"],
        "tool_names": ["ty"],
        "tool_binaries": ["@dx_tools//:ty"],
        "expected": """producer //quality/testdata:matrix_python_typecheck_fail
capability TYPECHECK
stages 1
stage ty classes=python sources=quality/testdata/real_dirty.py
completed_rounds 1
convergence STABLE
initial 1
initial ERROR ty invalid-argument-type quality/testdata/real_dirty.py 293 293 fixable=false "Argument to function `add` is incorrect: Expected `int`, found `Literal[\\"two\\"]`"
terminal 1
terminal ERROR ty invalid-argument-type quality/testdata/real_dirty.py 293 293 fixable=false "Argument to function `add` is incorrect: Expected `int`, found `Literal[\\"two\\"]`"
replacements 0
""",
    },
    {
        "name": "matrix_python_stub_lint_pass",
        "generated": {
            "matrix/stub_clean.pyi": "\"\"\"Matrix stub fixture.\"\"\"\n\n\ndef add(first: int, second: int) -> int: ...\n",
        },
        "capability": "lint",
        "stages": [
            "pydoclint;python_stub;matrix/stub_clean.pyi",
            "ruff;python_stub;matrix/stub_clean.pyi",
        ],
        "tool_names": ["pydoclint", "ruff"],
        "tool_binaries": ["//quality/tools/python:pydoclint", "@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_stub_lint_pass
capability LINT
stages 2
stage pydoclint classes=python_stub sources=matrix/stub_clean.pyi
stage ruff classes=python_stub sources=matrix/stub_clean.pyi
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_stub_lint_fail",
        "generated": {
            "matrix/stub_dirty.pyi": "\"\"\"Matrix dirty stub fixture.\"\"\"\n\nimport os\n\n\ndef add(first: int, second: int) -> int: ...\n",
        },
        "capability": "lint",
        "stages": [
            "pydoclint;python_stub;matrix/stub_dirty.pyi",
            "ruff;python_stub;matrix/stub_dirty.pyi",
        ],
        "tool_names": ["pydoclint", "ruff"],
        "tool_binaries": ["//quality/tools/python:pydoclint", "@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_stub_lint_fail
capability LINT
stages 2
stage pydoclint classes=python_stub sources=matrix/stub_dirty.pyi
stage ruff classes=python_stub sources=matrix/stub_dirty.pyi
completed_rounds 2
convergence STABLE
initial 2
initial ERROR ruff I001 matrix/stub_dirty.pyi 34 43 fixable=true "Import block is un-sorted or un-formatted"
initial ERROR ruff F401 matrix/stub_dirty.pyi 41 43 fixable=true "`os` imported but unused"
terminal 0
replacements 1
replacement matrix/stub_dirty.pyi 0 91 "\\"\\"\\"Matrix dirty stub fixture.\\"\\"\\"\\n\\n\\n\\ndef add(first: int, second: int) -> int: ...\\n"
""",
    },
    {
        "name": "matrix_python_stub_format_pass",
        "generated": {
            "matrix/stub_format_clean.pyi": "\"\"\"Matrix stub fixture.\"\"\"\n\ndef add(first: int, second: int) -> int: ...\n",
        },
        "capability": "format",
        "stages": ["ruff;python_stub;matrix/stub_format_clean.pyi"],
        "tool_names": ["ruff"],
        "tool_binaries": ["@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_stub_format_pass
capability FORMAT
stages 1
stage ruff classes=python_stub sources=matrix/stub_format_clean.pyi
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_stub_format_fail",
        "generated": {
            "matrix/stub_format_dirty.pyi": "\"\"\"Matrix stub fixture.\"\"\"\ndef add(first:int,second:int)->int: ...\n",
        },
        "capability": "format",
        "stages": ["ruff;python_stub;matrix/stub_format_dirty.pyi"],
        "tool_names": ["ruff"],
        "tool_binaries": ["@dx_tools//:ruff"],
        "expected": """producer //quality/testdata:matrix_python_stub_format_fail
capability FORMAT
stages 1
stage ruff classes=python_stub sources=matrix/stub_format_dirty.pyi
completed_rounds 2
convergence STABLE
initial 1
initial ERROR ruff unformatted matrix/stub_format_dirty.pyi 27 58 fixable=true "File would be reformatted"
terminal 0
replacements 1
replacement matrix/stub_format_dirty.pyi 0 67 "\\"\\"\\"Matrix stub fixture.\\"\\"\\"\\n\\ndef add(first: int, second: int) -> int: ...\\n"
""",
    },
    {
        "name": "matrix_python_stub_typecheck_pass",
        "generated": {
            "matrix/stub_typecheck_clean.pyi": "\"\"\"Matrix stub fixture.\"\"\"\n\n\ndef add(first: int, second: int) -> int: ...\n",
        },
        "capability": "typecheck",
        "stages": ["ty;python_stub;matrix/stub_typecheck_clean.pyi"],
        "tool_names": ["ty"],
        "tool_binaries": ["@dx_tools//:ty"],
        "expected": """producer //quality/testdata:matrix_python_stub_typecheck_pass
capability TYPECHECK
stages 1
stage ty classes=python_stub sources=matrix/stub_typecheck_clean.pyi
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_stub_typecheck_fail",
        "generated": {
            "matrix/stub_typecheck_dirty.pyi": "\"\"\"Matrix stub fixture.\"\"\"\n\n\nvalue: int = \"two\"\n",
        },
        "capability": "typecheck",
        "stages": ["ty;python_stub;matrix/stub_typecheck_dirty.pyi"],
        "tool_names": ["ty"],
        "tool_binaries": ["@dx_tools//:ty"],
        "expected": """producer //quality/testdata:matrix_python_stub_typecheck_fail
capability TYPECHECK
stages 1
stage ty classes=python_stub sources=matrix/stub_typecheck_dirty.pyi
completed_rounds 1
convergence STABLE
initial 1
initial ERROR ty invalid-assignment matrix/stub_typecheck_dirty.pyi 42 42 fixable=false "Object of type `Literal[\\"two\\"]` is not assignable to `int`"
terminal 1
terminal ERROR ty invalid-assignment matrix/stub_typecheck_dirty.pyi 42 42 fixable=false "Object of type `Literal[\\"two\\"]` is not assignable to `int`"
replacements 0
""",
    },
    {
        "name": "matrix_python_ruff_hinted",
        "srcs": [":real_clean.py"],
        "capability": "lint",
        "stages": ["ruff;python;quality/testdata/real_clean.py"],
        "tool_names": ["ruff"],
        "tool_binaries": ["@dx_tools//:ruff"],
        "config_tools": ["ruff"],
        "config_files": [":ruff.toml"],
        "toolfile_tools": ["ruff"],
        "toolfile_srcs": [":ruff.toml"],
        "expected": """producer //quality/testdata:matrix_python_ruff_hinted
capability LINT
stages 1
stage ruff classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_flake8_pass",
        "srcs": [":real_clean.py"],
        "capability": "lint",
        "stages": ["flake8;python;quality/testdata/real_clean.py"],
        "tool_names": ["flake8"],
        "tool_binaries": ["//quality/tools/python:flake8"],
        "expected": """producer //quality/testdata:matrix_python_flake8_pass
capability LINT
stages 1
stage flake8 classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_flake8_fail",
        "srcs": [":real_dirty.py"],
        "capability": "lint",
        "stages": ["flake8;python;quality/testdata/real_dirty.py"],
        "tool_names": ["flake8"],
        "tool_binaries": ["//quality/tools/python:flake8"],
        "expected": """producer //quality/testdata:matrix_python_flake8_fail
capability LINT
stages 1
stage flake8 classes=python sources=quality/testdata/real_dirty.py
completed_rounds 1
convergence STABLE
initial 3
initial ERROR flake8 F401 quality/testdata/real_dirty.py 42 42 fixable=false "'os' imported but unused"
initial ERROR flake8 E231 quality/testdata/real_dirty.py 281 281 fixable=false "missing whitespace after ':'"
initial ERROR flake8 E225 quality/testdata/real_dirty.py 285 285 fixable=false "missing whitespace around operator"
terminal 3
terminal ERROR flake8 F401 quality/testdata/real_dirty.py 42 42 fixable=false "'os' imported but unused"
terminal ERROR flake8 E231 quality/testdata/real_dirty.py 281 281 fixable=false "missing whitespace after ':'"
terminal ERROR flake8 E225 quality/testdata/real_dirty.py 285 285 fixable=false "missing whitespace around operator"
replacements 0
""",
    },
    {
        "name": "matrix_python_pylint_pass",
        "srcs": [":real_clean.py"],
        "capability": "lint",
        "stages": ["pylint;python;quality/testdata/real_clean.py"],
        "tool_names": ["pylint"],
        "tool_binaries": ["//quality/tools/python:pylint"],
        "expected": """producer //quality/testdata:matrix_python_pylint_pass
capability LINT
stages 1
stage pylint classes=python sources=quality/testdata/real_clean.py
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_python_pylint_fail",
        "srcs": [":real_dirty.py"],
        "capability": "lint",
        "stages": ["pylint;python;quality/testdata/real_dirty.py"],
        "tool_names": ["pylint"],
        "tool_binaries": ["//quality/tools/python:pylint"],
        "expected": """producer //quality/testdata:matrix_python_pylint_fail
capability LINT
stages 1
stage pylint classes=python sources=quality/testdata/real_dirty.py
completed_rounds 1
convergence STABLE
initial 1
initial WARNING pylint W0611 quality/testdata/real_dirty.py 42 51 fixable=false "Unused import os"
terminal 1
terminal WARNING pylint W0611 quality/testdata/real_dirty.py 42 51 fixable=false "Unused import os"
replacements 0
""",
    },
]

# JavaScript cells: fixture-policy defaults (biome lint/format) over the
# real fixtures, the biome_cfg hinted closure (mirrors
# fixture_real_javascript_hinted), and the eslint lint opt-in.
