#!/usr/bin/env bash
# CLI execution/reporting gaps qualification harness.
#
# Qualifies the four wont-fix contract matrices with fixture evidence:
# - watch: exactly 8 watchable (build/test/run/lint/typecheck/format/
#   check/fix) with 22 fail-closed not watchable plus CI refusal plus
#   200ms debounce plus verbatim per-iteration reuse plus one iteration
#   at a time;
# - arg-forwarding: Bazel startup options plus test-binary args rejected
#   on workflow commands with `dx bazel` guidance, `dx bazel` forwards
#   unchanged, `dx run` forwards after `--` to the application binary;
# - report matrix: lint/typecheck/check/fix sarif, test junit, coverage
#   lcov, audit sarif+spdx, every other command none including format,
#   unsupported combos fail with UnsupportedFormat, never silently
#   substituted;
# - parallelism: check/fix sequential phases, watch one iteration, update
#   sequential per-set with continuation, run sequential multirun, no
#   parallel/caching/scheduling/daemon;
# - fixtures: `cli/cli/tests/fixtures/cli_execution_gaps/` (`pins.bzl`
#   plus `cli_execution_gaps.expected`) pins dispositions plus rejected
#   routes plus honesty; unit fixtures live in `dx_adopt::watch`,
#   `dx_process`, `dx_cli::plan_reports`, `dx_update::outcome`;
# - scope: CLI-only; no Bazel semantics change.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:cli_execution_gaps_qualification`,
# following //tools/ci:starlark_futures_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

contract="docs/cli/cli-contract.md"
watch_doc="docs/cli/commands/watch.md"
protocol="docs/cli/output-protocol.md"
reports_doc="docs/cli/standard-reports.md"
testing="docs/testing/cli.md"
pins="cli/cli/tests/fixtures/cli_execution_gaps/pins.bzl"
expected="cli/cli/tests/fixtures/cli_execution_gaps/cli_execution_gaps.expected"
fixture_build="cli/cli/tests/fixtures/cli_execution_gaps/BUILD.bazel"
adopt_watch="cli/adopt/src/watch.rs"
process_lib="cli/process/src/lib.rs"
planning="cli/cli/src/reports/planning.rs"
outcome="cli/update/src/outcome.rs"
reports_facade="cli/cli/src/reports.rs"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Contract owns the pinned record with fixtures plus qualification.
if grep -q -F -e 'pinned under issue #590' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:cli_execution_gaps_qualification' "$contract" &&
  grep -q -F -e 'cli/cli/tests/fixtures/cli_execution_gaps/' "$contract" &&
  grep -q -F -e 'silent substitution across commands stays rejected' "$contract"; then
  ok
else
  bad "cli-contract.md lost its #590 execution/reporting gaps record with fixtures plus qualification"
fi

# Contract pins the watch, forwarding, report, and parallelism matrices.
if grep -q -F -e 'Watch stays 8 watchable' "$contract" &&
  grep -q -F -e 'Arg-forwarding stays wont-fix' "$contract" &&
  grep -q -F -e 'Report matrix stays wont-fix' "$contract" &&
  grep -q -F -e 'Parallelism stays wont-fix' "$contract"; then
  ok
else
  bad "cli-contract.md lost its #590 four-matrix wont-fix pins"
fi

# Watch doc owns the execution-gaps section with 8 plus 22 plus CI plus sequential.
if grep -q -F -e '## Execution Gaps' "$watch_doc" &&
  grep -q -F -e 'Decided under issue #590' "$watch_doc" &&
  grep -q -F -e 'Watchable stays exactly' "$watch_doc" &&
  grep -q -F -e '22 commands' "$watch_doc" &&
  grep -q -F -e 'CI refusal stays wont-fix' "$watch_doc" &&
  grep -q -F -e 'one iteration at a time' "$watch_doc"; then
  ok
else
  bad "watch.md lost its #590 execution-gaps section with 8 plus 22 plus CI plus sequential"
fi

# Cli-contract owns the compatibility bullet with matrix plus sequential
# ordering; output-protocol keeps the no-parallel-execution boundary.
if grep -q -F -e 'issue #590' "$contract" &&
  grep -q -F -e 'cli/cli/tests/fixtures/cli_execution_gaps/' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:cli_execution_gaps_qualification' "$contract" &&
  grep -q -F -e 'UnsupportedFormat' "$contract" &&
  grep -q -F -e 'never silently substituted' "$contract" &&
  grep -q -F -e '`update` runs per-set sequentially' "$contract" &&
  grep -q -F -e 'parallel execution' "$protocol"; then
  ok
else
  bad "cli-contract lost its #590 compatibility bullet with matrix plus sequential ordering"
fi

# Standard-reports owns the wont-fix matrix table with check/fix plus SPDX plus format-none.
if grep -q -F -e 'wont-fix matrix, issue #590' "$reports_doc" &&
  grep -q -F -e 'cli/cli/tests/fixtures/cli_execution_gaps/' "$reports_doc" &&
  grep -q -F -e '`check`, `fix`' "$reports_doc" &&
  grep -q -F -e 'have no initial standard report (issue #590 wont-fix' "$reports_doc" &&
  grep -q -F -e 'silent substitution' "$reports_doc"; then
  ok
else
  bad "standard-reports.md lost its #590 wont-fix matrix table with check/fix plus format-none"
fi

