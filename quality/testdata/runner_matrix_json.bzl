"""Matrix JSON cases (split from `runner_matrix_cases.bzl`).

"""

JSON_CASES = [
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
replacement quality/testdata/real_dirty.json 1 6 " \\"a\\": 1 "
""",
    },
]

