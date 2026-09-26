"""Selective Cargo `dx update` per-crate pins.

Contract: `docs/decisions/0024-selective-update.md`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/selective_cargo/` via
`bazel run //tools/ci:selective_cargo_qualification`.
"""

# Disposition: per-crate selective wont-fix in V1 (whole-lock repin only).
SELECTIVE_CARGO = "wont-fix"

# Approved full operation (argv owned by `dx_update::backend`).
SELECTIVE_CARGO_FULL = "CARGO_BAZEL_REPIN=1 bazel build //rust/tests/fixtures/hello:hello"

# Per-crate selector shape (parses, then fails closed at execution).
SELECTIVE_CARGO_SELECTOR = "cargo:anyhow"
SELECTIVE_CARGO_SELECTOR_LABEL = "cargo:<crate>"

# Fail-closed hint (selective never widens to full silently).
SELECTIVE_CARGO_HINT = "use `dx update cargo` for the set"

BUMP_FOLLOWUP_CARGO = "dx update cargo"

# Rejected routes (never pinned as supported here).
REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_CARGO_UPDATE_P = "private cargo update -p rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #633"
