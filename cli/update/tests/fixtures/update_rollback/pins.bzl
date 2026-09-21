"""Atomic `dx update` rollback-plan pins.

Contract: `docs/cli/commands/audit-update-bazel.md#dx-update`,
`docs/cli/output-protocol.md#mutation`.
Fixture: `cli/update/tests/fixtures/update_rollback/` via
`bazel run //tools/ci:update_rollback_qualification`.
"""

# Atomicity boundary: per-set commit, never repository-wide.
ATOMICITY_BOUNDARY = "per-set commit"
REPO_WIDE_ATOMIC = False
AUTOMATIC_ROLLBACK = False

# Recovery plan: manual restore plus idempotent retry (dx never runs Git).
RETRY_COMMAND = "dx update <failed/blocked sets>"
RESTORE_COMMAND = "git checkout -- <kept locks>"
RETRY_IDEMPOTENT = True
DX_RUNS_GIT_RESTORE = False

# Reporting: failure/interrupted runs carry the recovery hint, never
# silent partial success.
RECOVERY_CODE = "update_recovery"
RECOVERY_TEXT_PREFIX = "dx: update_recovery:"
RECOVERY_JSON_CODE = "update_recovery"
NO_SILENT_PARTIAL_SUCCESS = True

# Interrupted multi-set run: preceding per-set events stay true,
# unattempted sets emit nothing, signal promises no command_finished.
INTERRUPTED_KEEPS_PRECEDING = True
INTERRUPTED_UNATTEMPTED_EMITS_NOTHING = True
INTERRUPTED_NO_COMMAND_FINISHED_ON_SIGNAL = True
INTERRUPTED_RETRY = "dx update <unattempted sets>"

# Rejected routes (no manifest, no inference, no private rollback).
REJECTED_AUTOMATIC_ROLLBACK = "automatic rollback rejected (no backend committed-change manifest)"
REJECTED_GIT_SCAN = "git scan rejected"
REJECTED_PRIVATE_ROLLBACK = "private lockfile surgery rejected"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #772"
