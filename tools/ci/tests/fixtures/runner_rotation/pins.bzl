"""Runner plus SDK rotation pins.

Contract: `docs/github-ci.md#runner-plus-sdk-rotation`.
Fixture: `tools/ci/tests/fixtures/runner_rotation/` via
`bazel run //tools/ci:runner_rotation_qualification`.
"""

# Qualified runner set (floating GitHub labels, pinned here, not floating in docs).
RUNNER_SEED = "ubuntu-latest"
RUNNER_ARM64 = "ubuntu-24.04-arm"
RUNNER_MACOS_ARM64 = "macos-14"
RUNNER_WINDOWS = "windows-latest"

# Retirement record (handled by review, never silent).
RETIRED_MACOS_13 = "macos-13 retired December 2025"
NOT_PLANNED_MACOS_X86_64 = "macos x86_64 Not planned per #976 with no runner"
FLOATING_NOTE = "ubuntu-latest plus windows-latest float and age out"

# Review cadence plus owner (sole maintainer via CODEOWNERS, no review owner gap).
CADENCE = "quarterly review plus on retirement notice plus on hermetic-llvm release"
OWNER = "sole maintainer owns every row until delegation"

# SDK plus floor review scope (exact values owned by issues #410-#412 plus #414 plus #500, not pinned here).
SDK_GLIBC = "glibc 2.28 symbol floor"
SDK_APPLE = "MacOSX26.5 via hermetic-llvm v0.8.19 pinned extraction"
SDK_WINDOWS = "MSVC 14.50.35717 plus redist 14.50.35710 plus SDK package 10.0.26100.7705"

# Customer-flows-only qualification (build plus test plus coverage, no new CI job).
CUSTOMER_BUILD = "bazel build //..."
CUSTOMER_TEST = "bazel test //..."
CUSTOMER_COVERAGE = "dx coverage --min-coverage 97 //..."
LOCAL_ON_DEMAND = "qualified locally/on-demand with customer flows only"

# Rejected substitutes per the issue alternatives.
REJECTED_ROTATION_JOB = "permanent rotation job in CI rejected"
REJECTED_CUSTOMER_ONLY = "keep CI customer-only"
REJECTED_NO_NEW_JOB = "no new non-customer CI jobs"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #642"
INFRA_ONLY = "infra only"
