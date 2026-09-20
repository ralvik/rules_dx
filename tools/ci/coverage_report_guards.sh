#!/usr/bin/env bash
# Coverage-report guards (; relates,,).
#
# First-party coverage PR reporting is adopted here (no service
# dependency) so Codecov stays at most opt-in. Source of truth stays
# Bazel-owned (`dx coverage --min-coverage`, combined LCOV through
# `coverage_bin` against the seed inventory); the summary comment is
# presentation only and never the gate.
#
# This harness machine-checks the landed contract on a clean tree: the
# gate stays enforced, the renderer exists, both workflows publish one
# deduped marker-owned comment per PR cell with fork-safe handling, no
# third-party coverage action is smuggled in, docs select first-party,
# and the renderer proves the failure cases (missing report, uncovered
# lines, partial verdict, rerun dedup) plus the fork/publication
# semantics statically.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_report_guards`,
# following //tools/ci:coverage_spill.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Gate stays Bazel-owned: the coverage job enforces `dx coverage
# --min-coverage` (presentation never substitutes for the gate).
if grep -q -F -e 'coverage --min-coverage' .github/workflows/ci.yml; then
  ok
else
  bad "ci coverage job lost the dx coverage gate"
fi

# The seed-cell source the comment presents stays versioned.
if [[ -f "tools/ci/coverage_cell.sh" &&
  -f "tools/coverage/seed-inventory.txt" ]]; then
  ok
else
  bad "coverage cell source (script + seed inventory) missing"
fi

# The first-party renderer stays versioned with its Bazel target.
if [[ -x "tools/coverage/coverage_comment.sh" ]] &&
  grep -q -F -e 'name = "coverage_comment"' tools/coverage/BUILD.bazel &&
  grep -q -F -e 'target_compatible_with = ["@platforms//os:linux"]' tools/coverage/BUILD.bazel; then
  ok
else
  bad "coverage comment renderer missing (script, target, or Linux-only label)"
fi

# Consumer coverage path stays wired (same gate for consumidores).
if grep -q -F -e 'min_coverage' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost its min_coverage coverage path"
fi

# No third-party coverage service smuggled into CI (first-party only).
if ! grep -q -F -e 'codecov-action' .github/workflows/ci.yml &&
  ! grep -q -F -e 'codecov-action' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "a third-party codecov action appeared (first-party only)"
fi

# Job summaries stay a PR-visible coverage surface.
if grep -q -F -e 'GITHUB_STEP_SUMMARY' .github/workflows/ci.yml &&
  grep -q -F -e 'GITHUB_STEP_SUMMARY' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the step-summary reporting surface"
fi

# First-party comment wired in this repo: marker, renderer, gh publish.
if grep -q -F -e 'dx-coverage-summary' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage_comment' .github/workflows/ci.yml &&
  grep -q -F -e 'gh pr comment' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its first-party coverage comment wiring (marker/renderer/gh)"
fi

# First-party comment wired for consumers: per-cell marker, gh publish.
if grep -q -F -e 'dx-coverage-summary' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'gh pr comment' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost its first-party coverage comment wiring"
fi

# Dedup: marker-owned update path preserves human comments (existing
# lookup plus PATCH, never delete/repost).
if grep -q -F -e 'existing' .github/workflows/ci.yml &&
  grep -q -F -e 'PATCH' .github/workflows/ci.yml &&
  grep -q -F -e 'existing' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the marker-dedup update path (existing/PATCH)"
fi

# Fork safety: fork code never gets write credentials (skip with a
# visible step-summary note; execution steps run without GH_TOKEN).
if grep -q -F -e 'head.repo.full_name' .github/workflows/ci.yml &&
  grep -q -F -e 'Fork PR' .github/workflows/ci.yml &&
  grep -q -F -e 'head.repo.full_name' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the fork-PR skip guard"
fi

# Stale runs never overwrite current: concurrency cancels superseded
# runs and the publish step updates only the marker-owned comment.
if grep -q -F -e 'cancel-in-progress: true' .github/workflows/ci.yml &&
  grep -q -F -e 'never overwrite' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the stale-run guard (concurrency + marker-only update)"
fi

# Publication failure fails CI separately: the publish step carries no
# continue-on-error escape (unlike warn-only advisories), while gate
# outcomes stay preserved in the summary.
if grep -A 12 -F -e 'Publish PR summary comment' .github/workflows/ci.yml | grep -q -F -e 'continue-on-error'; then
  bad "coverage publish step must not carry continue-on-error (publication failure fails CI)"
else
  ok
fi

# No cross-cell union: cells stay separate in renderer and workflows.
if grep -q -F -e 'no cross-cell union' tools/coverage/coverage_comment.sh &&
  grep -q -F -e 'no union' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "first-party reporting lost its no-union record"
fi

# Renderer never turns a failing gate into success.
if grep -q -F -e 'never the gate' tools/coverage/coverage_comment.sh; then
  ok
else
  bad "renderer lost its presentation-only record"
