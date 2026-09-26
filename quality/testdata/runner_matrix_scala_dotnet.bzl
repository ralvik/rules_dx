"""Scala plus.NET cohort matrix cells.

Contract: `docs/quality/runner-matrix.md`.
Seed-only wiring proof with fake doubles plus recorded diagnostics.
"""

# Recorded Scalafix callback NDJSON for the lint-fail cell, shaped like
# the parser unit samples (`quality/adapter/src/parsers/scalafix.rs`):
# workspace-relative path, rule plus positions. Injected verbatim.
SCALAFIX_LINT = """{"path": "matrix/scalafix_dirty.scala", "line": 4, "column": 3, "rule": "DisableSyntax.var", "message": "mutable state should be avoided", "severity": "error"}"""

# Recorded Roslyn SARIF union for the lint-fail cell, shaped like the
# parser unit samples (`quality/adapter/src/parsers/roslyn.rs`): single
# run with one CA1822 result addressing the workspace path.
ROSLYN_LINT = """{"version": "2.1.0", "runs": [{"results": [{"ruleId": "CA1822", "level": "warning", "message": {"text": "Member 'Greet' does not access instance data"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "matrix/roslyn_dirty.cs"}, "region": {"startLine": 6, "startColumn": 28, "endLine": 6, "endColumn": 33}}}]}]}]}"""

# Recorded FSharpLint library NDJSON for the lint-fail cell, shaped like
# the parser unit samples (`quality/adapter/src/parsers/fsharplint.rs`).
FSHARPLINT_LINT = """{"path": "matrix/fsharplint_dirty.fs", "rule": "FL0036", "message": "Consider changing `ExampleInterface` to be prefixed with `I`.", "startLine": 4, "startColumn": 6, "endLine": 4, "endColumn": 22}"""

