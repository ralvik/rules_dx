"""Action execution plus shared-cache pins.

Contract: `docs/quality/action-model.md#outputs-remote-cache-and-execution`, `docs/testing/github-ci.md#workflow-hygiene`.
Fixture: `tools/ci/tests/fixtures/action_execution_cache/` via
`bazel run //tools/ci:action_execution_cache_qualification`.
"""

# Dx quality actions stay local-only until remote is qualified: every
# pipeline plus evaluator action carries no-remote-exec via the single
# quality/execution_requirements.bzl helper, never a bare
# remote-executable shape and never a cache-disabling marker. The
# cli/bep/src/remote.rs interface (RemoteConfig plus LocalDownloader)
# stays the local-only downloader seam.
EXECUTION_REQUIREMENTS = '{"no-remote-exec": "1"}'
EXECUTION_HELPER = "quality/execution_requirements.bzl"
EXECUTION_HELPER_FN = "dx_execution_requirements()"
EXECUTION_SYNTHETIC_SITES = 2
EXECUTION_REAL_SITES = 1
EXECUTION_FORBIDDEN = '"no-remote": "1"'
REMOTE_INTERFACE = "cli/bep/src/remote.rs"
REMOTE_CONFIG = "RemoteConfig"
REMOTE_DOWNLOADER = "LocalDownloader"

# Shared remote cache: BuildBuddy through the workspace `.bazelrc`
# `ci` config, passed inline on every Bazel plus dx call as
# `$BAZEL_CONFIG $BB_ARGS` (dx spawns Bazel with --nohome_rc, so the
# home rc never applies). PRs upload nothing via the `ci-pr` config
# (issue #1059), an absent secret sets no remote-cache flag at all, and
# the secretless dry run stays local. The legacy actions/cache disk
# cache is deleted:
# no restore-keys, no hashFiles keys, no per-host prefixes.
CACHE_REMOTE_URL = "common:ci --remote_cache=grpcs://remote.buildbuddy.io"
CACHE_API_KEY = "BB_ARGS=--remote_header=x-buildbuddy-api-key"
CACHE_PR_READ_ONLY = "common:ci-pr --noremote_upload_local_results"
CACHE_DELIVERY = "BAZEL_CONFIG"
CACHE_SECRET = "BUILDBUDDY_API_KEY"
CACHE_NO_DISK = "no actions/cache, no restore-keys, no hashFiles keys"

# Remote boundary: remote cache is wired, but remote execution plus BES
# stay absent; aquery plus execution-log evidence is local, remote
# execution stays unverified.
REMOTE_NO_EXEC = ["--remote_executor", "--bes_backend"]
REMOTE_ELSE_BRANCH = "locally sandbox-tested but remote behavior remains unverified"
