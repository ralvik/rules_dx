"""Update mutation-event wont-fix pins (issue #586).

Contract: `docs/cli/output-protocol.md#mutation`,
`docs/cli/commands/audit-update-bazel.md#dx-update`.
Fixture: `cli/update/tests/fixtures/update_events/` via
`bazel run //tools/ci:update_events_qualification`.

V1 update emits no `change` or `mutation` events in any mode: the five
authoritative backends provide no committed-change manifest equivalent
to the Gazelle result manifest, and Git scan / BUILD parse / rerun
inference is rejected because the protocol already forbids it. Per-set
`notice`/`error` plus `command_finished` is the complete event
contract. Update-only; no workflow change. Seed only: no Supported
claim.
"""

# Disposition: wont-fix for all five sets (no backend provides a manifest).
UPDATE_CHANGE_EVENTS = "wont-fix"
UPDATE_MUTATION_EVENTS = "wont-fix"

# Complete event contract: exactly one terminal per-set event per
# selected set in sorted set order, then exactly one command_finished.
UPDATE_PER_SET_SUCCESS = "update_set_success"
UPDATE_PER_SET_FAILURE = "update_failed"
UPDATE_PER_SET_BLOCKED = "update_set_blocked"
UPDATE_FINISHED_RESULTS_COMPLETE_LIVE = True

# Finished omits file-level counts in every mode.
UPDATE_FINISHED_OMITS_CHANGES = True
UPDATE_FINISHED_OMITS_MUTATIONS = True
UPDATE_FINISHED_OMITS_DIAGNOSTICS = True

# Rejected inference routes (protocol already forbids them).
REJECTED_GIT_SCAN = "git scan rejected"
REJECTED_BUILD_PARSE = "BUILD parse rejected"
REJECTED_RERUN = "rerun rejected"

# Interrupted-run completeness: preceding per-set events stay true with
# no rollback; nothing is emitted for sets not yet attempted; signal
# termination promises no command_finished.
INTERRUPTED_KEEPS_PRECEDING = True
INTERRUPTED_NO_ROLLBACK = True
INTERRUPTED_UNATTEMPTED_EMITS_NOTHING = True

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #586"
