"""Layer-2 matrix cases (#59, snapshot workflow issue #322): every supported language x capability cell.

Contract: `docs/quality/runner-matrix.md`.
"""

load(":runner_matrix_tests.bzl", "runner_matrix_suite")

# Recorded upstream diagnostics for the delegated cells, byte-identical to
# the parser unit samples (`quality/adapter/src/parsers/rust.rs`: CLIPPY_LINT,
# RUSTC_TYPE_ERROR). Injected verbatim; do not hand-edit.
_CLIPPY_LINT = """{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"using `is_empty` is clearer and more explicit","code":null,"level":"help","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":"\\"x\\".is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}
{"$message_type":"diagnostic","message":"1 warning emitted","code":null,"level":"warning","spans":[],"children":[],"rendered":null}"""

_RUSTC_TYPE_ERROR = """{"$message_type":"diagnostic","message":"mismatched types","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"matrix/rustc_type.rs","byte_start":27,"byte_end":32,"line_start":2,"line_end":2,"column_start":9,"column_end":14,"is_primary":true,"text":[],"label":"expected `i32`, found `&str`","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}
{"$message_type":"diagnostic","message":"aborting due to 1 previous error","code":null,"level":"error","spans":[],"children":[],"rendered":null}"""

# Rust cells: rustfmt format over real bytes from the pinned toolchain,
# Clippy lint and rustc typecheck delegated over recorded upstream
# diagnostics (the upstream aspect/rule owns the invocation; dx never
# spawns them, per #47/#48 and the real_aspects.bzl contract).
_RUST_CASES = [
    {
        "name": "matrix_rust_format_pass",
        "srcs": [":clean.rs"],
        "capability": "format",
        "stages": ["rustfmt;rust;quality/testdata/clean.rs"],
        "rustfmt_from_toolchain": True,
        # Provider-less matrix input: no CrateInfo, so the edition the
        # aspect would fall back to (RUST_EDITION) is declared explicitly.
        "edition_tools": ["rustfmt"],
        "edition_values": ["2021"],
        "expected": """producer //quality/testdata:matrix_rust_format_pass
capability FORMAT
stages 1
stage rustfmt classes=rust sources=quality/testdata/clean.rs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_rust_format_fail",
        "generated": {
            "matrix/rustfmt_dirty.rs": "pub fn add(a:i32,b:i32)->i32 {\n    a+b\n}\n",
        },
        "capability": "format",
        "stages": ["rustfmt;rust;matrix/rustfmt_dirty.rs"],
        "rustfmt_from_toolchain": True,
        # Provider-less matrix input: no CrateInfo, so the edition the
        # aspect would fall back to (RUST_EDITION) is declared explicitly.
        "edition_tools": ["rustfmt"],
        "edition_values": ["2021"],
        "expected": """producer //quality/testdata:matrix_rust_format_fail
capability FORMAT
stages 1
stage rustfmt classes=rust sources=matrix/rustfmt_dirty.rs
completed_rounds 2
convergence STABLE
initial 1
initial WARNING rustfmt - matrix/rustfmt_dirty.rs 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/rustfmt_dirty.rs 0 41 "pub fn add(a: i32, b: i32) -> i32 {\\n    a + b\\n}\\n"
""",
    },
    {
        "name": "matrix_rust_lint_pass",
        "generated": {
            "matrix/clippy_clean.rs": "fn main() {}\n",
        },
        "capability": "lint",
        "stages": ["clippy;rust;matrix/clippy_clean.rs"],
        "upstream_tools": ["clippy"],
        "upstream_generated": {"clippy": ""},
        "expected": """producer //quality/testdata:matrix_rust_lint_pass
capability LINT
stages 1
stage clippy classes=rust sources=matrix/clippy_clean.rs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_rust_lint_fail",
        "generated": {
            "matrix/clippy_len.rs": "fn f(x:u8){\n       \"x\".len() == 0\n}\n",
        },
        "capability": "lint",
        "stages": ["clippy;rust;matrix/clippy_len.rs"],
        "upstream_tools": ["clippy"],
        "upstream_generated": {"clippy": _CLIPPY_LINT},
        "expected": """producer //quality/testdata:matrix_rust_lint_fail
capability LINT
stages 1
stage clippy classes=rust sources=matrix/clippy_len.rs
completed_rounds 1
convergence STABLE
initial 1
initial WARNING clippy clippy::len_zero matrix/clippy_len.rs 19 33 fixable=false "length comparison to zero"
terminal 1
terminal WARNING clippy clippy::len_zero matrix/clippy_len.rs 19 33 fixable=false "length comparison to zero"
replacements 0
""",
    },
    {
        "name": "matrix_rust_typecheck_pass",
        "generated": {
            "matrix/rustc_type_clean.rs": "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
        },
        "capability": "typecheck",
        "stages": ["rustc;rust;matrix/rustc_type_clean.rs"],
        "upstream_tools": ["rustc"],
        "upstream_generated": {"rustc": ""},
        "expected": """producer //quality/testdata:matrix_rust_typecheck_pass
capability TYPECHECK
stages 1
stage rustc classes=rust sources=matrix/rustc_type_clean.rs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_rust_typecheck_fail",
        "generated": {
            "matrix/rustc_type.rs": "pub fn add(a: i32, b: i32) -> i32 {\n    a + \"two\"\n}\n",
        },
        "capability": "typecheck",
        "stages": ["rustc;rust;matrix/rustc_type.rs"],
        "upstream_tools": ["rustc"],
        "upstream_generated": {"rustc": _RUSTC_TYPE_ERROR},
        "expected": """producer //quality/testdata:matrix_rust_typecheck_fail
capability TYPECHECK
stages 1
stage rustc classes=rust sources=matrix/rustc_type.rs
completed_rounds 1
convergence STABLE
initial 1
initial ERROR rustc E0308 matrix/rustc_type.rs 44 49 fixable=false "mismatched types"
terminal 1
terminal ERROR rustc E0308 matrix/rustc_type.rs 44 49 fixable=false "mismatched types"
replacements 0
""",
    },
]

# Python cells: fixture-policy stages (pydoclint+ruff lint, ruff format,
# ty typecheck) over the real fixtures, stub-class variants over generated
# `.pyi` bytes, the ruff_cfg hinted closure (mirrors
# fixture_real_python_hinted), and the flake8/pylint lint opt-ins.
_PYTHON_CASES = [
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
initial ERROR ruff F401 quality/testdata/real_dirty.py 53 55 fixable=true "`os` imported but unused"
initial ERROR pydoclint DOC103 quality/testdata/real_dirty.py 58 58 fixable=false "Function `add`: Docstring arguments are different from function arguments. (Or could be other formatting issues: https://jsh9.github.io/pydoclint/violation_codes.html#notes-on-doc103 ). Arguments in the function signature but not in the docstring: [second: int]."
initial ERROR pydoclint DOC101 quality/testdata/real_dirty.py 58 58 fixable=false "Function `add`: Docstring contains fewer arguments than in function signature."
terminal 2
terminal ERROR pydoclint DOC103 quality/testdata/real_dirty.py 48 48 fixable=false "Function `add`: Docstring arguments are different from function arguments. (Or could be other formatting issues: https://jsh9.github.io/pydoclint/violation_codes.html#notes-on-doc103 ). Arguments in the function signature but not in the docstring: [second: int]."
terminal ERROR pydoclint DOC101 quality/testdata/real_dirty.py 48 48 fixable=false "Function `add`: Docstring contains fewer arguments than in function signature."
replacements 1
replacement quality/testdata/real_dirty.py 0 304 "\\"\\"\\"Real-pipeline dirty fixture (WP2).\\"\\"\\"\\n\\n\\n\\ndef add(first: int, second: int) -> int:\\n    \\"\\"\\"Add two integers.\\n\\n    Parameters\\n    ----------\\n    first : int\\n        First operand.\\n\\n    Returns\\n    -------\\n    int\\n        The sum.\\n    \\"\\"\\"\\n    return first + second\\n\\n\\nTOTAL:int=add(1, \\"two\\")\\n"
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
initial ERROR ruff unformatted quality/testdata/real_dirty.py 286 290 fixable=true "File would be reformatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.py 0 304 "\\"\\"\\"Real-pipeline dirty fixture (WP2).\\"\\"\\"\\n\\nimport os\\n\\n\\ndef add(first: int, second: int) -> int:\\n    \\"\\"\\"Add two integers.\\n\\n    Parameters\\n    ----------\\n    first : int\\n        First operand.\\n\\n    Returns\\n    -------\\n    int\\n        The sum.\\n    \\"\\"\\"\\n    return first + second\\n\\n\\nTOTAL: int = add(1, \\"two\\")\\n"
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
initial ERROR ty invalid-argument-type quality/testdata/real_dirty.py 297 297 fixable=false "Argument to function `add` is incorrect: Expected `int`, found `Literal[\\"two\\"]`"
terminal 1
terminal ERROR ty invalid-argument-type quality/testdata/real_dirty.py 297 297 fixable=false "Argument to function `add` is incorrect: Expected `int`, found `Literal[\\"two\\"]`"
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
initial ERROR flake8 F401 quality/testdata/real_dirty.py 46 46 fixable=false "'os' imported but unused"
initial ERROR flake8 E231 quality/testdata/real_dirty.py 285 285 fixable=false "missing whitespace after ':'"
initial ERROR flake8 E225 quality/testdata/real_dirty.py 289 289 fixable=false "missing whitespace around operator"
terminal 3
terminal ERROR flake8 F401 quality/testdata/real_dirty.py 46 46 fixable=false "'os' imported but unused"
terminal ERROR flake8 E231 quality/testdata/real_dirty.py 285 285 fixable=false "missing whitespace after ':'"
terminal ERROR flake8 E225 quality/testdata/real_dirty.py 289 289 fixable=false "missing whitespace around operator"
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
initial WARNING pylint W0611 quality/testdata/real_dirty.py 46 55 fixable=false "Unused import os"
terminal 1
terminal WARNING pylint W0611 quality/testdata/real_dirty.py 46 55 fixable=false "Unused import os"
replacements 0
""",
    },
]

