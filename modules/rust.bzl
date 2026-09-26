"""Rust foundation pins plus crate-manifest groups. Contract: docs/decisions/0019-first-release-additional-foundations.md."""

# Pinned Rust foundation (see MODULE.bazel; per-split pin_consistency in tools/ci/pin_consistency.sh).
RULES_RUST_VERSION = "0.74.0"
RULES_RUST_PROST_VERSION = "0.74.0"
RUST_VERSION = "1.98.0"
RUSTFMT_VERSION = "1.98.0"

# Crate-manifest groups for the single `crates` hub (see MODULE.bazel crate.from_cargo).
# The hub stays single while one Cargo workspace lock covers every manifest; the groups
# own the inventory so a future multi-hub split moves whole groups, never single labels.
# See: //modules:npm-uv-crates.bzl (target hub layout).
CRATE_FIXTURE_MANIFESTS = [
    "//rust/tests/fixtures/hello:Cargo.toml",
]

CRATE_CLI_MANIFESTS = [
    "//cli/process:Cargo.toml",
    "//cli/apply:Cargo.toml",
    "//cli/atomic_fs:Cargo.toml",
    "//cli/adopt:Cargo.toml",
    "//cli/setup:Cargo.toml",
    "//cli/clean:Cargo.toml",
    "//cli/audit:Cargo.toml",
    "//cli/output:Cargo.toml",
    "//cli/path:Cargo.toml",
    "//cli/proto_validate:Cargo.toml",
    "//cli/schema:Cargo.toml",
    "//cli/bep:Cargo.toml",
    "//cli/cli:Cargo.toml",
    "//cli/diff:Cargo.toml",
    "//cli/digest:Cargo.toml",
    "//cli/lcov:Cargo.toml",
    "//cli/env:Cargo.toml",
    "//cli/codegen:Cargo.toml",
    "//cli/env_plan:Cargo.toml",
    "//cli/fingerprint:Cargo.toml",
    "//cli/test_scratch:Cargo.toml",
    "//cli/roots:Cargo.toml",
    "//cli/docgen:Cargo.toml",
    "//cli/update:Cargo.toml",
    "//cli/bump:Cargo.toml",
    "//cli/ci:Cargo.toml",
    "//cli/qualification:Cargo.toml",
]

CRATE_DEPLOY_MANIFESTS = [
    "//deploy/rules:Cargo.toml",
    "//deploy/release:Cargo.toml",
    "//deploy/install:Cargo.toml",
]

CRATE_SHARD_WRITER_MANIFESTS = [
    "//env/env_shard:Cargo.toml",
    "//generation/codegen_shard:Cargo.toml",
]

CRATE_SHARED_MANIFESTS = [
    "//generation/result:Cargo.toml",
    "//quality/adapter:Cargo.toml",
    "//quality/evaluator:Cargo.toml",
    "//quality/markdown:Cargo.toml",
    "//quality/result:Cargo.toml",
    "//quality/runner:Cargo.toml",
    "//docs/ir/ir:Cargo.toml",
    "//docs/adapters:Cargo.toml",
    "//tools/bazelrc:Cargo.toml",
    "//tools/depcheck:Cargo.toml",
]

# Hub input mirror: every group exactly once (MODULE.bazel cannot load
# wrappers, so it mirrors this list (drift in either direction breaks the
# hub; lockfile bytes are resolver-owned
# and order-independent, so grouping is ownership, not build order).
RUST_CRATE_MANIFESTS = (
    CRATE_FIXTURE_MANIFESTS +
    CRATE_CLI_MANIFESTS +
    CRATE_DEPLOY_MANIFESTS +
    CRATE_SHARD_WRITER_MANIFESTS +
    CRATE_SHARED_MANIFESTS
)

# Hub repin (single workspace lock; see MODULE.bazel).
CRATE_REPIN = "CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello"
CRATE_CARGO_LOCK = "//rust/tests/fixtures/hello:Cargo.lock"
CRATE_BAZEL_LOCK = "//:cargo-bazel-lock.json"

# Shell-env policy: first-party scripts default hermetic via the generator,
# third-party scripts defer to the global flag pinned in .bazelrc with zero
# opt-ins (see MODULE.bazel crate.from_cargo, .bazelrc, gazelle/rust/lang_generate.go).
SHELL_ENV_GENERATOR_DEFAULT = 0
SHELL_ENV_GLOBAL_FLAG = False
