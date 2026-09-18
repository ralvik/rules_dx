#!/usr/bin/env bash
# Coverage-report guards (issue #254; relates #54, #252, #5).
#
# PRs get no coverage summary from our own tooling today: the gate is
# local (`dx coverage --min-coverage`, Bazel-owned combined LCOV) plus a
# `$GITHUB_STEP_SUMMARY` line, while `docs/testing/README.md` selects
# Codecov with activation/upload unqualified (per #5). Issue #254 wants
# first-party coverage PR reporting (compact summary comment, no service
# dependency) adopted here so Codecov stays at most opt-in.
#
# This harness machine-checks the contract half verifiable on a clean
# tree today (10 checks): the Bazel-owned gate stays the source of
# truth, the LCOV/reporting/codecov-selection records stay honest, the
# consumer coverage path stays wired, no third-party coverage action is
# smuggled into CI, and the PR-summary-comment workflow stays recorded
# as open (not claimed). Comment presentation, dedup, fork handling, and
# failure-case proofs stay open under #254.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_report_guards`,
# following //tools/ci:coverage_spill.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# Gate stays Bazel-owned: the coverage job enforces `dx coverage
# --min-coverage` (presentation never substitutes for the gate).
if grep -q -F -e 'coverage --min-coverage' .github/workflows/ci.yml; then
  ok
else
  bad "ci coverage job lost the dx coverage gate"
fi

# LCOV stays the canonical validated report shape.
if grep -q -F -e '## LCOV' docs/cli/standard-reports.md; then
  ok
else
  bad "standard-reports lost its LCOV contract section"
fi

# Current selection stays honest: Codecov documented with activation
# unqualified per #5 (first-party comment arrives under #254).
if grep -q -F -e 'Codecov' docs/testing/README.md \
  && grep -q -F -e 'issues/5' docs/testing/README.md; then
  ok
else
  bad "testing README lost its Codecov-selection + #5 qualification record"
fi

# Reporting contract exists for consumers (ownership + check discipline).
if grep -q -F -e 'reporting' docs/github-ci.md; then
  ok
else
  bad "github-ci doc lost its reporting contract"
fi

# Fork security stays in the contract (unprivileged PRs never get write
# credentials for reporting).
if grep -q -F -e 'fork' docs/github-ci.md; then
  ok
else
  bad "github-ci doc lost its fork-security record"
fi

# The seed-cell source the comment must present stays versioned.
if [[ -f "tools/ci/coverage_cell.sh" \
  && -f "tools/coverage/seed-inventory.txt" ]]; then
  ok
else
  bad "coverage cell source (script + seed inventory) missing"
fi

# Consumer coverage path stays wired (same gate for consumidores).
if grep -q -F -e 'min_coverage' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost its min_coverage coverage path"
fi

# No third-party coverage service smuggled into CI (first-party only).
if ! grep -q -F -e 'codecov-action' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml gained a third-party codecov action (first-party only)"
fi

# Job summaries stay the current (only) PR-visible coverage surface.
if grep -q -F -e 'GITHUB_STEP_SUMMARY' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its step-summary reporting surface"
fi

# Gap stays honest: no PR-summary-comment workflow exists yet, so the
# #254 comment behavior is open, not claimed.
if [[ ! -f ".github/workflows/coverage-comment.yml" ]]; then
  ok
else
  bad "coverage-comment workflow appeared without #254 qualification"
fi

echo "coverage report guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
