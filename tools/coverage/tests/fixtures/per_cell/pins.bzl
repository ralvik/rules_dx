"""Per-cell coverage plus Codecov plus remote pins.

Contract: `docs/testing/README.md#coverage`, `docs/github-ci.md#scope-and-status`.
Fixture: `tools/coverage/tests/fixtures/per_cell/` via
`bazel run //tools/ci:coverage_qualification`.
"""

# Per-cell registry: seven qualified cells, same first-party scope.
# Each cell gates its own combined LCOV report through `coverage_bin`
# against its versioned inventory; no cross-cell union, no averaged
# percentages, no rounding up. Only the inventory path differs per cell.
PER_CELL_REGISTRY = "tools/coverage/cells.txt"
PER_CELL_COUNT = 7
PER_CELL_SEED = "qualified seed-linux_x86_64 tools/coverage/seed-inventory.txt"
PER_CELL_ARM64 = "qualified linux_arm64 tools/coverage/arm64-inventory.txt"
PER_CELL_MUSL_X86_64 = "qualified linux_x86_64_musl tools/coverage/musl-x86_64-inventory.txt"
PER_CELL_MUSL_ARM64 = "qualified linux_arm64_musl tools/coverage/musl-arm64-inventory.txt"
PER_CELL_MACOS_ARM64 = "qualified macos_arm64 tools/coverage/macos-arm64-inventory.txt"
PER_CELL_MACOS_X86_64 = "qualified macos_x86_64 tools/coverage/macos-x86_64-inventory.txt"
PER_CELL_WINDOWS_X86_64 = "qualified windows_x86_64 tools/coverage/windows-x86_64-inventory.txt"
PER_CELL_NO_UNION = "no cross-cell union"
PER_CELL_NO_UNION_CONSUMER = "no union"
PER_CELL_SEED_INVENTORY = "tools/coverage/seed-inventory.txt"
PER_CELL_INVENTORY_SCOPE = "eligible cli/lcov/src/lib.rs"
PER_CELL_RATE_GATE = "coverage --min-coverage"
PER_CELL_RATE_PIN = "coverage --min-coverage 97 //..."

# Accepted decision (issue comment): 100% non-ignored lines in this repo
# via the seed exact gate (zero uncovered lines in the versioned
# inventory scope), configurable `--min-coverage` requirement for users
# per-cell. Codecov stays opt-in only, never required. Seed-cell
# enforcement lives in `tools/coverage/seed-inventory.txt`; cells,
# languages, and metrics stay separate with no cross-cell union.
DECISION_REPO_EXACT = "zero uncovered lines"
DECISION_USER_CONFIGURABLE = "configurable requirement for users per-cell"
DECISION_CODECOV_OPT_IN = "Codecov stays opt-in only"
DECISION_CODECOV_NEVER_REQUIRED = "never required"
DECISION_SEED_INVENTORY = "tools/coverage/seed-inventory.txt"
DECISION_NO_UNION = "no cross-cell union"

# Starlark decision: genuine executable-line instrumentation is
# infeasible under the pinned Bazel, so the evidence-backed behavioral
# matrix is the fallback. Never a line-coverage percentage, never
# combined into source coverage.
STARLARK_PROBE = "0-byte `coverage.dat`"
STARLARK_UPSTREAM = "bazelbuild/bazel#15594"
STARLARK_BAZEL_VERSION = "9.2.0"
STARLARK_MATRIX = "behavioral matrix"
STARLARK_VALIDATION = "matrix_validation"
STARLARK_NO_LINE_COVERAGE = "no Starlark line-coverage percentage"
STARLARK_NEVER_PRESENTED = "never be presented as source-line"

# Codecov stays opt-in only: no activation, no upload wiring anywhere.
# The adopted PR surface is the first-party comment rendered from the
# Bazel-owned gate verdict.
CODECOV_NO_ACTION = "codecov-action"
CODECOV_NO_TOKEN = "CODECOV_TOKEN"
CODECOV_NO_UPLOAD = "codecov upload"
CODECOV_FIRST_PARTY = "first-party"
CODECOV_OPT_IN_DOC = "Codecov stays at most opt-in"
CODECOV_NO_ACCOUNT = "No Codecov account"
CODECOV_OPT_IN_CI = "Codecov stays opt-in only"

# Free-tier quotas: only standard runners plus the services actually
# used. Windows plus macOS runners below are the qualified standard
# free runners; macos-latest stays unpinned and
# self-hosted/larger stay paid and banned.
QUOTA_RUNNERS_FREE = "standard GitHub-hosted runners is free"
QUOTA_UBUNTU = "runs-on: ubuntu-latest"
QUOTA_WINDOWS = "runs-on: windows-latest"
QUOTA_CACHE = "actions/cache"
QUOTA_CACHE_SIZE = "10 GB"
QUOTA_ARTIFACT_SIZE = "500 MB"
QUOTA_LARGER_CHARGED = "Larger runners are always charged"
QUOTA_BANNED = [
    "self-hosted",
    "larger",
    "macos-latest",
]

# Remote stays local-only: no remote cache or executor wired anywhere.
# Hermeticity is designed and locally sandbox-tested (aquery action
# shape plus execution-log cache hits); remote behavior remains
# unverified and no remote-correctness claim is made.
REMOTE_NO_CACHE = "--remote_cache"
REMOTE_NO_EXECUTOR = "--remote_executor"
REMOTE_NO_BES = "--bes_backend"
REMOTE_LOCAL_ONLY = "local execution, no remote"
REMOTE_PLATFORM_OWNER = "issue #298"
REMOTE_ELSE_BRANCH = "locally sandbox-tested but remote behavior remains unverified"
REMOTE_HERMETICITY = "hermeticity is"
REMOTE_AQUERY = "cache hit"
REMOTE_NO_CORRECTNESS_CLAIM = "remote-cache correctness"

# Rejected substitutes per the issue alternatives: seed-only forever
# blocks Supported and needs an owner, so it stays rejected here.
REJECTED_ALTERNATIVES = [
    "seed-only forever",
    "cross-cell union",
    "averaged percentages",
    "rounding up",
    "required Codecov",
    "remote-cache correctness",
]

# Live proof labels: the gate binary plus the versioned inventories
# prove per-cell gating on the seed host; the behavioral matrix plus
# the local cache harnesses prove the Starlark plus remote halves.
PER_CELL_GATE_BIN = "//tools/coverage:coverage_bin"
PER_CELL_LIVE_PROOFS = [
    "//tools/coverage:coverage_bin",
    "//cli/lcov:dx_lcov",
]
PER_CELL_FIXTURE_CORPUS = "//tools/coverage/tests/fixtures/per_cell:corpus_starlark"
OWNED_GAPS_NOTE = "platform plus consumer plus release evidence stays owned gap"
NO_SUPPORTED_CLAIM = "no Supported claim"
