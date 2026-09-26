"""Selective Go `dx update` per-module pins.

Contract: `docs/decisions/0024-selective-update.md`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/selective_go/` via
`bazel run //tools/ci:selective_go_qualification`.
"""

# Disposition: full is intentional no-op, selective is wont-fix in V1.
SELECTIVE_GO = "wont-fix"
GO_FULL = "noop"

# Approved full operation (no launch; pinned module lock tracks Gazelle).
GO_FULL_LOCK = "third_party/go/go.mod plus go.sum via go_deps.from_file tracks Gazelle; no launch"
GO_FROM_FILE = "go_deps.from_file"
GO_MOD = "third_party/go/go.mod"
GO_SUM = "third_party/go/go.sum"

# Per-module selector shape (parses, then fails closed at execution).
SELECTIVE_GO_SELECTOR = "go:github.com/google/go-cmp/cmp"
SELECTIVE_GO_SELECTOR_LABEL = "go:<module-path>"

# Fail-closed hint (selective never widens to full silently).
SELECTIVE_GO_HINT = "widen explicitly via `dx bump gomod:<module> <version>`"

BUMP_FOLLOWUP_GO = "dx update go"

# Rejected routes (never pinned as supported here).
REJECTED_SILENT_FULL_SUBSTITUTION = "silent full-update substitution rejected"
REJECTED_PRIVATE_GO_GET = "private go get rejected"
REJECTED_PRIVATE_GO_MOD_TIDY = "private go mod tidy rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #636"
