#!/usr/bin/env bash
# LCOV excludes-budget qualification harness (issue #1055).
#
# Blanket `policy:` excludes with no what/why stay bounded here, never
# unbounded: every source-level ignore needs a specific `reason:` plus
# `issue:` tracking (bare `policy:` without `reason:` plus `issue:` fails
# the `dx_lcov` gate), the count stays within the versioned budget, and
# the budget carries an expiry review date that fails closed after expiry.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_excludes_qualification`,
# following //tools/ci:coverage_cell.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

budget="tools/coverage/excludes-budget.txt"
strategy="docs/testing/strategy-details.md"
build="tools/ci/ci_targets_a.bzl"
prove="tools/ci/prove.sh"
dogfood="tools/ci/dogfood_freshness.sh"

# Budget file stays present with pinned ranges/lines/total plus expiry plus issue.
if [[ -f "$budget" ]] &&
  grep -q -F -e 'BUDGET_RANGES=60' "$budget" &&
  grep -q -F -e 'BUDGET_LINES=28' "$budget" &&
  grep -q -F -e 'BUDGET_TOTAL=148' "$budget" &&
  grep -q -F -e 'EXPIRES=2027-03-01' "$budget" &&
  grep -q -F -e 'ISSUE=1055' "$budget"; then
  ok
else
  bad "excludes budget missing or drifted (want $budget with 60 ranges plus 28 singles plus 148 total plus 2027-03-01 plus 1055, issue #1055)"
fi

# Expiry review fails closed after the review date (like audit exceptions).
expires="$(grep -E -e '^EXPIRES=' "$budget" | cut -d= -f2)"
today="$(date -u +%F)"
if [[ "$today" < "$expires" ]]; then
  ok
else
  bad "excludes budget expired $expires (today $today); renew review plus budget plus strategy-details in one reviewed PR, issue #1055"
fi

# Real-marker scope: tracked *.rs/*.go/*.sh markers excluding intentional
# fail-closed fixtures and pins constants (which name the directive but are
# not excludes) plus the gate's own split-literal tests (no contiguous marker).
# (hermetic tree search: host grep -rn variance, issue #1006).
dx_mkscratch excludes_scratch
git grep -n "LCOV_EXCL_" -- '*.rs' '*.go' '*.sh' 2>/dev/null |
  grep -v -F -e 'cli/cli/src/reports/lcov.rs' |
  grep -v -F -e 'tools/ci/coverage_cell.sh' |
  grep -v -F -e 'tools/ci/coverage_excludes_qualification.sh' |
  grep -v -F -e 'tools/ci/lcov_accounting_qualification.sh' |
  grep -v -F -e 'cc/tests/fixtures/lcov_accounting/pins.bzl' |
  grep -v -F -e 'cli/lcov/src/' >"$excludes_scratch/real.txt" || true
# The exec test-data bare string is inert for its own gate (inside a string
# literal) but names the directive; drop that one line, keep the real marker.
grep -v -F -e 'test_reports.rs:556' "$excludes_scratch/real.txt" >"$excludes_scratch/scoped.txt" || true
ranges="$(grep -c -F -e 'LCOV_EXCL_START' "$excludes_scratch/scoped.txt" || true)"
lines="$(grep -c -F -e 'LCOV_EXCL_LINE' "$excludes_scratch/scoped.txt" || true)"
total="$(wc -l <"$excludes_scratch/scoped.txt" | tr -d ' ')"
if [[ "$ranges" == "60" ]] && [[ "$lines" == "28" ]] && [[ "$total" == "148" ]]; then
  ok
else
  bad "excludes inventory drifted (want 60 ranges plus 28 singles plus 148 total, found $ranges plus $lines plus $total; update budget plus strategy-details in one reviewed PR, issue #1055)"
fi

# Budget caps growth: scoped counts stay within budget (fail-closed upper bound).
if [[ "$ranges" -le "60" ]] && [[ "$lines" -le "28" ]] && [[ "$total" -le "148" ]]; then
  ok
else
  bad "excludes exceeded budget (want <= 60 ranges plus <= 28 singles plus <= 148 total, found $ranges plus $lines plus $total, issue #1055)"
fi

# No bare policy: without reason: plus issue: in the scoped inventory.
# Every scoped marker must carry issue: tracking; bare policy: lines fail here.
bare_policy=""
while IFS= read -r entry; do
  [[ -z "$entry" ]] && continue
  file="${entry%%:*}"
  rest="${entry#*:}"
  # entry is file:lineno:text; extract text after second colon
  text="${rest#*:}"
  if ! echo "$text" | grep -q -F -e 'issue:'; then
    bare_policy="$bare_policy $file:${rest%%:*}"
  fi
done <"$excludes_scratch/scoped.txt"
if [[ -z "$bare_policy" ]]; then
  ok
else
  bad "bare policy: without reason: plus issue: in scoped excludes:$bare_policy (want every marker with reason: plus issue: plus policy:, issue #1055)"
fi

# Scoped markers carry specific reason: (not bare policy: alone).
if ! grep -F -e 'LCOV_EXCL_' "$excludes_scratch/scoped.txt" | grep -v -F -e 'reason:' | grep -q .; then
  ok
else
  bad "scoped exclude without reason: (want specific reason: on every marker, issue #1055)"
fi

# Strategy details own the budgeted record with expiry plus harness pin.
if grep -q -F -e 'reason: <what/why>, issue:' "$strategy" &&
  grep -q -F -e 'Bare `policy:` without' "$strategy" &&
  grep -q -F -e 'tools/coverage/excludes-budget.txt' "$strategy" &&
  grep -q -F -e '60 ranges plus 28 singles' "$strategy" &&
  grep -q -F -e 'expires 2027-03-01' "$strategy" &&
  grep -q -F -e 'issue #1055' "$strategy" &&
  grep -q -F -e 'bazel run //tools/ci:coverage_excludes_qualification' "$strategy"; then
  ok
else
  bad "docs/testing/strategy-details.md lost its excludes-budget record (want reason plus issue syntax plus bare-policy rejection plus budget plus expiry plus harness pin, issue #1055)"
fi

# AGENTS root/docs stay synced on the short-specific reason rule.
if grep -q -F -e 'LCOV_EXCL_*` reasons stay short specific' AGENTS.md &&
  grep -q -F -e 'LCOV_EXCL_*` reasons stay short specific' docs/AGENTS.md &&
  grep -q -F -e 'reason:` plus `issue:`' AGENTS.md &&
  grep -q -F -e 'reason:` plus `issue:`' docs/AGENTS.md; then
  ok
else
  bad "AGENTS root/docs lost the short-specific LCOV reason rule (want reason plus issue plus policy in both, issue #1055)"
fi

# BUILD owns the harness target.
if grep -q -F -e 'name = "coverage_excludes_qualification"' "$build"; then
  ok
else
  bad "tools/ci/ci_targets_a.bzl lost the coverage_excludes_qualification target (want Bazel-owned harness, issue #1055)"
fi

# CI wires the harness in prove plus dogfood-freshness.
if grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_excludes_qualification' "$prove" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:coverage_excludes_qualification' "$dogfood"; then
  ok
else
  bad "tools/ci/prove.sh or dogfood_freshness.sh lost the coverage_excludes_qualification wiring (want both, issue #1055)"
fi

dx_test_summary "lcov excludes-budget qualification harness"
