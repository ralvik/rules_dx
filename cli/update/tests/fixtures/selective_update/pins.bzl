"""Selective `dx update` per-set pins.

Contract: `docs/decisions/0024-selective-update.md`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/selective_update/` via
`bazel run //tools/ci:selective_update_qualification`.
"""

# Per-set selective dispositions (one supported, four wont-fix).
SELECTIVE_NPM = "supported"
SELECTIVE_CARGO = "wont-fix"
SELECTIVE_MAVEN = "wont-fix"
SELECTIVE_NUGET = "wont-fix"
SELECTIVE_GO = "wont-fix"

# Approved upstream operations (argv owned by `dx_update::backend`).
SELECTIVE_NPM_ARGV = "bazel run @pnpm//:pnpm -- update [<pkg>...]"
SELECTIVE_CARGO_FULL = "CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello"
SELECTIVE_MAVEN_FULL = "REPIN=1 bazel run @maven//:pin"
SELECTIVE_NUGET_FULL = "bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file third_party/dotnet/paket.dependencies --output-folder third_party/dotnet/deps"
SELECTIVE_GO_FULL = "noop (pinned module lock tracks Gazelle)"

# Fail-closed hints (selective never widens to full silently).
SELECTIVE_CARGO_HINT = "use `dx update cargo` for the set"
SELECTIVE_MAVEN_HINT = "use `dx update maven` for the set"
SELECTIVE_NUGET_HINT = "use `dx update nuget` for the set"
SELECTIVE_GO_HINT = "go pins track Gazelle for the shared go_deps extension; widen explicitly via `dx bump gomod:<module> <version>`"

# Bump follow-up chains automatically resolver-owned (issue #638).
BUMP_FOLLOWUP_CARGO = "dx update cargo"
BUMP_FOLLOWUP_NPM = "dx update npm:<pkg> or dx update npm"
BUMP_FOLLOWUP_GO = "dx update go"

# Rejected and honesty lines (never pinned as supported here).
REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_EMULATION = "private per-package emulation rejected"
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #583"
