#!/usr/bin/env bash
# Update mutation-event wont-fix qualification harness.
#
# Qualifies the explicit wont-fix plus event-completeness contract:
# - wont-fix: update emits no v1 `change` or `mutation` events and no
#   `changes`/`mutations`/`diagnostics` counts in any mode because the
#   five authoritative backends provide no committed-change manifest
#   equivalent to the Gazelle result manifest, and Git scan / BUILD
#   parse / rerun inference is rejected because the protocol already
#   forbids it;
# - contract: per-set `notice`/`error` plus `command_finished` is the
#   complete event set (exactly one terminal per-set event per selected
#   set in sorted order, then exactly one `command_finished`, with
#   `results_complete=true` on live terminal reports);
# - interrupted: preceding per-set events stay true with no rollback,
#   unattempted sets emit nothing and are never inferred, signal
#   termination promises no `command_finished`;
# - fixtures: `cli/update/tests/fixtures/update_events/` (`pins.bzl`
#   plus `update_events.expected`) pins dispositions plus rejected
#   routes plus honesty; unit fixtures live in
#   `cli/cli/src/exec/update.rs`;
# - scope: protocol-only; no workflow change until a manifest lands.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:update_events_qualification`,
# following //tools/ci:selective_update_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

protocol="docs/cli/output-protocol.md"
command_doc="docs/cli/commands/audit-update-bazel.md"
pins="cli/update/tests/fixtures/update_events/pins.bzl"
expected="cli/update/tests/fixtures/update_events/update_events.expected"
fixture_build="cli/update/tests/fixtures/update_events/BUILD.bazel"
exec_update="cli/cli/src/exec/update.rs"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Protocol owns the wont-fix under with resolver-owned backends.
if grep -q -F -e 'wont-fix, issue #586' "$protocol" &&
  grep -q -F -e 'resolver-owned by' "$protocol" &&
  grep -q -F -e '`dx_update::backend`' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its #586 wont-fix with resolver-owned backend record"
fi

# Protocol says update emits no change/mutation events and no file counts.
if grep -q -F -e 'Update emits no v1 `change` or `mutation` events' "$protocol" &&
  grep -q -F -e 'carries no' "$protocol" &&
  grep -q -F -e '`changes`, `mutations`, or `diagnostics` counts' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its no-change-no-mutation-no-counts wont-fix"
fi

# Protocol names the completeness contract (per-set plus finished).
if grep -q -F -e 'Per-set' "$protocol" &&
  grep -q -F -e 'are the complete update event contract' "$protocol" &&
  grep -q -F -e 'one terminal per-set event' "$protocol" &&
  grep -q -F -e 'update_set_success' "$protocol" &&
  grep -q -F -e 'update_set_blocked' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its per-set plus finished completeness contract"
fi

# Protocol pins live results_complete with dry-run/check omission.
if grep -q -F -e 'For update it reports whether every' "$protocol" &&
  grep -q -F -e 'dry-run, `--check`,' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its update results_complete live-vs-omitted pin"
fi

# Protocol rejects Git scan, BUILD parse, and rerun inference.
if grep -q -F -e 'Git scan, BUILD parse, or rerun is rejected' "$protocol" &&
  grep -q -F -e 'the protocol already forbids it' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its rejected Git-scan BUILD-parse rerun record"
fi

# Protocol pins interrupted-run completeness (keeps preceding, no rollback, unattempted emits nothing, signal promises nothing).
if grep -q -F -e 'Interrupted-run completeness follows the same contract' "$protocol" &&
  grep -q -F -e 'leaves their preceding per-set events true with' "$protocol" &&
  grep -q -F -e 'no rollback' "$protocol" &&
  grep -q -F -e 'must not' "$protocol" &&
  grep -q -F -e 'No `command_finished` is promised after signal' "$protocol"; then
  ok
else
  bad "output-protocol.md lost its interrupted-run completeness pin"
fi

