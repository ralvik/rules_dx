"""Matrix Markdown cases (split from `runner_matrix_cases.bzl`).

Contract: `docs/quality/runner-matrix.md`.
"""

MARKDOWN_CASES = [
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
        "toolfile_srcs": [":vale_test.ini", "styles/Dx/Markers.yml"],
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
        "toolfile_srcs": [":vale_test.ini", "styles/Dx/Markers.yml"],
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
        "toolfile_srcs": [":vale_test.ini", "styles/Dx/Markers.yml"],
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

