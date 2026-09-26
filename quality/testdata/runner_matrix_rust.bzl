"""Layer-2 matrix cases (snapshot workflow): every supported language x capability cell."""

CLIPPY_LINT = """{"$message_type":"diagnostic","message":"length comparison to zero","code":{"code":"clippy::len_zero","explanation":null},"level":"warning","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[{"message":"using is_empty is clearer and more explicit","code":null,"level":"help","spans":[{"file_name":"matrix/clippy_len.rs","byte_start":19,"byte_end":33,"line_start":2,"line_end":2,"column_start":8,"column_end":22,"is_primary":true,"text":[],"label":null,"suggested_replacement":"\\"x\\".is_empty()","suggestion_applicability":"MachineApplicable","expansion":null}],"children":[],"rendered":null}],"rendered":null}"""

RUSTC_TYPE_ERROR = """{"$message_type":"diagnostic","message":"mismatched types","code":{"code":"E0308","explanation":null},"level":"error","spans":[{"file_name":"matrix/rustc_type.rs","byte_start":27,"byte_end":32,"line_start":2,"line_end":2,"column_start":9,"column_end":14,"is_primary":true,"text":[],"label":"expected i32, found &str","suggested_replacement":null,"suggestion_applicability":null,"expansion":null}],"children":[],"rendered":null}"""

RUST_CASES = [
    {
        "name": "matrix_rust_format_pass",
        "srcs": [":clean.rs"],
        "capability": "format",
        "stages": ["rustfmt;rust;quality/testdata/clean.rs"],
        "rustfmt_from_toolchain": True,
        "edition_tools": ["rustfmt"],
        "edition_values": ["2021"],
        "expected": """producer //quality/testdata:matrix_rust_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_rust_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_rust_format_edition_2015""",
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
        "expected": """producer //quality/testdata:matrix_rust_format_edition_mismatch""",
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
        "expected": """producer //quality/testdata:matrix_rust_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_rust_lint_fail""",
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
        "expected": """producer //quality/testdata:matrix_rust_typecheck_pass""",
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
        "expected": """producer //quality/testdata:matrix_rust_typecheck_fail""",
    },
]
