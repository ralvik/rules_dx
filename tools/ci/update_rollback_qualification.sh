#!/usr/bin/env bash
# Atomic `dx update` rollback-plan qualification harness (issue #772).
#
# Qualifies the per-set commit boundary plus manual recovery plan:
# - boundary: each backend success commits its set immediately;
#   repository-wide is never atomic and successes are never rolled back
#   automatically (no backend committed-change manifest; Git scan, BUILD
#   parse, rerun, and private lockfile surgery stay rejected);
# - recovery: idempotent retry (`dx update <failed/blocked sets>`,
#   interrupted runs retry `<unattempted sets>`) plus manual restore
#   (`git checkout -- <kept locks>`, run by the operator; `dx` never
#   runs Git), planned in `dx_update::recovery`;
# - reporting: failed runs carry an `update_recovery` warning notice
#   (JSON) plus a `dx: update_recovery:` line (text stderr and JSON
#   stderr), so partial runs never read as silent success;
# - fixtures: `cli/update/tests/fixtures/update_rollback/` (`pins.bzl`
#   plus `update_rollback.expected`) pins the boundary, retry/restore
#   commands, reporting codes, interrupted evidence, and rejected routes;
# - scope: update-only, no resolver change, no lock format change.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:update_rollback_qualification`,
# following //tools/ci:update_events_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

recovery="cli/update/src/recovery.rs"
lib="cli/update/src/lib.rs"
outcome="cli/update/src/outcome.rs"
protocol="docs/cli/output-protocol.md"
command_doc="docs/cli/commands/audit-update-bazel.md"
pins="cli/update/tests/fixtures/update_rollback/pins.bzl"
expected="cli/update/tests/fixtures/update_rollback/update_rollback.expected"
fixture_build="cli/update/tests/fixtures/update_rollback/BUILD.bazel"
exec_update="cli/cli/src/exec/update.rs"
exec_tests_b="cli/cli/src/exec/update_tests_b.rs"
targets="tools/ci/ci_targets_c.bzl"
dogfood="tools/ci/dogfood_freshness.sh"
verify="docs/testing/verification-matrix.md"
verify_remaining="docs/testing/verification-matrix-remaining.md"

# Recovery module owns the per-set boundary with no automatic rollback.
if grep -q -F -e 'per-set commit' "$recovery" &&
  grep -q -F -e 'never repository-wide' "$recovery" &&
  grep -q -F -e 'AUTOMATIC_ROLLBACK' "$recovery" &&
  grep -q -F -e 'RECOVERY_CODE' "$recovery"; then
  ok
else
  bad "recovery.rs lost its per-set commit boundary with no automatic rollback"
fi

# Recovery plans retry plus restore with idempotent commands.
if grep -q -F -e 'pub fn plan(' "$recovery" &&
  grep -q -F -e 'pub fn plan_interrupted(' "$recovery" &&
  grep -q -F -e 'pub fn retry_command(' "$recovery" &&
  grep -q -F -e 'pub fn restore_command(' "$recovery" &&
  grep -q -F -e 'idempotent' "$recovery"; then
  ok
else
  bad "recovery.rs lost its retry-plus-restore planning with idempotent commands"
fi

# Recovery unit tests pin failed, interrupted, and clean runs.
if grep -q -F -e 'failed_report_plans_retry_plus_restore' "$recovery" &&
  grep -q -F -e 'interrupted_run_retries_unattempted_and_restores_kept' "$recovery" &&
  grep -q -F -e 'success_needs_no_recovery' "$recovery"; then
  ok
else
  bad "recovery.rs lost its failed plus interrupted plus clean unit tests"
fi

# Crate root exposes the recovery module.
if grep -q -F -e 'pub mod recovery;' "$lib" &&
  grep -q -F -e '[`recovery`]' "$lib"; then
  ok
else
  bad "cli/update/src/lib.rs lost its recovery module wiring"
fi

# Outcome keeps the per-set boundary with recovery ownership.
if grep -q -F -e 'not a repository-wide' "$outcome" &&
  grep -q -F -e 'super::recovery' "$outcome"; then
  ok
else
  bad "outcome.rs lost its per-set boundary with recovery ownership"
fi

