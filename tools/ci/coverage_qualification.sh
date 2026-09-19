#!/usr/bin/env bash
# Coverage/remote qualification harness (issue #308).
#
# Closes the five qualification gaps named in #308 with machine-checked
# evidence on a clean tree, without paid infrastructure:
# - per-cell LCOV gating (seed qualified, rest unqualified, never unioned),
# - Starlark instrumentation-vs-behavioral-matrix decision with evidence,
# - Codecov opt-in-only qualification (no activation, no upload wiring),
# - free-tier quota qualification for the services actually used,
# - remote-cache plus remote-execution local-only evidence (else branch:
#   hermeticity designed and locally sandbox-tested, remote unverified).
#
# First-party Bazel-owned coverage stays the gate (issue #254 adopted);
# Codecov stays opt-in only and is never required. Remote correctness is
# not claimed: local aquery plus execution-log evidence proves cache
# behavior locally, and docs state remote remains unverified. All required
# non-seed cells stay unqualified per the platform policy (issues #5/#298)
# with clean refusal, never silent substitution.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_qualification`,
# following //tools/ci:coverage_cell.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

cells="tools/coverage/cells.txt"
seed_inventory="tools/coverage/seed-inventory.txt"

# Per-cell registry exists with exactly one qualified seed row.
if [[ -f "$cells" ]] &&
  [[ "$(grep -c -E -e '^qualified ' "$cells")" == "1" ]] &&
  grep -q -F -e 'qualified seed-linux_x86_64 tools/coverage/seed-inventory.txt' "$cells"; then
  ok
else
  bad "cells registry missing or seed row wrong: $cells"
fi

# Unqualified rows cover the v1 required hosts that stay platform-gated.
if grep -q -F -e 'unqualified linux_arm64 issue-298' "$cells" &&
  grep -q -F -e 'unqualified linux_x86_64_musl issue-298' "$cells" &&
  grep -q -F -e 'unqualified linux_arm64_musl issue-298' "$cells" &&
  grep -q -F -e 'unqualified macos_arm64 issue-298' "$cells" &&
  grep -q -F -e 'unqualified windows_x86_64 issue-298' "$cells" &&
  [[ "$(grep -c -E -e '^unqualified ' "$cells")" == "5" ]]; then
  ok
else
  bad "cells registry lost an unqualified required host (want 5 issue-298 rows)"
fi

# No duplicate cell names in the registry.
if [[ "$(grep -E -e '^(qualified|unqualified) ' "$cells" | awk '{print $2}' | LC_ALL=C sort | uniq -d | wc -l)" == "0" ]]; then
  ok
else
  bad "cells registry has a duplicate cell name"
fi

# Qualified inventory exists and stays Rust-only (no Starlark line data).
if [[ -f "$seed_inventory" ]] &&
  ! grep -E -e '\.bzl$' "$seed_inventory" | grep -q . &&
  grep -q -F -e 'eligible cli/lcov/src/lib.rs' "$seed_inventory"; then
  ok
else
  bad "seed inventory missing, empty, or gained non-Rust scope"
fi

# No cross-cell union: renderer, workflows, and docs keep cells separate.
if grep -q -F -e 'no cross-cell union' tools/coverage/coverage_comment.sh &&
  grep -q -F -e 'no union' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'no cross-cell union' docs/testing/README.md; then
  ok
else
  bad "per-cell no-union record lost (renderer, consumer workflow, or testing README)"
fi

# Seed coverage job stays seed-cell scoped in CI.
if grep -q -F -e 'coverage (dx coverage gate, seed cell)' .github/workflows/ci.yml &&
  grep -q -F -e 'seed linux_x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage --min-coverage' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the seed-cell coverage gate scope"
fi

# Consumer coverage stays per-cell with no union and Codecov opt-in only.
if grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'Codecov stays opt-in only' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost its per-cell no-union coverage shape"
fi

# Functional per-cell proof without a full rebuild: two synthetic cell
# LCOVs where a cross-cell union would hide the gap. The gate binary must
# pass the covered cell and fail the uncovered cell with its location, so
# per-cell gating (not unioning) is what decides.
dx_mkscratch scratch
check_bin="bazel-bin/tools/coverage/coverage_bin"
if [[ ! -x "$check_bin" ]]; then
  bazel build --noshow_progress //tools/coverage:coverage_bin >/dev/null 2>&1
fi
printf 'SF:cli/lcov/src/lib.rs\nDA:1,1\nend_of_record\n' >"$scratch/cell-pass.lcov"
printf 'SF:cli/lcov/src/lib.rs\nDA:1,0\nend_of_record\n' >"$scratch/cell-fail.lcov"
printf 'eligible cli/lcov/src/lib.rs\n' >"$scratch/cell-inventory.txt"
printf 'cli/lcov/src/lib.rs\n' >"$scratch/cell-sources.txt"
pass_rc=0
pass_out="$("$check_bin" --report "$scratch/cell-pass.lcov" \
  --inventory "$scratch/cell-inventory.txt" --sources "$scratch/cell-sources.txt" \
  --root . 2>&1)" || pass_rc=$?
fail_rc=0
fail_out="$("$check_bin" --report "$scratch/cell-fail.lcov" \
  --inventory "$scratch/cell-inventory.txt" --sources "$scratch/cell-sources.txt" \
  --root . 2>&1)" || fail_rc=$?
if [[ "$pass_rc" == "0" ]] && echo "$pass_out" | grep -q 'coverage gate: PASS' &&
  [[ "$fail_rc" == "1" ]] && echo "$fail_out" | grep -q 'uncovered: cli/lcov/src/lib.rs:1'; then
  ok
