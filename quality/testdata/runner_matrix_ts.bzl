"""Split from `runner_matrix_cases.bzl`. No behavior change."""

TS_CASES = [
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
