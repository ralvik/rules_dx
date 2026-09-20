"""Env plus codegen pins.

Contract: `docs/environments/environment.md#test-requirements`, `docs/environments/codegen.md#test-requirements`,
`docs/environments/managed-state.md#test-requirements`, `docs/environments/node.md#importer-projection`.
Fixture: `env/tests/fixtures/env_codegen/` via
`bazel run //tools/ci:env_codegen_qualification`.
"""

# Public env contribution protocol stays deferred PATH-tools-only.
PROTOCOL_BOUNDARY = "PATH-tools-only"
PROTOCOL_PROVIDER_NOTE = "EnvironmentInfo stays PATH-tool-only"
PROTOCOL_DEFERRED_NOTE = "third-party language-integration plugins are deferred past v1"
PROTOCOL_PUBLIC_API = "environment_tool(name, executable, bin_name)"
PROTOCOL_PLUGIN_REJECTED = "not third-party environment plugins"

# Windows fallback stays unsupported: manual PATH plus symlink-only.
WINDOWS_GUIDANCE = "Windows keeps manual `PATH` guidance"
WINDOWS_NO_FALLBACK = "no junction or copy fallback"
WINDOWS_FAIL_BEFORE_MUTATION = "failure before mutation"
WINDOWS_MANAGED_NOTE = "There is no launcher, junction, copy"
WINDOWS_MANAGED_FAIL = "missing capability fails before mutation"
WINDOWS_CAPABILITY_NOTE = "Developer Mode symlink capability"

# Direnv snippet stays PATH-only with watch plus missing-dir guidance.
DIRENV_PATH_ADD = "PATH_add"
DIRENV_WATCH = "watch_file"
DIRENV_NO_BAZEL = "never invokes Bazel"
DIRENV_SCAFFOLD_REFUSAL = "init_must_refuse"

# Standalone-without-Bazel stays Bazel-first with staged standalone.
STANDALONE_BAZEL_FIRST = "bazel run //dx:env"
STANDALONE_TARGET = "dx_standalone"
STANDALONE_STAGED_NOTE = "until releases are cut"
STANDALONE_NO_CHECKSUM_ONLY = "no checksum-only fallback"

# Signing plus trust stays implemented-verifier plus deferred generation.
SIGNING_TUF_ROOT = "https://tuf-repo-cdn.sigstore.dev"
SIGNING_COSIGN = "cosign verify-blob"
SIGNING_NO_CHECKSUM_ONLY = "no checksum-only"
SIGNING_DRAFT_GATE = "draft != True"
SIGNING_DRAFT_DEFAULT = "draft = True"
SIGNING_DRYRUN_TAG = "v0.0.0-dryrun"
SIGNING_SBOM_NOTE = "SBOM/provenance in `deploy/release/sbom.bzl`"
SIGNING_BCR_NOTE = "BCR submission are implemented"

# Bootstrap fixture evidence: spaces path plus noop plus unmanaged plus marker.
BOOTSTRAP_SPACES = "work space"
BOOTSTRAP_NOOP = "already current"
BOOTSTRAP_UNMANAGED = "unmanaged"
BOOTSTRAP_MARKER = ".rules_dx_managed"
BOOTSTRAP_THREE_TOOLS = "installed 3 tool(s)"

# Fidelity stays symlink exec links preserving invocation semantics.
FIDELITY_PRESERVES = "Executable links preserve arguments, working directory, environment, exit status, and signal"

# Lock plus atomic-commit evidence stays pinned in cli/env plus atomic_fs.
LOCK_EXCLUSIVE = "lock_exclusive"
LOCK_TRY = "file.try_lock()"
LOCK_ACQUIRE = "acquire_lock"
LOCK_DEADLINE = "ten seconds"
LOCK_STD = "File::try_lock"
LOCK_COMMIT_NOTE = "one workspace commit lock"
LOCK_ATOMIC_POINTER = "atomically replaces `.dx/setups/current`"

# Stale behavior stays explicit repair with no implicit Bazel startup.
STALE_BAZEL_CLEAN = "bazel clean"
STALE_OUTPUT_BASE = "output-base change"
STALE_EXPLICIT_REPAIR = "only explicit `dx codegen` repairs"
STALE_NO_IMPLICIT = "without implicit Bazel startup"

# IDE stays separate output base with no selection mutation.
IDE_SEPARATE_BASE = "Use a separate IDE output base"
IDE_RUST_DISCOVERY = "gen_rust_project"
IDE_RUST_FLYCHECK = "flycheck"
IDE_GO_DRIVER = "GOPACKAGESDRIVER"
IDE_NO_MUTATION = "do not mutate the selected environment"

# BEP stays one stream with private output groups and no second downloader.
BEP_ONE_STREAM = "one BEP stream"
BEP_ENV_GROUP = "dx_env_plans"
BEP_CODEGEN_GROUP = "dx_codegen_plans"
BEP_NO_SECOND_DOWNLOADER = "never implements a second remote-cache downloader"
BEP_ENV_SUFFIX = ".dxenv.pb"
BEP_CODEGEN_SUFFIX = ".dxcodegen.pb"

# Projection stays symlink-only read-only with pnpm-selected node stores.
PROJECTION_SYMLINK_ONLY = "symlink-only"
PROJECTION_READ_ONLY = "read-only generated artifacts"
PROJECTION_NO_PNPM_INSTALL = "never invokes `pnpm install`"
PROJECTION_IMPORTER_FACADE = "importer-local `node_modules` facades"
PROJECTION_MIRROR = ".dx/setups/current/generated"

# Roots stay frozen on the //... baseline with exact-target bypass.
ROOTS_FROZEN_STRATEGY = "FROZEN_STRATEGY"
ROOTS_RECURSIVE_PATTERN = "RecursivePattern"
ROOTS_BASELINE = "//..."
ROOTS_FROZEN_EVIDENCE = "FROZEN_EVIDENCE"
ROOTS_INCREMENTALITY = "INCREMENTALITY_EVIDENCE"
ROOTS_EXACT_BYPASS = "exact-target bypass"
ROOTS_DECLARED_ONLY = "declared-BUILD-only"

# Cold plus warm stays frozen baseline by fiat with no timing claims.
COLD_WARM_SCORE = "cold_ms + WARM_WEIGHT"
COLD_WARM_WEIGHT = "WARM_WEIGHT"
COLD_WARM_NO_TIMING_CLAIM = "no timing claims are made per [ADR 0022]"
COLD_WARM_BASELINE_WINS = "baseline wins ties"

# WP slices plus admitted plus frozen plus rejected substitutes.
WP1_ADMITTED_PAIRS = ("protobuf", "rust")
WP2_ADMITTED_INTEGRATIONS = ("rust",)
WP_FROZEN_ENV_GROUP = "dx_env_plans"
WP_FROZEN_CODEGEN_GROUP = "dx_codegen_plans"
WP_REJECTED = [
    "junction or copy fallback",
    "checksum-only fallback",
    "third-party plugin claim",
    "second remote-cache downloader",
    "timing claim",
]

# Fixture corpus plus live proofs plus owned gaps.
ENV_CODEGEN_FIXTURE_CORPUS = "//env/tests/fixtures/env_codegen:corpus_starlark"
ENV_CODEGEN_SEED_PROOFS = [
    "//env:env_shard_alpha",
    "//generation/codegen_shard:codegen_shard",
    "//cli/roots:roots_pattern_fixture",
]
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