else
  bad "per-cell functional proof failed (pass rc=$pass_rc fail rc=$fail_rc)"
fi

# Starlark decision: infeasibility evidence stays versioned in the matrix.
if grep -q -F -e '0-byte `coverage.dat`' libs/starlark/behavioral_matrix.md &&
  grep -q -F -e 'bazelbuild/bazel#15594' libs/starlark/behavioral_matrix.md &&
  grep -q -F -e '9.2.0' libs/starlark/behavioral_matrix.md; then
  ok
else
  bad "behavioral matrix lost its Starlark infeasibility evidence (probe, flags, upstream #15594)"
fi

# Starlark fallback stays behavioral, never line coverage.
if grep -q -F -e 'no Starlark line-coverage percentage' docs/testing/starlark.md &&
  grep -q -F -e 'matrix_validation' docs/testing/starlark.md &&
  grep -q -F -e 'never be presented as source-line' docs/testing/starlark.md; then
  ok
else
  bad "docs/testing/starlark.md lost its no-line-coverage plus matrix_validation record"
fi

# Matrix validation target stays wired with all ten anchors.
if grep -q -F -e 'matrix_validation_tests(name = "matrix_validation")' libs/starlark/tests/BUILD.bazel &&
  grep -q -F -e 'matrix-item: expect_equal' libs/starlark/tests/matrix_tests.bzl &&
  grep -q -F -e 'matrix-item: tested-stack' libs/starlark/tests/matrix_tests.bzl; then
  ok
else
  bad "matrix_validation wiring lost (BUILD target or anchor list)"
fi

# Testing README records the Starlark route as investigated plus fallback.
if grep -q -F -e 'Empirical Starlark feasibility evidence ran against the pinned Bazel' docs/testing/README.md &&
  grep -q -F -e 'behavioral matrix' docs/testing/README.md; then
  ok
else
  bad "testing README lost its Starlark feasibility plus fallback record"
fi

# Codecov stays opt-in only: no activation, no upload wiring anywhere.
# (Self-excluded: this script names the banned forms in its own patterns.)
if ! grep -rn -F -e 'codecov-action' .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'CODECOV_TOKEN' --exclude='coverage_qualification.sh' .github/workflows/ tools/ cli/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'codecov upload' --exclude='coverage_qualification.sh' .github/workflows/ tools/ 2>/dev/null | grep -q .; then
  ok
else
  bad "Codecov upload wiring appeared (action, token, or upload step)"
fi

# Docs select first-party with Codecov at most opt-in and no activation.
if grep -q -F -e 'Codecov stays at most opt-in' docs/testing/README.md &&
  grep -q -F -e 'No Codecov account' docs/testing/README.md &&
  grep -q -F -e 'Codecov stays opt-in only' docs/github-ci.md; then
  ok
else
  bad "docs lost the Codecov opt-in-only plus no-activation record"
fi

# Free-tier qualification: only standard runners, no paid services.
if ! grep -rn -E -e 'runs-on:.*(self-hosted|larger|windows-latest|macos-latest)' .github/workflows/ 2>/dev/null | grep -q . &&
  grep -q -F -e 'runs-on: ubuntu-latest' .github/workflows/ci.yml &&
  grep -q -F -e 'actions/cache' .github/workflows/ci.yml; then
  ok
else
  bad "workflows gained a non-standard runner (paid or unqualified)"
fi

# Free-tier quotas stay recorded in the infrastructure budget.
if grep -q -F -e 'standard GitHub-hosted runners is free' docs/testing/README.md &&
  grep -q -F -e '10 GB' docs/testing/README.md &&
  grep -q -F -e '500 MB' docs/testing/README.md &&
  grep -q -F -e 'Larger runners are always charged' docs/testing/README.md; then
  ok
else
  bad "testing README lost its free-tier quota record (runners, cache, artifact)"
fi

# No remote execution or cache flags in owned config or workflows.
if ! grep -rn -F -e '--remote_cache' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e '--remote_executor' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e '--bes_backend' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "a remote cache/executor flag appeared (local-only execution)"
fi

# CI header stays local-only with platform qualification owned elsewhere.
if grep -q -F -e 'local execution, no remote' .github/workflows/ci.yml &&
  grep -q -F -e 'issue #298' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its local-only plus #298 platform record"
fi

# Remote Tests section states the local-only else branch explicitly.
if grep -q -F -e 'locally sandbox-tested but remote behavior remains unverified' docs/testing/README.md &&
  grep -q -F -e 'hermeticity is' docs/testing/README.md; then
  ok
else
  bad "testing README lost its locally-tested-only remote record"
fi

# No remote-correctness claim may appear in docs.
if ! grep -rn -F -e 'remote-cache correctness' docs/ 2>/dev/null | grep -v -F -e 'Remote cache tests are required' | grep -q . &&
  ! grep -rn -F -e 'remotely executable' docs/ 2>/dev/null | grep -v -F -e 'required before declaring' | grep -v -F -e 'when a' | grep -q .; then
  ok
else
  bad "docs claim remote correctness that has no remote evidence"
fi

# Local cache evidence stays wired (aquery shape plus exec-log hits).
if [[ -f "tools/ci/quality_cache_aquery.sh" ]] &&
  grep -q -F -e 'cache hit' tools/ci/quality_cache_aquery.sh &&
  [[ -f "tools/ci/examples_laziness_runtime.sh" ]]; then
  ok
else
  bad "local cache evidence harnesses missing (aquery plus exec-log)"
fi

dx_test_summary "coverage qualification harness"
