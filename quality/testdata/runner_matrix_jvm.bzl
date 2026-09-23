"""Matrix JVM cases (split from `runner_matrix_cases.bzl`).

Contract: `docs/quality/runner-matrix.md`.
"""

JVM_CASES = [
    {
        "name": "matrix_java_format_pass",
        "srcs": [":real_clean.java"],
        "capability": "format",
        "stages": ["google_java_format;java;quality/testdata/real_clean.java"],
        "tool_names": ["google_java_format"],
        "tool_binaries": ["//quality/tools/jvm:google_java_format"],
        "expected": """producer //quality/testdata:matrix_java_format_pass
capability FORMAT
stages 1
stage google_java_format classes=java sources=quality/testdata/real_clean.java
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_java_format_fail",
        "srcs": [":real_dirty.java"],
        "capability": "format",
        "stages": ["google_java_format;java;quality/testdata/real_dirty.java"],
        "tool_names": ["google_java_format"],
        "tool_binaries": ["//quality/tools/jvm:google_java_format"],
        "expected": """producer //quality/testdata:matrix_java_format_fail
capability FORMAT
stages 1
stage google_java_format classes=java sources=quality/testdata/real_dirty.java
completed_rounds 3
convergence STABLE
initial 1
initial WARNING google_java_format - quality/testdata/real_dirty.java 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.java 15 128 "\\npublic class Dirty {\\n  public static String hello(String name) {\\n    return \\"hello \\" + name;\\n  "
""",
    },
    {
        "name": "matrix_java_checkstyle_pass",
        "srcs": [":real_clean.java"],
        "capability": "lint",
        "stages": ["checkstyle;java;quality/testdata/real_clean.java"],
        "tool_names": ["checkstyle"],
        "tool_binaries": ["//quality/tools/jvm:checkstyle"],
        "config_tools": ["checkstyle"],
        "config_files": [":checkstyle_cfg/checkstyle.xml"],
        "toolfile_tools": ["checkstyle"],
        "toolfile_srcs": [":checkstyle_cfg/checkstyle.xml"],
        "expected": """producer //quality/testdata:matrix_java_checkstyle_pass
capability LINT
stages 1
stage checkstyle classes=java sources=quality/testdata/real_clean.java
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_java_checkstyle_fail",
        "srcs": [":real_dirty.java"],
        "capability": "lint",
        "stages": ["checkstyle;java;quality/testdata/real_dirty.java"],
        "tool_names": ["checkstyle"],
        "tool_binaries": ["//quality/tools/jvm:checkstyle"],
        "config_tools": ["checkstyle"],
        "config_files": [":checkstyle_cfg/checkstyle.xml"],
        "toolfile_tools": ["checkstyle"],
        "toolfile_srcs": [":checkstyle_cfg/checkstyle.xml"],
        "expected": """producer //quality/testdata:matrix_java_checkstyle_fail
capability LINT
stages 1
stage checkstyle classes=java sources=quality/testdata/real_dirty.java
completed_rounds 1
convergence STABLE
initial 1
initial ERROR checkstyle com.puppycrawl.tools.checkstyle.checks.imports.UnusedImportsCheck quality/testdata/real_dirty.java 22 22 fixable=false "Unused import - java.util.ArrayList."
terminal 1
terminal ERROR checkstyle com.puppycrawl.tools.checkstyle.checks.imports.UnusedImportsCheck quality/testdata/real_dirty.java 22 22 fixable=false "Unused import - java.util.ArrayList."
replacements 0
""",
    },
    {
        "name": "matrix_java_pmd_pass",
        "srcs": [":real_clean.java"],
        "capability": "lint",
        "stages": ["pmd;java;quality/testdata/real_clean.java"],
        "tool_names": ["pmd"],
        "tool_binaries": ["//quality/tools/jvm:pmd"],
        "expected": """producer //quality/testdata:matrix_java_pmd_pass
capability LINT
stages 1
stage pmd classes=java sources=quality/testdata/real_clean.java
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_java_pmd_fail",
        "srcs": [":real_dirty.java"],
        "capability": "lint",
        "stages": ["pmd;java;quality/testdata/real_dirty.java"],
        "tool_names": ["pmd"],
        "tool_binaries": ["//quality/tools/jvm:pmd"],
        "expected": """producer //quality/testdata:matrix_java_pmd_fail
capability LINT
stages 1
stage pmd classes=java sources=quality/testdata/real_dirty.java
completed_rounds 1
convergence STABLE
initial 2
initial INFO pmd UnnecessaryImport quality/testdata/real_dirty.java 15 42 fixable=false "Unused import 'java.util.ArrayList'"
initial WARNING pmd InstantiableUtilityClass quality/testdata/real_dirty.java 56 61 fixable=false "All members are static. Consider adding a private no-args constructor to prevent instantiation."
terminal 2
terminal INFO pmd UnnecessaryImport quality/testdata/real_dirty.java 15 42 fixable=false "Unused import 'java.util.ArrayList'"
terminal WARNING pmd InstantiableUtilityClass quality/testdata/real_dirty.java 56 61 fixable=false "All members are static. Consider adding a private no-args constructor to prevent instantiation."
replacements 0
""",
    },
    {
        "name": "matrix_kotlin_format_pass",
        "srcs": [":real_clean.kt"],
        "capability": "format",
        "stages": ["ktfmt;kotlin;quality/testdata/real_clean.kt"],
        "tool_names": ["ktfmt"],
        "tool_binaries": ["//quality/tools/jvm:ktfmt"],
        "expected": """producer //quality/testdata:matrix_kotlin_format_pass
capability FORMAT
stages 1
stage ktfmt classes=kotlin sources=quality/testdata/real_clean.kt
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_kotlin_format_fail",
        "srcs": [":real_dirty.kt"],
        "capability": "format",
        "stages": ["ktfmt;kotlin;quality/testdata/real_dirty.kt"],
        "tool_names": ["ktfmt"],
        "tool_binaries": ["//quality/tools/jvm:ktfmt"],
        "expected": """producer //quality/testdata:matrix_kotlin_format_fail
capability FORMAT
stages 1
stage ktfmt classes=kotlin sources=quality/testdata/real_dirty.kt
completed_rounds 2
convergence STABLE
initial 1
initial WARNING ktfmt - quality/testdata/real_dirty.kt 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement quality/testdata/real_dirty.kt 14 111 "\\nobject Dirty {\\n  fun hello(name: String): String {\\n    return \\"hello \\" + name\\n  "
""",
    },
    {
        "name": "matrix_kotlin_lint_pass",
        "generated": {
            "matrix/Hello.kt": "package hello\n\nobject Hello {\n    fun hello(name: String): String = \"hello \" + name\n}\n",
        },
        "capability": "lint",
        "stages": ["ktlint;kotlin;matrix/Hello.kt"],
        "tool_names": ["ktlint"],
        "tool_binaries": ["//quality/tools/jvm:ktlint"],
        "expected": """producer //quality/testdata:matrix_kotlin_lint_pass
capability LINT
stages 1
stage ktlint classes=kotlin sources=matrix/Hello.kt
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_kotlin_lint_fail",
        "srcs": [":real_dirty.kt"],
        "capability": "lint",
        "stages": ["ktlint;kotlin;quality/testdata/real_dirty.kt"],
        "tool_names": ["ktlint"],
        "tool_binaries": ["//quality/tools/jvm:ktlint"],
        "expected": """producer //quality/testdata:matrix_kotlin_lint_fail
capability LINT
stages 1
stage ktlint classes=kotlin sources=quality/testdata/real_dirty.kt
completed_rounds 2
convergence STABLE
initial 7
initial ERROR ktlint standard:filename quality/testdata/real_dirty.kt 0 0 fixable=false "File 'real_dirty.kt' contains a single top level declaration and should be named 'Dirty.kt'"
initial ERROR ktlint standard:blank-line-before-declaration quality/testdata/real_dirty.kt 41 41 fixable=true "Expected a blank line for this declaration"
initial ERROR ktlint standard:indent quality/testdata/real_dirty.kt 56 56 fixable=true "Unexpected indentation (0) (should be 4)"
initial ERROR ktlint standard:function-expression-body quality/testdata/real_dirty.kt 88 88 fixable=true "Function body should be replaced with body expression"
initial ERROR ktlint standard:indent quality/testdata/real_dirty.kt 90 90 fixable=true "Unexpected indentation (0) (should be 8)"
initial ERROR ktlint standard:op-spacing quality/testdata/real_dirty.kt 105 105 fixable=true "Missing spacing around \\"+\\""
initial ERROR ktlint standard:indent quality/testdata/real_dirty.kt 111 111 fixable=true "Unexpected indentation (0) (should be 4)"
terminal 1
terminal ERROR ktlint standard:filename quality/testdata/real_dirty.kt 0 0 fixable=false "File 'real_dirty.kt' contains a single top level declaration and should be named 'Dirty.kt'"
replacements 1
replacement quality/testdata/real_dirty.kt 41 112 "\\nobject Dirty {\\n    fun hello(name: String): String = \\"hello \\" + name"
""",
    },
]