fi

# Docs select first-party with Codecov at most opt-in.
if grep -q -F -e 'first-party coverage PR reporting' docs/testing/README.md &&
  grep -q -F -e 'Codecov stays at most opt-in' docs/testing/README.md &&
  grep -q -F -e 'first-party' docs/github-ci.md &&
  grep -q -F -e 'first-party PR summary' docs/contributing/local-workflows.md; then
  ok
else
  bad "docs lost the first-party selection (testing README, github-ci, local-workflows)"
fi

# Functional proofs below exercise the real renderer over fixtures.
dx_mkscratch scratch

# PASS fixture renders PASS with zero uncovered and the stable marker.
printf 'coverage gate: PASS 10/10 executable lines\n  cli/lcov/src/lib.rs: 10/10 (0 ignored)\ninformational line rate: 100.00%% (exact counts decide, never rounding)\n' >"$scratch/pass.txt"
if tools/coverage/coverage_comment.sh --cell "seed linux_x86_64" --revision abc123 --run run-1 \
  --out "$scratch/pass.md" --gate-output "$scratch/pass.txt" --gate-exit 0 --lcov "$scratch/pass.txt" &&
  grep -q -F -e '## Coverage summary' "$scratch/pass.md" &&
  grep -q -F -e 'PASS' "$scratch/pass.md" &&
  grep -q -F -e '<!-- dx-coverage-summary: seed linux_x86_64 -->' "$scratch/pass.md" &&
  grep -q -F -e 'zero uncovered' "$scratch/pass.md"; then
  ok
else
  bad "renderer PASS fixture did not render PASS with marker and zero-uncovered"
fi

# Uncovered lines render FAIL with the file:line location, never PASS.
printf 'coverage gate: FAIL 9/10 executable lines\n  cli/lcov/src/lib.rs: 9/10 uncovered: cli/lcov/src/lib.rs:7\nerrors:\n  - uncovered\n' >"$scratch/fail.txt"
if tools/coverage/coverage_comment.sh --cell "seed linux_x86_64" --revision abc123 --run run-1 \
  --out "$scratch/fail.md" --gate-output "$scratch/fail.txt" --gate-exit 1 --lcov "$scratch/fail.txt" &&
  grep -q -F -e 'FAIL' "$scratch/fail.md" &&
  grep -q -F -e 'cli/lcov/src/lib.rs:7' "$scratch/fail.md" &&
  ! grep -q -F -e '— PASS' "$scratch/fail.md"; then
  ok
else
  bad "renderer uncovered-lines fixture did not fail visibly with its location"
fi

# Missing report renders FAIL closed (never success, never empty).
if tools/coverage/coverage_comment.sh --cell "seed linux_x86_64" --revision abc123 --run run-1 \
  --out "$scratch/missing.md" --gate-output "$scratch/absent.txt" --gate-exit 1 --lcov "$scratch/absent.lcov" &&
  grep -q -F -e 'FAIL' "$scratch/missing.md" &&
  grep -q -F -e 'missing' "$scratch/missing.md"; then
  ok
else
  bad "renderer missing-report fixture did not fail closed"
fi

# Partial verdict (unrecognized header) renders FAIL, never PASS.
printf 'partial garbage without a verdict header\nDA:1,1\n' >"$scratch/partial.txt"
if tools/coverage/coverage_comment.sh --cell "seed linux_x86_64" --revision abc123 --run run-1 \
  --out "$scratch/partial.md" --gate-output "$scratch/partial.txt" --gate-exit 1 --lcov "$scratch/partial.txt" &&
  grep -q -F -e 'FAIL' "$scratch/partial.md" &&
  ! grep -q -F -e '— PASS' "$scratch/partial.md"; then
  ok
else
  bad "renderer partial-verdict fixture did not fail visibly"
fi

# Rerun dedup: two renders carry the same single marker line.
if tools/coverage/coverage_comment.sh --cell "seed linux_x86_64" --revision abc123 --run run-2 \
  --out "$scratch/rerun.md" --gate-output "$scratch/pass.txt" --gate-exit 0 --lcov "$scratch/pass.txt" &&
  [[ "$(grep -c -F -e 'dx-coverage-summary' "$scratch/pass.md")" == "1" ]] &&
  [[ "$(grep -c -F -e 'dx-coverage-summary' "$scratch/rerun.md")" == "1" ]] &&
  [[ "$(grep -F -e 'dx-coverage-summary' "$scratch/pass.md")" == "$(grep -F -e 'dx-coverage-summary' "$scratch/rerun.md")" ]]; then
  ok
else
  bad "renderer marker is not stable/single across reruns (dedup)"
fi

# Revision/run identity is bound into every summary.
if grep -q -F -e 'abc123' "$scratch/pass.md" && grep -q -F -e 'run-1' "$scratch/pass.md"; then
  ok
else
  bad "renderer summary lost its revision/run identity"
fi

dx_test_summary "coverage report guards harness"