# Live execution emits the recovery notice plus stderr hint on failure.
if grep -q -F -e 'dx_update::recovery::plan(&report)' "$exec_update" &&
  grep -q -F -e 'dx_update::recovery::RECOVERY_CODE' "$exec_update" &&
  grep -q -F -e 'update_recovery' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its update_recovery notice plus stderr hint"
fi

# Execution tests pin recovery in text and JSON with clean-run silence.
if grep -q -F -e 'update_failure_reports_recovery_in_text_and_json' "$exec_tests_b" &&
  grep -q -F -e 'update_success_emits_no_recovery' "$exec_tests_b" &&
  grep -q -F -e 'update_json_completeness_is_per_set_plus_finished' "$exec_tests_b"; then
  ok
else
  bad "exec/update_tests_b.rs lost its recovery text plus JSON plus clean-run tests"
fi

# Fixture pins stay present with boundary plus retry plus restore.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'ATOMICITY_BOUNDARY = "per-set commit"' "$pins" &&
  grep -q -F -e 'RETRY_COMMAND = "dx update <failed/blocked sets>"' "$pins" &&
  grep -q -F -e 'RESTORE_COMMAND = "git checkout -- <kept locks>"' "$pins" &&
  grep -q -F -e 'RECOVERY_CODE = "update_recovery"' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #772' "$pins"; then
  ok
else
  bad "update_rollback pins fixture lost its boundary plus retry plus restore wiring under #772"
fi

# Expected fixture pins the boundary plus interrupted plus honesty lines.
if grep -q -F -e 'atomicity boundary is per-set commit' "$expected" &&
  grep -q -F -e 'no automatic rollback' "$expected" &&
  grep -q -F -e 'retry is idempotent' "$expected" &&
  grep -q -F -e 'git checkout --' "$expected" &&
  grep -q -F -e 'interrupted keeps preceding per-set events true' "$expected" &&
  grep -q -F -e 'update_recovery notice' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #772' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "update_rollback.expected lost its boundary plus interrupted plus honesty lines under #772"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'update_rollback.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "update_rollback BUILD.bazel lost its pins plus expected exports with corpus under #772"
fi

# Command docs carry the per-set boundary with retry plus restore.
if grep -q -F -e 'Atomicity is per set, never repository-wide' "$command_doc" &&
  grep -q -F -e 'update_recovery' "$command_doc" &&
  grep -q -F -e 'git checkout --' "$command_doc" &&
  grep -q -F -e 'dx_update::recovery' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/update_rollback/' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its per-set boundary with retry plus restore under #772"
fi

# Protocol owns the boundary plus recovery notice plus interrupted retry.
if grep -q -F -e 'Atomicity is per set, never repository-wide' "$protocol" &&
  grep -q -F -e 'update_recovery' "$protocol" &&
  grep -q -F -e 'idempotent retry' "$protocol" &&
  grep -q -F -e 'dx: update_recovery:' "$protocol" &&
  grep -q -F -e 'cli/update/tests/fixtures/update_rollback/' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its per-set boundary with recovery notice under #772"
fi

# Protocol notice codes name update_recovery next to per-set codes.
if grep -q -F -e '`update_recovery`' "$protocol" &&
  grep -q -F -e '`update_set_success`' "$protocol" &&
  grep -q -F -e '`update_set_blocked`' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its update_recovery notice-code record under #772"
fi

# Protocol Compatibility Tests require the rollback fixtures.
if grep -q -F -e 'cli/update/tests/fixtures/update_rollback/' "$protocol" &&
  grep -q -F -e 'dx_update::recovery' "$protocol"; then
  ok
else
  bad "output-protocol.md Compatibility Tests lost the update_rollback fixture requirement under #772"
fi

# Targets own the harness plus dogfood wires it.
if grep -q -F -e 'name = "update_rollback_qualification"' "$targets" &&
  grep -q -F -e 'update_rollback_qualification.sh' "$targets" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:update_rollback_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci targets or dogfood lost the update_rollback_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':update_rollback_qualification' "$verify" &&
  grep -q -F -e 'issue #772' "$verify" &&
  grep -q -F -e ':update_rollback_qualification' "$verify_remaining" &&
  grep -q -F -e 'issue #772' "$verify_remaining"; then
  ok
else
  bad "verification-matrix.md lost its update_rollback_qualification entry under #772"
fi

dx_test_summary "update rollback qualification harness"