# Protocol Compatibility Tests require the update_events fixtures.
if grep -q -F -e 'cli/update/tests/fixtures/update_events/' "$protocol" &&
  grep -q -F -e 'issue #586' "$protocol" &&
  grep -q -F -e 'interrupted runs keeping preceding per-set events true' "$protocol"; then
  ok
else
  bad "output-protocol.md Compatibility Tests lost the update_events fixture requirement under #586"
fi

# Command docs carry the wont-fix cross-link with the fixture paths.
if grep -q -F -e 'wont-fix, issue #586' "$command_doc" &&
  grep -q -F -e 'cli/update/tests/fixtures/update_events/' "$command_doc" &&
  grep -q -F -e 'cli/cli/src/exec/update.rs' "$command_doc" &&
  grep -q -F -e '[Output Protocol](../output-protocol.md#mutation)' "$command_doc"; then
  ok
else
  bad "audit-update-bazel.md lost its #586 wont-fix cross-link with fixtures"
fi

# Fixture pins stay present with wont-fix plus contract plus rejected plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'UPDATE_MUTATION_EVENTS = "wont-fix"' "$pins" &&
  grep -q -F -e 'UPDATE_CHANGE_EVENTS = "wont-fix"' "$pins" &&
  grep -q -F -e 'UPDATE_PER_SET_SUCCESS = "update_set_success"' "$pins" &&
  grep -q -F -e 'UPDATE_PER_SET_BLOCKED = "update_set_blocked"' "$pins" &&
  grep -q -F -e 'REJECTED_GIT_SCAN' "$pins" &&
  grep -q -F -e 'REJECTED_BUILD_PARSE' "$pins" &&
  grep -q -F -e 'REJECTED_RERUN' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #586' "$pins"; then
  ok
else
  bad "update_events pins fixture lost its wont-fix plus contract plus rejected wiring under #586"
fi

# Expected fixture pins the wont-fix plus contract plus interrupted plus honesty lines.
if grep -q -F -e 'update mutation events wont-fix' "$expected" &&
  grep -q -F -e 'per-set notice/error plus command_finished is the complete contract' "$expected" &&
  grep -q -F -e 'finished omits changes/mutations/diagnostics' "$expected" &&
  grep -q -F -e 'git scan rejected' "$expected" &&
  grep -q -F -e 'BUILD parse rejected' "$expected" &&
  grep -q -F -e 'rerun rejected' "$expected" &&
  grep -q -F -e 'interrupted keeps preceding per-set events true' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #586' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "update_events.expected lost its wont-fix plus contract plus honesty lines under #586"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'update_events.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "update_events BUILD.bazel lost its pins plus expected exports with corpus under #586"
fi

# Execution pins the wont-fix with three dedicated JSON tests.
if grep -q -F -e 'update_json_never_emits_change_or_mutation' "$exec_update" &&
  grep -q -F -e 'update_json_completeness_is_per_set_plus_finished' "$exec_update" &&
  grep -q -F -e 'update_json_check_and_dryrun_emit_no_file_events_or_counts' "$exec_update" &&
  grep -q -F -e 'Issue #586' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its three #586 wont-fix JSON fixtures"
fi

# Execution never builds change/mutation events on the update path.
if ! grep -q -F -e 'change_event' "$exec_update" &&
  ! grep -q -F -e 'mutation_event' "$exec_update"; then
  ok
else
  bad "exec/update.rs must not build change/mutation events on the update path under #586"
fi

# Live execution keeps results_complete with no file counts on finished.
if grep -q -F -e 'results_complete: Some(true)' "$exec_update" &&
  grep -q -F -e '..FinishedCounts::default()' "$exec_update"; then
  ok
else
  bad "exec/update.rs lost its results_complete-only finished wiring under #586"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "update_events_qualification"' "$build" &&
  grep -q -F -e 'update_events_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:update_events_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the update_events_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':update_events_qualification' "$verify" &&
  grep -q -F -e 'issue #586' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its update_events_qualification entry under #586"
fi

dx_test_summary "update events qualification harness"
