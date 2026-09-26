"""Exact-target discovery pins."""

RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"
RUST_PROJECT_RS = "tools/rust_analyzer/rust_project.rs"

DISCOVERY_ARG_VARIANTS = ["Path", "Buildfile"]

DISCOVERY_WIDENING_SHAPES = ["//pkg:all", "//..."]

EXACT_TARGET_BINARIES = ["gen_rust_project", "flycheck"]

RESOLVER_EXPRESSION_SHAPE = "kind('rule', rdeps(//..., set(<file-labels>), 1))"

REJECTED_ALTERNATIVES = [
    "query-only without contract",
    "path-only //pkg:all widening",
    "project-owned crate graph",
    "internal RustAnalyzerInfo",
]

DISCOVERY_FIXTURE_LIB_OWNER = "//rust/tests/fixtures/hello:hello_lib"
DISCOVERY_FIXTURE_BIN_OWNER = "//rust/tests/fixtures/hello:hello"
