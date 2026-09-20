"""Selective NuGet `dx update` per-package pins.

Contract: `docs/decisions/0024-selective-update.md`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/selective_nuget/` via
`bazel run //tools/ci:selective_nuget_qualification`.
"""

# Disposition: per-package selective wont-fix in V1 (whole-folder regen only).
SELECTIVE_NUGET = "wont-fix"

# Approved full operation (argv owned by `dx_update::backend`).
SELECTIVE_NUGET_FULL = "bazel run @rules_dotnet//tools/paket2bazel -- --dependencies-file third_party/dotnet/paket.dependencies --output-folder third_party/dotnet/deps"

# Per-package selector shape (parses, then fails closed at execution).
SELECTIVE_NUGET_SELECTOR = "nuget:FSharp.Core"
SELECTIVE_NUGET_SELECTOR_LABEL = "nuget:<id>"

# Fail-closed hint (selective never widens to full silently).
SELECTIVE_NUGET_HINT = "use `dx update nuget` for the set"

# Bump follow-up chains automatically resolver-owned (issue #638).
BUMP_FOLLOWUP_NUGET = "dx update nuget"

# Rejected routes (never pinned as supported here).
REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_PAKET_LOCK_SURGERY = "private paket.lock surgery rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #635"
