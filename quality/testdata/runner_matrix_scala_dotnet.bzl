
SCALAFIX_LINT = """{"path": "matrix/scalafix_dirty.scala", "line": 4, "column": 3, "rule": "DisableSyntax.var", "message": "mutable state should be avoided", "severity": "error"}"""

ROSLYN_LINT = """{"version": "2.1.0", "runs": [{"results": [{"ruleId": "CA1822", "level": "warning", "message": {"text": "Member 'Greet' does not access instance data"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "matrix/roslyn_dirty.cs"}, "region": {"startLine": 6, "startColumn": 28, "endLine": 6, "endColumn": 33}}}]}]}]}"""

FSHARPLINT_LINT = """{"path": "matrix/fsharplint_dirty.fs", "rule": "FL0036", "message": "Consider changing ExampleInterface to be prefixed with I.", "startLine": 4, "startColumn": 6, "endLine": 4, "endColumn": 22}"""

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
        "expected": """producer //quality/testdata:matrix_scala_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_scala_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_scala_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_scala_lint_fail""",
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
        "expected": """producer //quality/testdata:matrix_csharp_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_csharp_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_csharp_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_csharp_lint_fail""",
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
        "expected": """producer //quality/testdata:matrix_fsharp_format_pass""",
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
        "expected": """producer //quality/testdata:matrix_fsharp_format_fail""",
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
        "expected": """producer //quality/testdata:matrix_fsharp_lint_pass""",
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
        "expected": """producer //quality/testdata:matrix_fsharp_lint_fail""",
    },
]