SCALA_DOTNET_CASES = [
    {
        "name": "matrix_scala_format_pass",
        "generated": {
            "matrix/scalafmt_clean.scala": "package fixtures.scalafmt\n\nobject Sample {\n  def greet = 1\n}\n",
        },
        "capability": "format",
        "stages": ["scalafmt;scala;matrix/scalafmt_clean.scala"],
        "tool_names": ["scalafmt"],
        "tool_binaries": ["//quality/testdata:fake_scalafmt"],
        "expected": """producer //quality/testdata:matrix_scala_format_pass
capability FORMAT
stages 1
stage scalafmt classes=scala sources=matrix/scalafmt_clean.scala
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_scala_format_fail",
        "generated": {
            "matrix/scalafmt_dirty.scala": "package fixtures.scalafmt\n\nobject Sample { BADFMT }\n",
        },
        "capability": "format",
        "stages": ["scalafmt;scala;matrix/scalafmt_dirty.scala"],
        "tool_names": ["scalafmt"],
        "tool_binaries": ["//quality/testdata:fake_scalafmt"],
        "expected": """producer //quality/testdata:matrix_scala_format_fail
capability FORMAT
stages 1
stage scalafmt classes=scala sources=matrix/scalafmt_dirty.scala
completed_rounds 2
convergence STABLE
initial 1
initial WARNING scalafmt - matrix/scalafmt_dirty.scala 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/scalafmt_dirty.scala 43 49 "fixed"
""",
    },
    {
        "name": "matrix_scala_lint_pass",
        "generated": {
            "matrix/scalafix_clean.scala": "package fixtures.scalafix\n\nobject Sample {\n  def greet = 1\n}\n",
        },
        "capability": "lint",
        "stages": ["scalafix;scala;matrix/scalafix_clean.scala"],
        "upstream_tools": ["scalafix"],
        "upstream_generated": {"scalafix": ""},
        "expected": """producer //quality/testdata:matrix_scala_lint_pass
capability LINT
stages 1
stage scalafix classes=scala sources=matrix/scalafix_clean.scala
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_scala_lint_fail",
        "generated": {
            "matrix/scalafix_dirty.scala": "package fixtures.scalafix\n\nobject Sample {\n  var x = 1\n  def greet = 1\n}\n",
        },
        "capability": "lint",
        "stages": ["scalafix;scala;matrix/scalafix_dirty.scala"],
        "upstream_tools": ["scalafix"],
        "upstream_generated": {"scalafix": SCALAFIX_LINT},
        "expected": """producer //quality/testdata:matrix_scala_lint_fail
capability LINT
stages 1
stage scalafix classes=scala sources=matrix/scalafix_dirty.scala
completed_rounds 1
convergence STABLE
initial 1
initial ERROR scalafix DisableSyntax.var matrix/scalafix_dirty.scala 45 45 fixable=false "mutable state should be avoided"
terminal 1
terminal ERROR scalafix DisableSyntax.var matrix/scalafix_dirty.scala 45 45 fixable=false "mutable state should be avoided"
replacements 0
""",
    },
    {
        "name": "matrix_csharp_format_pass",
        "generated": {
            "matrix/csharpier_clean.cs": "// Seed C# format fixture.\nnamespace Fixtures.CSharpier;\n\npublic static class Greeter\n{\n}\n",
        },
        "capability": "format",
        "stages": ["csharpier;csharp;matrix/csharpier_clean.cs"],
        "tool_names": ["csharpier"],
        "tool_binaries": ["//quality/testdata:fake_csharpier"],
        "expected": """producer //quality/testdata:matrix_csharp_format_pass
capability FORMAT
stages 1
stage csharpier classes=csharp sources=matrix/csharpier_clean.cs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_csharp_format_fail",
        "generated": {
            "matrix/csharpier_dirty.cs": "// Seed C# format fixture.\nnamespace Fixtures.CSharpier;\n\npublic static class Greeter { BADFMT }\n",
        },
        "capability": "format",
        "stages": ["csharpier;csharp;matrix/csharpier_dirty.cs"],
        "tool_names": ["csharpier"],
        "tool_binaries": ["//quality/testdata:fake_csharpier"],
        "expected": """producer //quality/testdata:matrix_csharp_format_fail
capability FORMAT
stages 1
stage csharpier classes=csharp sources=matrix/csharpier_dirty.cs
completed_rounds 2
convergence STABLE
initial 1
initial WARNING csharpier - matrix/csharpier_dirty.cs 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/csharpier_dirty.cs 88 94 "fixed"
""",
    },
    {
        "name": "matrix_csharp_lint_pass",
        "generated": {
            "matrix/roslyn_clean.cs": "// Seed C# lint fixture.\nnamespace Fixtures.Roslyn;\n\npublic static class Greeter\n{\n}\n",
        },
        "capability": "lint",
        "stages": ["roslyn;csharp;matrix/roslyn_clean.cs"],
        "upstream_tools": ["roslyn"],
        "upstream_generated": {"roslyn": """{"version": "2.1.0", "runs": []}"""},
        "expected": """producer //quality/testdata:matrix_csharp_lint_pass
capability LINT
stages 1
stage roslyn classes=csharp sources=matrix/roslyn_clean.cs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_csharp_lint_fail",
        "generated": {
            "matrix/roslyn_dirty.cs": "// Seed C# lint fixture.\nnamespace Fixtures.Roslyn;\n\npublic static class Greeter\n{\n    public static string Greet(string name)\n    {\n        return \"hello \";\n    }\n}\n",
        },
        "capability": "lint",
        "stages": ["roslyn;csharp;matrix/roslyn_dirty.cs"],
        "upstream_tools": ["roslyn"],
        "upstream_generated": {"roslyn": ROSLYN_LINT},
        "expected": """producer //quality/testdata:matrix_csharp_lint_fail
capability LINT
stages 1
stage roslyn classes=csharp sources=matrix/roslyn_dirty.cs
completed_rounds 1
convergence STABLE
initial 1
initial WARNING roslyn CA1822 matrix/roslyn_dirty.cs 110 115 fixable=false "Member 'Greet' does not access instance data"
terminal 1
terminal WARNING roslyn CA1822 matrix/roslyn_dirty.cs 110 115 fixable=false "Member 'Greet' does not access instance data"
replacements 0
""",
    },
    {
        "name": "matrix_fsharp_format_pass",
        "generated": {
            "matrix/fantomas_clean.fs": "// Seed F# format fixture.\nmodule Sample\n\nlet greet name = 1\n",
        },
        "capability": "format",
        "stages": ["fantomas;fsharp;matrix/fantomas_clean.fs"],
        "tool_names": ["fantomas"],
        "tool_binaries": ["//quality/testdata:fake_fantomas"],
        "expected": """producer //quality/testdata:matrix_fsharp_format_pass
capability FORMAT
stages 1
stage fantomas classes=fsharp sources=matrix/fantomas_clean.fs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_fsharp_format_fail",
        "generated": {
            "matrix/fantomas_dirty.fs": "// Seed F# format fixture.\nmodule Sample\n\nlet greet name = BADFMT\n",
        },
        "capability": "format",
        "stages": ["fantomas;fsharp;matrix/fantomas_dirty.fs"],
        "tool_names": ["fantomas"],
        "tool_binaries": ["//quality/testdata:fake_fantomas"],
        "expected": """producer //quality/testdata:matrix_fsharp_format_fail
capability FORMAT
stages 1
stage fantomas classes=fsharp sources=matrix/fantomas_dirty.fs
completed_rounds 2
convergence STABLE
initial 1
initial WARNING fantomas - matrix/fantomas_dirty.fs 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/fantomas_dirty.fs 59 65 "fixed"
""",
    },
    {
        "name": "matrix_fsharp_lint_pass",
        "generated": {
            "matrix/fsharplint_clean.fs": "// Seed F# lint fixture.\nmodule Sample\n\nlet greet name = 1\n",
        },
        "capability": "lint",
        "stages": ["fsharplint;fsharp;matrix/fsharplint_clean.fs"],
        "upstream_tools": ["fsharplint"],
        "upstream_generated": {"fsharplint": ""},
        "expected": """producer //quality/testdata:matrix_fsharp_lint_pass
capability LINT
stages 1
stage fsharplint classes=fsharp sources=matrix/fsharplint_clean.fs
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_fsharp_lint_fail",
        "generated": {
            "matrix/fsharplint_dirty.fs": "// Seed F# lint fixture.\nmodule Sample\n\ntype ExampleInterface =\n    abstract Member: int\n",
        },
        "capability": "lint",
        "stages": ["fsharplint;fsharp;matrix/fsharplint_dirty.fs"],
        "upstream_tools": ["fsharplint"],
        "upstream_generated": {"fsharplint": FSHARPLINT_LINT},
        "expected": """producer //quality/testdata:matrix_fsharp_lint_fail
capability LINT
stages 1
stage fsharplint classes=fsharp sources=matrix/fsharplint_dirty.fs
completed_rounds 1
convergence STABLE
initial 1
initial WARNING fsharplint FL0036 matrix/fsharplint_dirty.fs 45 61 fixable=false "Consider changing `ExampleInterface` to be prefixed with `I`."
terminal 1
terminal WARNING fsharplint FL0036 matrix/fsharplint_dirty.fs 45 61 fixable=false "Consider changing `ExampleInterface` to be prefixed with `I`."
replacements 0
""",
    },
]
