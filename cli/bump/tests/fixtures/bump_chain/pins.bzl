"""Bump then update chaining pins.

Contract: `docs/cli/commands/audit-update-bazel.md#dx-bump`,
`docs/decisions/0024-selective-update.md`.
Fixture: `cli/bump/tests/fixtures/bump_chain/` via
`bazel run //tools/ci:bump_chain_qualification`.
"""

BUMP_CHAIN = "automatic"

# Per-set chaining (widen then refresh, never a manual second step).
BUMP_CHAIN_CARGO = "dx update cargo"
BUMP_CHAIN_CARGO_MODE = "full (CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello)"
BUMP_CHAIN_NPM = "dx update npm:<pkg>"
BUMP_CHAIN_NPM_MODE = "selective (bazel run @pnpm//:pnpm -- update [<pkg>])"
BUMP_CHAIN_GO = "dx update go"
BUMP_CHAIN_GO_MODE = "noop (pinned module lock tracks Gazelle; no launch)"
BUMP_CHAIN_MAVEN = "dx update maven"
BUMP_CHAIN_MAVEN_MODE = "full (REPIN=1 bazel run @maven//:pin)"
BUMP_CHAIN_NUGET = "dx update nuget"
BUMP_CHAIN_NUGET_MODE = "full (bazel run @rules_dotnet//tools/paket2bazel)"

# Refresh selector shapes (planned in `dx_bump::BumpRequest::refresh_selector`).
BUMP_CHAIN_CARGO_SELECTOR = "cargo:anyhow refreshes cargo"
BUMP_CHAIN_NPM_SELECTOR = "npm:jest refreshes npm:jest"
BUMP_CHAIN_GO_SELECTOR = "go:example.com/mod refreshes go"
BUMP_CHAIN_MAVEN_SELECTOR = "maven:junit:junit refreshes maven"
BUMP_CHAIN_NUGET_SELECTOR = "nuget:FSharp.Core refreshes nuget"

BUMP_CHAIN_BAZEL = "file-only (preset flag-diff review plus bazel build //...)"
BUMP_CHAIN_GHA = "file-only (preset flag-diff review plus bazel build //...)"

# Execution owns the chain through the approved backends (never private).
BUMP_CHAIN_BACKEND = "dx_update::backend::plan"
BUMP_CHAIN_RUNNER = "runner.run"

BUMP_CHAIN_FAILURE = "widen kept in manifest"
BUMP_CHAIN_FAILURE_CODE = "update_failed"

# missing-manifest hint with exit mapping; Git shapes never hint.
BUMP_MAJOR_HINT = "major bump"
BUMP_MAJOR_MIGRATE = "dx migrate --from <old> --to <new>"
BUMP_MAJOR_MANIFEST = "migrate-v1-to-v2.json"
BUMP_MAJOR_LIVE_CODE = "migrate_failed"
BUMP_MAJOR_USAGE_CODE = "missing-versions"

# Rejected routes (never pinned as supported here).
REJECTED_MANUAL_SECOND_STEP = "manual second step rejected"
REJECTED_PRIVATE_RESOLVER = "private resolver rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #638"
