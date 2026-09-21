"""Build-profile flags plus DX_PROFILE forwarding fixture.

Contract: `docs/cli/commands/build-test-coverage.md#build-profiles`,
`docs/deploy/authoring.md`, `docs/decisions/0021-build-profiles.md`.
Fixture: `cli/cli/tests/fixtures/build_profiles/` via
`bazel run //tools/ci:build_profiles_qualification`.
Unit fixtures: `cli/cli/src/args/profile.rs` (`profile_*`),
`cli/cli/src/plan/workflow.rs` (`workflow_profile_*`),
`cli/cli/src/plan/run_deploy.rs` (`run_profile_*`),
`cli/cli/src/exec/deploy.rs` (`deploy_flag_over_attr_*`),
`deploy/rules/deploy_tests.bzl`.
"""

# Flags belong to build, run, test, and deploy only.
PROFILE_COMMANDS = [
    "build",
    "run",
    "test",
    "deploy",
]
PROFILE_FLAGS = ["--debug", "--release"]

# Shared configs behind stable dx_* names (ADR 0021).
PROFILE_DEBUG_CONFIG = "dx_debug"
PROFILE_DEV_CONFIG = "dx_dev"
PROFILE_RELEASE_CONFIG = "dx_release"
PROFILE_DEBUG_MODE = "dbg"
PROFILE_DEV_MODE = "fastbuild"
PROFILE_RELEASE_MODE = "opt"

# Command defaults: dev for build/run/test, release for deploy.
PROFILE_DEFAULT_BUILD = "dev"
PROFILE_DEFAULT_RUN = "dev"
PROFILE_DEFAULT_TEST = "dev"
PROFILE_DEFAULT_DEPLOY = "release"

# Precedence: explicit flag over deploy target profile over command default.
PROFILE_PRECEDENCE = "flag over attr over default"

# Bare invocation means dev (no --dev flag); both flags together fail (exit 2).
PROFILE_NO_DEV_FLAG = True
PROFILE_CONFLICT_EXIT = 2

# Coverage takes no profile flags; its argv is unchanged.
PROFILE_COVERAGE_UNCHANGED = True

# Deploy target profile attribute vocabulary (DxDeployInfo).
PROFILE_ATTR_VOCABULARY = ["debug", "dev", "release"]

# DX_PROFILE forwarding name plus values on the deploy run env.
DX_PROFILE_ENV = "DX_PROFILE"
DX_PROFILE_VALUES = ["debug", "dev", "release"]

# Rejected substitutes (never accepted as the resolution).
REJECTED_SILENT_IGNORE = "silent ignore rejected"
REJECTED_DEV_FLAG = "--dev flag rejected"
REJECTED_COVERAGE_PROFILE = "coverage profile rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #814"
