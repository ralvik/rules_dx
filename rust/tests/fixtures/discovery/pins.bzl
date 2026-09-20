"""Exact-target discovery pins.

Contract: `docs/cli/target-resolution.md#exact-target-discovery`,
`docs/native-toolchains.md`, `docs/environments/rust.md`.
"""

# Upstream discovery source: patched rules_rust discovery library.
RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"
RUST_PROJECT_RS = "tools/rust_analyzer/rust_project.rs"

# Dynamic discovery entry: rust-analyzer passes Path or Buildfile,
# never an exact target label (RustAnalyzerArg::Path/Buildfile).
DISCOVERY_ARG_VARIANTS = ["Path", "Buildfile"]

# Package-wide expansion shape the dynamic path takes:
# buildfile_to_targets maps a buildfile dir to `//pkg:all`
# (workspace root to `//...`). Exact context is lost there.
DISCOVERY_WIDENING_SHAPES = ["//pkg:all", "//..."]

# Target interfaces that keep exact context: gen_rust_project plus
# flycheck accept explicit TARGETS (proven by
# //rust/ide:ide_acquisition_test checking `--help` for TARGETS
# and the flycheck wrapper usage).
EXACT_TARGET_BINARIES = ["gen_rust_project", "flycheck"]

# Resolver-owned exact-target expression: unconfigured `bazel query`
# only, depth-1 direct owners over //..., deterministic sorted set.
# Query-only without this contract stays rejected.
RESOLVER_EXPRESSION_SHAPE = "kind('rule', rdeps(//..., set(<file-labels>), 1))"

# Project-owned crate graph and internal RustAnalyzerInfo stay rejected:
# no static graph is generated, and the internal provider is not a
# public discovery API.
REJECTED_ALTERNATIVES = [
    "query-only without contract",
    "path-only //pkg:all widening",
    "project-owned crate graph",
    "internal RustAnalyzerInfo",
]

# Exact-isolation fixture pair: src/lib.rs resolves to hello_lib only,
# src/main.rs resolves to hello only (plus their private upstreams).
# Package-wide //...:all would merge both; exact discovery keeps them apart.
DISCOVERY_FIXTURE_LIB_OWNER = "//rust/tests/fixtures/hello:hello_lib"
DISCOVERY_FIXTURE_BIN_OWNER = "//rust/tests/fixtures/hello:hello"
