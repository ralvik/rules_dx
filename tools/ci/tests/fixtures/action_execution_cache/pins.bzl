"""Action execution plus disk-cache pins.

Contract: `docs/quality/action-model.md#outputs-remote-cache-and-execution`, `docs/testing/github-ci.md#workflow-hygiene`.
Fixture: `tools/ci/tests/fixtures/action_execution_cache/` via
`bazel run //tools/ci:action_execution_cache_qualification`.
"""

# Dx quality actions stay local-only until remote is qualified: every
# pipeline plus evaluator action carries no-remote-exec, never a bare
# remote-executable shape and never a cache-disabling marker.
EXECUTION_REQUIREMENTS = '{"no-remote-exec": "1"}'
EXECUTION_SYNTHETIC_SITES = 2
EXECUTION_REAL_SITES = 1
EXECUTION_FORBIDDEN = '"no-remote": "1"'

# Disk-cache keys stay exact-only: every Bazel-affecting lock/config is
# hashed and no prefix fallback reuses a stale entry after a bust.
CACHE_KEY_FILES = [
    "MODULE.bazel",
    "MODULE.bazel.lock",
    ".bazelrc",
    "tools/bazelrc/preset.bazelrc",
    ".bazelversion",
    "cargo-bazel-lock.json",
    "Cargo.lock",
    "Cargo.toml",
    "pnpm-lock.yaml",
    "maven_install.json",
    "go.mod",
    "go.sum",
    "uv.lock",
    "pyproject.toml",
]
CACHE_NO_FALLBACK = "exact key only, bust starts cold"
CACHE_SCOPES = [
    "bazel-seed-",
    "bazel-arm64-",
    "bazel-musl-x86_64-",
    "bazel-musl-arm64-",
    "bazel-macos-arm64-",
    "bazel-windows-x86_64-",
]

# Remote boundary stays local-only: no remote cache/executor/BES flags,
# aquery plus execution-log evidence is local, remote stays unverified.
REMOTE_NO_FLAGS = ["--remote_cache", "--remote_executor", "--bes_backend"]
REMOTE_ELSE_BRANCH = "locally sandbox-tested but remote behavior remains unverified"