# Fixture pins stay present with watch plus forwarding plus reports plus parallelism plus honesty.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'WATCHABLE_COMMANDS = [' "$pins" &&
  grep -q -F -e 'NOT_WATCHABLE_COMMANDS = [' "$pins" &&
  grep -q -F -e 'WATCH_REFUSES_CI = True' "$pins" &&
  grep -q -F -e 'STARTUP_OPTIONS_REJECTED = [' "$pins" &&
  grep -q -F -e 'TEST_BINARY_ARGS_REJECTED = ["test_arg"]' "$pins" &&
  grep -q -F -e 'REPORT_AUDIT = ["sarif", "spdx"]' "$pins" &&
  grep -q -F -e 'PARALLEL_DISPOSITION = "wont-fix"' "$pins" &&
  grep -q -F -e 'REJECTED_SILENT_SUBSTITUTION' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #590' "$pins"; then
  ok
else
  bad "cli_execution_gaps pins fixture lost its watch plus forwarding plus reports plus parallelism wiring under #590"
fi

# Expected fixture pins the four gaps plus rejected plus honesty lines.
if grep -q -F -e 'watch stays 8 watchable' "$expected" &&
  grep -q -F -e 'startup options rejected' "$expected" &&
  grep -q -F -e 'test-binary args rejected' "$expected" &&
  grep -q -F -e 'report matrix wont-fix' "$expected" &&
  grep -q -F -e 'parallelism wont-fix' "$expected" &&
  grep -q -F -e 'silent substitution rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #590' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "cli_execution_gaps.expected lost its four gaps plus rejected plus honesty lines under #590"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'cli_execution_gaps.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "cli_execution_gaps BUILD.bazel lost its pins plus expected exports with corpus under #590"
fi

# Adopt watch keeps the full 8 plus 22 matrix with CI refusal under
# (bare issue refs live in docs, not non-test source).
if grep -q -F -e 'watch_execution_gaps_matrix_is_wont_fix' "$adopt_watch" &&
  grep -q -F -e 'WATCHABLE_COMMANDS.len(), 8' "$adopt_watch" &&
  grep -q -F -e 'WATCH_DEBOUNCE_MS, 200' "$adopt_watch" &&
  grep -q -F -e '"docs",' "$adopt_watch" &&
  grep -q -F -e 'issue #590' "$watch_doc"; then
  ok
else
  bad "adopt/watch.rs plus watch.md lost the #590 8-plus-22 watch matrix fixture"
fi

# Process keeps the startup plus test-binary forwarding matrix under
# (the wont-fix test lives in lib_tests; bare issue refs live in docs).
if grep -q -F -e 'execution_gaps_forwarding_matrix_is_wont_fix' "cli/process/src/lib_tests.rs" &&
  grep -q -F -e 'is_startup_option' "$process_lib" &&
  grep -q -F -e 'is_test_binary_arg' "$process_lib" &&
  grep -q -F -e 'build_bazel_passthrough' "$process_lib" &&
  grep -q -F -e 'issue #590' "$contract"; then
  ok
else
  bad "process lib plus tests lost the #590 startup plus test-binary forwarding fixture"
fi

# Planning keeps the per-command report matrix under.
if grep -q -F -e 'execution_gaps_report_matrix_is_wont_fix' "$planning" &&
  grep -q -F -e 'spec(Command::Format).reports.is_empty()' "$planning" &&
  grep -q -F -e 'issue #590' "$contract"; then
  ok
else
  bad "reports/planning.rs lost the #590 per-command report matrix fixture"
fi

# Outcome keeps the sequential parallelism pin under.
if grep -q -F -e 'execution_gaps_parallelism_stays_sequential' "$outcome" &&
  grep -q -F -e 'wont-fix, sequential per-set' "$outcome" &&
  grep -q -F -e 'issue #590' "$contract"; then
  ok
else
  bad "update/outcome.rs lost the #590 sequential parallelism fixture"
fi

# Reports facade plus standard-reports own the wont-fix UnsupportedFormat record under.
if grep -q -F -e 'issue #590' "$reports_doc" &&
  grep -q -F -e 'never silently' "$reports_facade" &&
  grep -q -F -e 'UnsupportedFormat' "$reports_facade"; then
  ok
else
  bad "reports.rs plus standard-reports lost the #590 wont-fix UnsupportedFormat record"
fi

# Testing matrix pins watch plus forwarding plus reports plus parallelism under.
if grep -q -F -e 'bazel run //tools/ci:cli_execution_gaps_qualification' "$testing" &&
  grep -q -F -e 'issue #590' "$testing" &&
  grep -q -F -e 'dx_adopt::plan_watch' "$testing" &&
  grep -q -F -e 'dx_process::build_workflow_argv' "$testing" &&
  grep -q -F -e 'dx_cli::plan_reports' "$testing" &&
  grep -q -F -e 'dx_update::aggregate' "$testing"; then
  ok
else
  bad "testing/cli.md lost its #590 watch plus forwarding plus reports plus parallelism pins"
fi

# ci_targets_c.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "cli_execution_gaps_qualification"' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'cli_execution_gaps_qualification.sh' "tools/ci/ci_targets_c.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cli_execution_gaps_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_c.bzl or dogfood_freshness.sh lost the cli_execution_gaps_qualification wiring (want target plus dogfood-freshness)"
fi

dx_test_summary "cli execution gaps qualification harness"
