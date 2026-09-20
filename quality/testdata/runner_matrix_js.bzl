"""Split from `runner_matrix_cases.bzl`. No behavior change."""

JS_CASES = [
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
