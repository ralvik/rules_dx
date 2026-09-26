"""Native cohort matrix cells."""

CLANG_TIDY_LINT = """matrix/clang_tidy_dirty.c:4:3: warning: do not use 'else' after 'return' [readability-else-after-return]"""

CPPCHECK_LINT = """<?xml version="1.0" encoding="UTF-8"?>\n<results version="2">\n  <cppcheck version="2.21.0"/>\n  <errors>\n    <error id="nullPointer" severity="error" msg="Possible null pointer dereference: slot" verbose="Possible null pointer dereference: slot">\n      <location file="matrix/cppcheck_dirty.c" line="7" column="10"/>\n    </error>\n  </errors>\n</results>\n"""

STATICCHECK_LINT = """[{"code": "SA4006", "severity": "warning", "location": {"file": "matrix/staticcheck_dirty.go", "line": 5, "column": 2}, "end": {"line": 5, "column": 3}, "message": "this value of x is never used"}]"""

GOVET_LINT = """matrix/govet_dirty.go:7:2: non-constant format string in call to fmt.Printf"""

ERRCHECK_LINT = """matrix/errcheck_dirty.go:7:2: unchecked error: f.WriteString("hello")"""

NATIVE_CASES = [
    {
        "name": "matrix_c_format_pass",
        "generated": {
            "matrix/clang_format_clean.c": "// Seed C format fixture.\n#include <stdio.h>\n\nint greet(const char *name) {\n  return 0;\n}\n",
        },
        "capability": "format",
        "stages": ["clang_format;c;matrix/clang_format_clean.c"],
        "tool_names": ["clang_format"],
        "tool_binaries": ["//quality/testdata:fake_clang_format"],
        "expected": """producer //quality/testdata:matrix_c_format_pass
capability FORMAT
stages 1
stage clang_format classes=c sources=matrix/clang_format_clean.c
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_c_format_fail",
        "generated": {
            "matrix/clang_format_dirty.c": "// Seed C format fixture.\n#include <stdio.h>\n\nint greet( const char*name){ BADFMT return 0; }\n",
        },
        "capability": "format",
        "stages": ["clang_format;c;matrix/clang_format_dirty.c"],
        "tool_names": ["clang_format"],
        "tool_binaries": ["//quality/testdata:fake_clang_format"],
        "expected": """producer //quality/testdata:matrix_c_format_fail
capability FORMAT
stages 1
stage clang_format classes=c sources=matrix/clang_format_dirty.c
completed_rounds 2
convergence STABLE
initial 1
initial WARNING clang_format - matrix/clang_format_dirty.c 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/clang_format_dirty.c 75 81 "fixed"
""",
    },
    {
        "name": "matrix_cpp_format_pass",
        "generated": {
            "matrix/clang_format_clean.cpp": "// Seed C++ format fixture.\n\nint greet(const char *name) {\n  return 0;\n}\n",
        },
        "capability": "format",
        "stages": ["clang_format;cpp;matrix/clang_format_clean.cpp"],
        "tool_names": ["clang_format"],
        "tool_binaries": ["//quality/testdata:fake_clang_format"],
        "expected": """producer //quality/testdata:matrix_cpp_format_pass
capability FORMAT
stages 1
stage clang_format classes=cpp sources=matrix/clang_format_clean.cpp
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_cpp_format_fail",
        "generated": {
            "matrix/clang_format_dirty.cpp": "// Seed C++ format fixture.\n\nint greet( const char*name){ BADFMT return 0; }\n",
        },
        "capability": "format",
        "stages": ["clang_format;cpp;matrix/clang_format_dirty.cpp"],
        "tool_names": ["clang_format"],
        "tool_binaries": ["//quality/testdata:fake_clang_format"],
        "expected": """producer //quality/testdata:matrix_cpp_format_fail
capability FORMAT
stages 1
stage clang_format classes=cpp sources=matrix/clang_format_dirty.cpp
completed_rounds 2
convergence STABLE
initial 1
initial WARNING clang_format - matrix/clang_format_dirty.cpp 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/clang_format_dirty.cpp 58 64 "fixed"
""",
    },
    {
        "name": "matrix_c_lint_pass",
        "generated": {
            "matrix/clang_tidy_clean.c": "// Seed C lint fixture.\n#include <stdio.h>\n\nint check(int value) {\n  return value;\n}\n",
        },
        "capability": "lint",
        "stages": ["clang_tidy;c;matrix/clang_tidy_clean.c"],
        "upstream_tools": ["clang_tidy"],
        "upstream_generated": {"clang_tidy": ""},
        "expected": """producer //quality/testdata:matrix_c_lint_pass
capability LINT
stages 1
stage clang_tidy classes=c sources=matrix/clang_tidy_clean.c
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_c_lint_fail",
        "generated": {
            "matrix/clang_tidy_dirty.c": "// Seed C lint fixture.\n#include <stdio.h>\n\nint check(int value) {\n  if (value) {\n    return 1;\n  } else {\n    return 0;\n  }\n}\n",
        },
        "capability": "lint",
        "stages": ["clang_tidy;c;matrix/clang_tidy_dirty.c"],
        "upstream_tools": ["clang_tidy"],
        "upstream_generated": {"clang_tidy": CLANG_TIDY_LINT},
        "expected": """producer //quality/testdata:matrix_c_lint_fail
capability LINT
stages 1
stage clang_tidy classes=c sources=matrix/clang_tidy_dirty.c
completed_rounds 1
convergence STABLE
initial 1
initial WARNING clang_tidy readability-else-after-return matrix/clang_tidy_dirty.c 46 46 fixable=false "do not use 'else' after 'return'"
terminal 1
terminal WARNING clang_tidy readability-else-after-return matrix/clang_tidy_dirty.c 46 46 fixable=false "do not use 'else' after 'return'"
replacements 0
""",
    },
    {
        "name": "matrix_cpp_lint_pass",
        "generated": {
            "matrix/cppcheck_clean.c": "// Seed C++ lint fixture.\n\nint check(int value) {\n  return value;\n}\n",
        },
        "capability": "lint",
        "stages": ["cppcheck;cpp;matrix/cppcheck_clean.c"],
        "upstream_tools": ["cppcheck"],
        "upstream_generated": {"cppcheck": ""},
        "expected": """producer //quality/testdata:matrix_cpp_lint_pass
capability LINT
stages 1
stage cppcheck classes=cpp sources=matrix/cppcheck_clean.c
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_cpp_lint_fail",
        "generated": {
            "matrix/cppcheck_dirty.c": "// Seed C++ lint fixture.\nint check(int value) {\n  int *slot = nullptr;\n  if (value > 0) {\n    slot = &value;\n  }\n  return *slot;\n}\n",
        },
        "capability": "lint",
        "stages": ["cppcheck;cpp;matrix/cppcheck_dirty.c"],
        "upstream_tools": ["cppcheck"],
        "upstream_generated": {"cppcheck": CPPCHECK_LINT},
        "expected": """producer //quality/testdata:matrix_cpp_lint_fail
capability LINT
stages 1
stage cppcheck classes=cpp sources=matrix/cppcheck_dirty.c
completed_rounds 1
convergence STABLE
initial 1
initial ERROR cppcheck nullPointer matrix/cppcheck_dirty.c 123 123 fixable=false "Possible null pointer dereference: slot"
terminal 1
terminal ERROR cppcheck nullPointer matrix/cppcheck_dirty.c 123 123 fixable=false "Possible null pointer dereference: slot"
replacements 0
""",
    },
    {
        "name": "matrix_go_format_pass",
        "generated": {
            "matrix/gofumpt_clean.go": "// Seed Go format fixture.\npackage gofumpt\n\nfunc greet(name string) string {\n\treturn \"hello \" + name\n}\n",
        },
        "capability": "format",
        "stages": ["gofumpt;go;matrix/gofumpt_clean.go"],
        "tool_names": ["gofumpt"],
        "tool_binaries": ["//quality/testdata:fake_gofumpt"],
        "expected": """producer //quality/testdata:matrix_go_format_pass
capability FORMAT
stages 1
stage gofumpt classes=go sources=matrix/gofumpt_clean.go
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_go_format_fail",
        "generated": {
            "matrix/gofumpt_dirty.go": "// Seed Go format fixture.\npackage gofumpt\n\nfunc greet( name string)string{ BADFMT return \"hello \"+name }\n",
        },
        "capability": "format",
        "stages": ["gofumpt;go;matrix/gofumpt_dirty.go"],
        "tool_names": ["gofumpt"],
        "tool_binaries": ["//quality/testdata:fake_gofumpt"],
        "expected": """producer //quality/testdata:matrix_go_format_fail
capability FORMAT
stages 1
stage gofumpt classes=go sources=matrix/gofumpt_dirty.go
completed_rounds 2
convergence STABLE
initial 1
initial WARNING gofumpt - matrix/gofumpt_dirty.go 0 0 fixable=true "file is not formatted"
terminal 0
replacements 1
replacement matrix/gofumpt_dirty.go 76 82 "fixed"
""",
    },
    {
        "name": "matrix_go_staticcheck_pass",
        "generated": {
            "matrix/staticcheck_clean.go": "// Seed Go lint fixture.\npackage staticcheck\n\nfunc check(name string) string {\n\treturn \"hello \" + name\n}\n",
        },
        "capability": "lint",
        "stages": ["staticcheck;go;matrix/staticcheck_clean.go"],
        "upstream_tools": ["staticcheck"],
        "upstream_generated": {"staticcheck": "[]"},
        "expected": """producer //quality/testdata:matrix_go_staticcheck_pass
capability LINT
stages 1
stage staticcheck classes=go sources=matrix/staticcheck_clean.go
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_go_staticcheck_fail",
        "generated": {
            "matrix/staticcheck_dirty.go": "// Seed Go lint fixture.\npackage staticcheck\n\nfunc check(name string) string {\n\tx := 1\n\t_ = x\n\treturn \"hello \" + name\n}\n",
        },
        "capability": "lint",
        "stages": ["staticcheck;go;matrix/staticcheck_dirty.go"],
        "upstream_tools": ["staticcheck"],
        "upstream_generated": {"staticcheck": STATICCHECK_LINT},
        "expected": """producer //quality/testdata:matrix_go_staticcheck_fail
capability LINT
stages 1
stage staticcheck classes=go sources=matrix/staticcheck_dirty.go
completed_rounds 1
convergence STABLE
initial 1
initial WARNING staticcheck SA4006 matrix/staticcheck_dirty.go 80 81 fixable=false "this value of x is never used"
terminal 1
terminal WARNING staticcheck SA4006 matrix/staticcheck_dirty.go 80 81 fixable=false "this value of x is never used"
replacements 0
""",
    },
    {
        "name": "matrix_go_govet_pass",
        "generated": {
            "matrix/govet_clean.go": "// Seed Go vet fixture.\npackage govet\n\nfunc check(name string) string {\n\treturn \"hello \" + name\n}\n",
        },
        "capability": "lint",
        "stages": ["govet;go;matrix/govet_clean.go"],
        "upstream_tools": ["govet"],
        "upstream_generated": {"govet": ""},
        "expected": """producer //quality/testdata:matrix_go_govet_pass
capability LINT
stages 1
stage govet classes=go sources=matrix/govet_clean.go
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_go_govet_fail",
        "generated": {
            "matrix/govet_dirty.go": "// Seed Go vet fixture.\npackage govet\n\nimport \"fmt\"\n\nfunc check(name string) string {\n\tfmt.Printf(name)\n\treturn \"hello \" + name\n}\n",
        },
        "capability": "lint",
        "stages": ["govet;go;matrix/govet_dirty.go"],
        "upstream_tools": ["govet"],
        "upstream_generated": {"govet": GOVET_LINT},
        "expected": """producer //quality/testdata:matrix_go_govet_fail
capability LINT
stages 1
stage govet classes=go sources=matrix/govet_dirty.go
completed_rounds 1
convergence STABLE
initial 1
initial WARNING govet - matrix/govet_dirty.go 87 87 fixable=false "non-constant format string in call to fmt.Printf"
terminal 1
terminal WARNING govet - matrix/govet_dirty.go 87 87 fixable=false "non-constant format string in call to fmt.Printf"
replacements 0
""",
    },
    {
        "name": "matrix_go_errcheck_pass",
        "generated": {
            "matrix/errcheck_clean.go": "// Seed Go errcheck fixture.\npackage errcheck\n\nfunc persist(path string) {\n}\n",
        },
        "capability": "lint",
        "stages": ["errcheck;go;matrix/errcheck_clean.go"],
        "upstream_tools": ["errcheck"],
        "upstream_generated": {"errcheck": ""},
        "expected": """producer //quality/testdata:matrix_go_errcheck_pass
capability LINT
stages 1
stage errcheck classes=go sources=matrix/errcheck_clean.go
completed_rounds 1
convergence STABLE
initial 0
terminal 0
replacements 0
""",
    },
    {
        "name": "matrix_go_errcheck_fail",
        "generated": {
            "matrix/errcheck_dirty.go": "// Seed Go errcheck fixture.\npackage errcheck\n\nimport \"os\"\n\nfunc persist(path string) {\n\tf, _ := os.Create(path)\n\tf.WriteString(\"hello\")\n\tf.Close()\n}\n",
        },
        "capability": "lint",
        "stages": ["errcheck;go;matrix/errcheck_dirty.go"],
        "upstream_tools": ["errcheck"],
        "upstream_generated": {"errcheck": ERRCHECK_LINT},
        "expected": """producer //quality/testdata:matrix_go_errcheck_fail
capability LINT
stages 1
stage errcheck classes=go sources=matrix/errcheck_dirty.go
completed_rounds 1
convergence STABLE
initial 1
initial WARNING errcheck - matrix/errcheck_dirty.go 89 89 fixable=false "unchecked error: f.WriteString(\\"hello\\")"
terminal 1
terminal WARNING errcheck - matrix/errcheck_dirty.go 89 89 fixable=false "unchecked error: f.WriteString(\\"hello\\")"
replacements 0
""",
    },
]
