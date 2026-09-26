"""Layer-2 matrix cases (snapshot workflow): every supported language x capability cell.

"""

CLIPPY_LINT = """{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"using `is_empty` is clearer and more explicit","code":null,"level":"help","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":"\\"x\\".is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}
{"$message_type":"diagnostic","message":"1 warning emitted","code":null,"level":"warning","spans":[],"children":[],"rendered":null}"""

RUSTC_TYPE_ERROR = """{"$message_type":"diagnostic","message":"mismatched types","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"matrix/rustc_type.rs","byte_start":27,"byte_end":32,"line_start":2,"line_end":2,"column_start":9,"column_end":14,"is_primary":true,"text":[],"label":"expected `i32`, found `&str`","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}
{"$message_type":"diagnostic","message":"aborting due to 1 previous error","code":null,"level":"error","spans":[],"children":[],"rendered":null}"""

RUST_CASES = [
    {
        "name": "matrix_rust_format_pass",
        "srcs": [":clean.rs"],
        "capability": "format",
        "stages": ["rustfmt;rust;quality/testdata/clean.rs"],
        "rustfmt_from_toolchain": True,
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
replacement matrix/rustfmt_dirty.rs 13 37 " i32, b: i32) -> i32 {\\n    a + "
""",
    },
    {
        "name": "matrix_rust_format_edition_2015",
        "generated": {
            "matrix/rustfmt_edition_2015.rs": "pub fn edition_gate() -> i32 {\n    let async = 1;\n    async\n}\n",
        },
        "capability": "format",
        "stages": ["rustfmt;rust;matrix/rustfmt_edition_2015.rs"],
        "rustfmt_from_toolchain": True,
        "edition_tools": ["rustfmt"],
        "edition_values": ["2015"],
        "expected": """producer //quality/testdata:matrix_rust_format_edition_2015
capability FORMAT
stages 1
stage rustfmt classes=rust sources=matrix/rustfmt_edition_2015.rs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_rust_format_edition_mismatch",
        "generated": {
            "matrix/rustfmt_edition_mismatch.rs": "pub fn edition_gate() -> i32 {\n    let async = 1;\n    async\n}\n",
        },
        "capability": "format",
        "stages": ["rustfmt;rust;matrix/rustfmt_edition_mismatch.rs"],
        "rustfmt_from_toolchain": True,
        "edition_tools": ["rustfmt"],
        "edition_values": ["2021"],
        "expected": """producer //quality/testdata:matrix_rust_format_edition_mismatch
capability FORMAT
stages 1
stage rustfmt classes=rust sources=matrix/rustfmt_edition_mismatch.rs
completed_rounds 1
convergence STABLE
initial 2
initial ERROR rustfmt - matrix/rustfmt_edition_mismatch.rs 39 39 fixable=false "error: expected identifier, found keyword `async`"
initial ERROR rustfmt - matrix/rustfmt_edition_mismatch.rs 60 60 fixable=false "error: expected one of `move`, `use`, `{`, `|`, or `||`, found `}`"
terminal 2
terminal ERROR rustfmt - matrix/rustfmt_edition_mismatch.rs 39 39 fixable=false "error: expected identifier, found keyword `async`"
terminal ERROR rustfmt - matrix/rustfmt_edition_mismatch.rs 60 60 fixable=false "error: expected one of `move`, `use`, `{`, `|`, or `||`, found `}`"
replacements 0
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
        "upstream_generated": {"clippy": CLIPPY_LINT},
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
        "upstream_generated": {"rustc": RUSTC_TYPE_ERROR},
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