# JavaScript cells: fixture-policy defaults (biome lint/format) over the
# real fixtures, the biome_cfg hinted closure (mirrors
# fixture_real_javascript_hinted), and the eslint lint opt-in.
_JS_CASES = [
    {
        "name": "matrix_javascript_lint_pass",
        "srcs": [":real_clean.js"],
        "capability": "lint",
        "stages": ["biome;javascript;quality/testdata/real_clean.js"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_javascript_lint_pass
capability LINT
stages 1
stage biome classes=javascript sources=quality/testdata/real_clean.js
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_javascript_lint_fail",
        "srcs": [":real_dirty.js"],
        "capability": "lint",
        "stages": ["biome;javascript;quality/testdata/real_dirty.js"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_javascript_lint_fail
capability LINT
stages 1
stage biome classes=javascript sources=quality/testdata/real_dirty.js
completed_rounds 1
convergence STABLE
initial 1
initial WARNING biome lint/correctness/noUnusedVariables quality/testdata/real_dirty.js 43 49 fixable=false "This variable unused is unused."
terminal 1
terminal WARNING biome lint/correctness/noUnusedVariables quality/testdata/real_dirty.js 43 49 fixable=false "This variable unused is unused."
replacements 0
""",
    },
    {
        "name": "matrix_javascript_format_pass",
        "srcs": [":real_clean.js"],
        "capability": "format",
        "stages": ["biome;javascript;quality/testdata/real_clean.js"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_javascript_format_pass
capability FORMAT
stages 1
stage biome classes=javascript sources=quality/testdata/real_clean.js
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_javascript_format_fail",
        "srcs": [":real_dirty.js"],
        "capability": "format",
        "stages": ["biome;javascript;quality/testdata/real_dirty.js"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_javascript_format_fail
capability FORMAT
stages 1
stage biome classes=javascript sources=quality/testdata/real_dirty.js
completed_rounds 2
convergence STABLE
initial 1
initial WARNING biome - quality/testdata/real_dirty.js 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.js 0 77 "export function add(first, second) {\\n\\tconst unused = 1;\\n\\treturn first + second;\\n}\\n"
""",
    },
    {
        "name": "matrix_javascript_biome_hinted",
        "srcs": [":real_clean.js"],
        "capability": "lint",
        "stages": ["biome;javascript;quality/testdata/real_clean.js"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "config_tools": ["biome"],
        "config_files": [":biome_cfg/biome.json"],
        "toolfile_tools": ["biome"],
        "toolfile_srcs": [":biome_cfg/biome.json"],
        "expected": """producer //quality/testdata:matrix_javascript_biome_hinted
capability LINT
stages 1
stage biome classes=javascript sources=quality/testdata/real_clean.js
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_javascript_eslint_pass",
        "srcs": [":real_clean.js"],
        "capability": "lint",
        "stages": ["eslint;javascript;quality/testdata/real_clean.js"],
        "tool_names": ["eslint"],
        "tool_binaries": ["//quality/tools/javascript/bin:eslint"],
        "config_tools": ["eslint"],
        "config_files": [":eslint_cfg/eslint.config.js"],
        "toolfile_tools": ["eslint"],
        "toolfile_srcs": [":eslint_cfg/eslint.config.js"],
        "expected": """producer //quality/testdata:matrix_javascript_eslint_pass
capability LINT
stages 1
stage eslint classes=javascript sources=quality/testdata/real_clean.js
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_javascript_eslint_fail",
        "srcs": [":real_dirty.js"],
        "capability": "lint",
        "stages": ["eslint;javascript;quality/testdata/real_dirty.js"],
        "tool_names": ["eslint"],
        "tool_binaries": ["//quality/tools/javascript/bin:eslint"],
        "config_tools": ["eslint"],
        "config_files": [":eslint_cfg/eslint.config.js"],
        "toolfile_tools": ["eslint"],
        "toolfile_srcs": [":eslint_cfg/eslint.config.js"],
        "expected": """producer //quality/testdata:matrix_javascript_eslint_fail
capability LINT
stages 1
stage eslint classes=javascript sources=quality/testdata/real_dirty.js
completed_rounds 1
convergence STABLE
initial 1
initial ERROR eslint no-unused-vars quality/testdata/real_dirty.js 43 49 fixable=false "'unused' is assigned a value but never used."
terminal 1
terminal ERROR eslint no-unused-vars quality/testdata/real_dirty.js 43 49 fixable=false "'unused' is assigned a value but never used."
replacements 0
""",
    },
]

# TypeScript/JSX/TSX cells: fixture-policy Biome defaults. TypeScript
# reuses the real clean/dirty pair; JSX/TSX reuse the real clean files
# with generated dirty bytes (no aspect-visible dirty subject exists).
_TS_CASES = [
    {
        "name": "matrix_typescript_lint_pass",
        "srcs": [":real_clean.ts"],
        "capability": "lint",
        "stages": ["biome;typescript;quality/testdata/real_clean.ts"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_typescript_lint_pass
capability LINT
stages 1
stage biome classes=typescript sources=quality/testdata/real_clean.ts
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_typescript_lint_fail",
        "srcs": [":real_dirty.ts"],
        "capability": "lint",
        "stages": ["biome;typescript;quality/testdata/real_dirty.ts"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_typescript_lint_fail
capability LINT
stages 1
stage biome classes=typescript sources=quality/testdata/real_dirty.ts
completed_rounds 1
convergence STABLE
initial 1
initial WARNING biome lint/correctness/noUnusedVariables quality/testdata/real_dirty.ts 57 63 fixable=false "This variable unused is unused."
terminal 1
terminal WARNING biome lint/correctness/noUnusedVariables quality/testdata/real_dirty.ts 57 63 fixable=false "This variable unused is unused."
replacements 0
""",
    },
    {
        "name": "matrix_typescript_format_pass",
        "srcs": [":real_clean.ts"],
        "capability": "format",
        "stages": ["biome;typescript;quality/testdata/real_clean.ts"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_typescript_format_pass
capability FORMAT
stages 1
stage biome classes=typescript sources=quality/testdata/real_clean.ts
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_typescript_format_fail",
        "srcs": [":real_dirty.ts"],
        "capability": "format",
        "stages": ["biome;typescript;quality/testdata/real_dirty.ts"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_typescript_format_fail
capability FORMAT
stages 1
stage biome classes=typescript sources=quality/testdata/real_dirty.ts
completed_rounds 2
convergence STABLE
initial 1
initial WARNING biome - quality/testdata/real_dirty.ts 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.ts 0 91 "export function add(first: number, second: number) {\\n\\tconst unused = 1;\\n\\treturn first + second;\\n}\\n"
""",
    },
    {
        "name": "matrix_jsx_lint_pass",
        "srcs": [":real_clean.jsx"],
        "capability": "lint",
        "stages": ["biome;jsx;quality/testdata/real_clean.jsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_jsx_lint_pass
capability LINT
stages 1
stage biome classes=jsx sources=quality/testdata/real_clean.jsx
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_jsx_lint_fail",
        "generated": {
            "matrix/jsx_dirty.jsx": "export function View( ){ const unused = 1; return <div>Hello</div> }\n",
        },
        "capability": "lint",
        "stages": ["biome;jsx;matrix/jsx_dirty.jsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_jsx_lint_fail
capability LINT
stages 1
stage biome classes=jsx sources=matrix/jsx_dirty.jsx
completed_rounds 1
convergence STABLE
initial 1
initial WARNING biome lint/correctness/noUnusedVariables matrix/jsx_dirty.jsx 31 37 fixable=false "This variable unused is unused."
terminal 1
terminal WARNING biome lint/correctness/noUnusedVariables matrix/jsx_dirty.jsx 31 37 fixable=false "This variable unused is unused."
replacements 0
""",
    },
    {
        "name": "matrix_jsx_format_pass",
        "srcs": [":real_clean.jsx"],
        "capability": "format",
        "stages": ["biome;jsx;quality/testdata/real_clean.jsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_jsx_format_pass
capability FORMAT
stages 1
stage biome classes=jsx sources=quality/testdata/real_clean.jsx
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_jsx_format_fail",
        "generated": {
            "matrix/jsx_dirty.jsx": "export function View( ){ const unused = 1; return <div>Hello</div> }\n",
        },
        "capability": "format",
        "stages": ["biome;jsx;matrix/jsx_dirty.jsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_jsx_format_fail
capability FORMAT
stages 1
stage biome classes=jsx sources=matrix/jsx_dirty.jsx
completed_rounds 2
convergence STABLE
initial 1
initial WARNING biome - matrix/jsx_dirty.jsx 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/jsx_dirty.jsx 0 69 "export function View() {\\n\\tconst unused = 1;\\n\\treturn <div>Hello</div>;\\n}\\n"
""",
    },
    {
        "name": "matrix_tsx_lint_pass",
        "srcs": [":real_clean.tsx"],
        "capability": "lint",
        "stages": ["biome;tsx;quality/testdata/real_clean.tsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_tsx_lint_pass
capability LINT
stages 1
stage biome classes=tsx sources=quality/testdata/real_clean.tsx
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_tsx_lint_fail",
        "generated": {
            "matrix/tsx_dirty.tsx": "export function View(props:{name:string}){ const unused = 1; return <div>Hello {props.name}</div> }\n",
        },
        "capability": "lint",
        "stages": ["biome;tsx;matrix/tsx_dirty.tsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_tsx_lint_fail
capability LINT
stages 1
stage biome classes=tsx sources=matrix/tsx_dirty.tsx
completed_rounds 1
convergence STABLE
initial 1
initial WARNING biome lint/correctness/noUnusedVariables matrix/tsx_dirty.tsx 49 55 fixable=false "This variable unused is unused."
terminal 1
terminal WARNING biome lint/correctness/noUnusedVariables matrix/tsx_dirty.tsx 49 55 fixable=false "This variable unused is unused."
replacements 0
""",
    },
    {
        "name": "matrix_tsx_format_pass",
        "srcs": [":real_clean.tsx"],
        "capability": "format",
        "stages": ["biome;tsx;quality/testdata/real_clean.tsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_tsx_format_pass
capability FORMAT
stages 1
stage biome classes=tsx sources=quality/testdata/real_clean.tsx
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_tsx_format_fail",
        "generated": {
            "matrix/tsx_dirty.tsx": "export function View(props:{name:string}){ const unused = 1; return <div>Hello {props.name}</div> }\n",
        },
        "capability": "format",
        "stages": ["biome;tsx;matrix/tsx_dirty.tsx"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_tsx_format_fail
capability FORMAT
stages 1
stage biome classes=tsx sources=matrix/tsx_dirty.tsx
completed_rounds 2
convergence STABLE
initial 1
initial WARNING biome - matrix/tsx_dirty.tsx 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/tsx_dirty.tsx 0 100 "export function View(props: { name: string }) {\\n\\tconst unused = 1;\\n\\treturn <div>Hello {props.name}</div>;\\n}\\n"
""",
    },
]

# JSON cells: fixture-policy defaults (biome lint, prettier format). The
# compact dirty document is lint-clean by design (one prettier format
# finding), so the lint-fail cell uses generated duplicate-key bytes.
_JSON_CASES = [
    {
        "name": "matrix_json_lint_pass",
        "srcs": [":real_clean.json"],
        "capability": "lint",
        "stages": ["biome;json;quality/testdata/real_clean.json"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_json_lint_pass
capability LINT
stages 1
stage biome classes=json sources=quality/testdata/real_clean.json
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_json_lint_fail",
        "generated": {
            "matrix/json_dup.json": "{\"a\": 1, \"a\": 2}\n",
        },
        "capability": "lint",
        "stages": ["biome;json;matrix/json_dup.json"],
        "tool_names": ["biome"],
        "tool_binaries": ["@dx_tools//:biome"],
        "expected": """producer //quality/testdata:matrix_json_lint_fail
capability LINT
stages 1
stage biome classes=json sources=matrix/json_dup.json
completed_rounds 1
convergence STABLE
initial 1
initial ERROR biome lint/suspicious/noDuplicateObjectKeys matrix/json_dup.json 1 4 fixable=false "The key a was already declared."
terminal 1
terminal ERROR biome lint/suspicious/noDuplicateObjectKeys matrix/json_dup.json 1 4 fixable=false "The key a was already declared."
replacements 0
""",
    },
    {
        "name": "matrix_json_format_pass",
        "srcs": [":real_clean.json"],
        "capability": "format",
        "stages": ["prettier;json;quality/testdata/real_clean.json"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/tools/javascript/bin:prettier"],
        "expected": """producer //quality/testdata:matrix_json_format_pass
capability FORMAT
stages 1
stage prettier classes=json sources=quality/testdata/real_clean.json
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_json_format_fail",
        "srcs": [":real_dirty.json"],
        "capability": "format",
        "stages": ["prettier;json;quality/testdata/real_dirty.json"],
        "tool_names": ["prettier"],
        "tool_binaries": ["//quality/tools/javascript/bin:prettier"],
        "expected": """producer //quality/testdata:matrix_json_format_fail
capability FORMAT
stages 1
stage prettier classes=json sources=quality/testdata/real_dirty.json
completed_rounds 2
convergence STABLE
initial 1
initial WARNING prettier - quality/testdata/real_dirty.json 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.json 0 8 "{ \\"a\\": 1 }\\n"
""",
    },
]

# Starlark/TOML cells: fixture-policy defaults (buildifier/taplo
# lint+format). No aspect-visible dirty subject exists, so fail cells use
# generated bytes.
_DATA_CASES = [
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
replacement matrix/starlark_dirty.bzl 0 4 "x = 1\\n"
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
replacement matrix/starlark_dirty.bzl 0 4 "x = 1\\n"
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
replacement matrix/toml_dirty.toml 0 4 "a = 1\\n"
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
replacement matrix/toml_dirty.toml 0 4 "a = 1\\n"
""",
    },
]

# Markdown cells: fixture-policy lint defaults (markdown_check link/structure
# plus vale prose) with the vale_test.ini closure, over the real clean file,
# generated broken-link bytes, and the sibling link-resolution pair.
_MARKDOWN_CASES = [
    {
        "name": "matrix_markdown_lint_pass",
        "srcs": [":real_clean.md"],
        "capability": "lint",
        "stages": [
            "markdown_check;markdown;quality/testdata/real_clean.md",
            "vale;markdown;quality/testdata/real_clean.md",
        ],
        "tool_names": ["markdown_check", "vale"],
        "tool_binaries": ["//quality/markdown:quality_markdown", "@dx_tools//:vale"],
        "config_tools": ["vale"],
        "config_files": [":vale_test.ini"],
        "toolfile_tools": ["vale", "vale"],
        "toolfile_srcs": [":vale_test.ini", "styles/.keep"],
        "expected": """producer //quality/testdata:matrix_markdown_lint_pass
capability LINT
stages 2
stage markdown_check classes=markdown sources=quality/testdata/real_clean.md
stage vale classes=markdown sources=quality/testdata/real_clean.md
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_markdown_lint_fail",
        "generated": {
            "matrix/markdown_dirty.md": "# Matrix dirty\n\nSee [missing](matrix/missing.md).\n",
        },
        "capability": "lint",
        "stages": [
            "markdown_check;markdown;matrix/markdown_dirty.md",
            "vale;markdown;matrix/markdown_dirty.md",
        ],
        "tool_names": ["markdown_check", "vale"],
        "tool_binaries": ["//quality/markdown:quality_markdown", "@dx_tools//:vale"],
        "config_tools": ["vale"],
        "config_files": [":vale_test.ini"],
        "toolfile_tools": ["vale", "vale"],
        "toolfile_srcs": [":vale_test.ini", "styles/.keep"],
        "expected": """producer //quality/testdata:matrix_markdown_lint_fail
capability LINT
stages 2
stage markdown_check classes=markdown sources=matrix/markdown_dirty.md
stage vale classes=markdown sources=matrix/markdown_dirty.md
completed_rounds 1
convergence STABLE
initial 1
initial ERROR markdown_check missing-file-target matrix/markdown_dirty.md 16 16 fixable=false "link target does not resolve to a declared sibling: matrix/matrix/missing.md"
terminal 1
terminal ERROR markdown_check missing-file-target matrix/markdown_dirty.md 16 16 fixable=false "link target does not resolve to a declared sibling: matrix/matrix/missing.md"
replacements 0
""",
    },
    {
        "name": "matrix_markdown_sibling_pass",
        "srcs": [":sibling_clean.md"],
        "siblings": [":sibling_license.txt"],
        "capability": "lint",
        "stages": [
            "markdown_check;markdown;quality/testdata/sibling_clean.md",
            "vale;markdown;quality/testdata/sibling_clean.md",
        ],
        "tool_names": ["markdown_check", "vale"],
        "tool_binaries": ["//quality/markdown:quality_markdown", "@dx_tools//:vale"],
        "config_tools": ["vale"],
        "config_files": [":vale_test.ini"],
        "toolfile_tools": ["vale", "vale"],
        "toolfile_srcs": [":vale_test.ini", "styles/.keep"],
        "expected": """producer //quality/testdata:matrix_markdown_sibling_pass
capability LINT
stages 2
stage markdown_check classes=markdown sources=quality/testdata/sibling_clean.md
stage vale classes=markdown sources=quality/testdata/sibling_clean.md
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
]

def runner_matrix_cases(name):
    runner_matrix_suite(name, _RUST_CASES + _PYTHON_CASES + _JS_CASES + _TS_CASES + _JSON_CASES + _DATA_CASES + _MARKDOWN_CASES)
