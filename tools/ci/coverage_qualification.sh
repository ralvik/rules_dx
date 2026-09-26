#!/usr/bin/env bash
# Coverage/remote qualification harness.
#
# Qualifies the per-cell non-seed coverage plus Codecov opt-in plus
# remote evidence slice with fixture evidence pinned in
# `tools/coverage/tests/fixtures/per_cell/pins.bzl` (plus
# `per_cell.expected` plus `codecov_remote.expected`), without claiming
# qualified floors, qualified accounting, or Supported:
# - per-cell LCOV gating (seed plus arm64 plus two static-musl plus macos
#   arm64 plus windows x86_64 qualified, all
#   required qualified, never unioned),
# - Starlark instrumentation-vs-behavioral-matrix decision with evidence,
# - Codecov opt-in-only qualification (no activation, no upload wiring),
# - free-tier quota qualification for the services actually used,
# - remote-execution local-only evidence plus the optional shared
#   BuildBuddy remote cache (read-only, PR uploads disabled; else
#   branch: hermeticity designed and locally sandbox-tested, remote
#   unverified).
#
# First-party Bazel-owned coverage stays the gate (adopted);
# Codecov stays opt-in only and is never required. Remote correctness is
# not claimed: local aquery plus execution-log evidence proves cache
# behavior locally, and docs state remote remains unverified. All required
# cells are qualified per the platform policy (issues
# /, arm64 qualified under, static musl under, macos arm64
# under, windows x86_64 under
# with clean refusal for the remaining out-of-v1 hosts plus Not-planned macOS
# x86_64, never silent
# substitution.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_qualification`,
# following //tools/ci:coverage_cell.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

cells="tools/coverage/cells.txt"
seed_inventory="tools/coverage/seed-inventory.txt"
pins="tools/coverage/tests/fixtures/per_cell/pins.bzl"
pins_build="tools/coverage/tests/fixtures/per_cell/BUILD.bazel"
per_cell_expected="tools/coverage/tests/fixtures/per_cell/per_cell.expected"
codecov_remote_expected="tools/coverage/tests/fixtures/per_cell/codecov_remote.expected"
testing_readme="docs/testing/strategy-details.md"
github_ci="docs/github-ci.md"
build_coverage_doc="docs/cli/commands/build-test-coverage.md"

# Per-cell registry exists with exactly six qualified rows (seed x86_64
# plus arm64 native plus two static-musl profiles under
# plus macos arm64 native
# plus windows x86_64 MSVC-compatible
# native).
if [[ -f "$cells" ]] &&
  [[ "$(grep -c -E -e '^qualified ' "$cells")" == "6" ]] &&
  grep -q -F -e 'qualified seed-linux_x86_64 tools/coverage/seed-inventory.txt' "$cells" &&
  grep -q -F -e 'qualified linux_arm64 tools/coverage/arm64-inventory.txt' "$cells" &&
  grep -q -F -e 'qualified linux_x86_64_musl tools/coverage/musl-x86_64-inventory.txt' "$cells" &&
  grep -q -F -e 'qualified linux_arm64_musl tools/coverage/musl-arm64-inventory.txt' "$cells" &&
  grep -q -F -e 'qualified macos_arm64 tools/coverage/macos-arm64-inventory.txt' "$cells" &&
  grep -q -F -e 'qualified windows_x86_64 tools/coverage/windows-x86_64-inventory.txt' "$cells"; then
  ok
else
  bad "cells registry missing or a qualified row wrong: $cells"
fi

# All required hosts are qualified (windows x86_64 flipped qualified
# under); no unqualified coverage row remains. Out-of-v1 Windows arm64
# plus Not-planned macOS x86_64 carry no coverage cell.
if [[ "$(grep -c -E -e '^unqualified ' "$cells" || true)" == "0" ]] &&
  ! grep -q -F -e 'unqualified windows_x86_64' "$cells"; then
  ok
else
  bad "cells registry gained an unqualified row (want 0: all required qualified)"
fi

# No duplicate cell names in the registry.
if [[ "$(grep -E -e '^(qualified|unqualified) ' "$cells" | awk '{print $2}' | LC_ALL=C sort | uniq -d | wc -l)" == "0" ]]; then
  ok
else
  bad "cells registry has a duplicate cell name"
fi

# Qualified inventories exist and stay Rust-only (no Starlark line data).
# All six cells gate the same first-party scope; only the header prose differs.
if [[ -f "$seed_inventory" ]] &&
  [[ -f "tools/coverage/arm64-inventory.txt" ]] &&
  [[ -f "tools/coverage/musl-x86_64-inventory.txt" ]] &&
  [[ -f "tools/coverage/musl-arm64-inventory.txt" ]] &&
  [[ -f "tools/coverage/macos-arm64-inventory.txt" ]] &&
  [[ -f "tools/coverage/windows-x86_64-inventory.txt" ]] &&
  ! grep -E -e '\.bzl$' "$seed_inventory" | grep -q . &&
  ! grep -E -e '\.bzl$' tools/coverage/arm64-inventory.txt | grep -q . &&
  ! grep -E -e '\.bzl$' tools/coverage/musl-x86_64-inventory.txt | grep -q . &&
  ! grep -E -e '\.bzl$' tools/coverage/musl-arm64-inventory.txt | grep -q . &&
  ! grep -E -e '\.bzl$' tools/coverage/macos-arm64-inventory.txt | grep -q . &&
  ! grep -E -e '\.bzl$' tools/coverage/windows-x86_64-inventory.txt | grep -q . &&
  grep -q -F -e 'eligible cli/lcov/src/lib.rs' "$seed_inventory" &&
  cmp -s <(grep -E -e '^(eligible|support) ' "$seed_inventory") <(grep -E -e '^(eligible|support) ' tools/coverage/arm64-inventory.txt) &&
  cmp -s <(grep -E -e '^(eligible|support) ' "$seed_inventory") <(grep -E -e '^(eligible|support) ' tools/coverage/musl-x86_64-inventory.txt) &&
  cmp -s <(grep -E -e '^(eligible|support) ' "$seed_inventory") <(grep -E -e '^(eligible|support) ' tools/coverage/musl-arm64-inventory.txt) &&
  cmp -s <(grep -E -e '^(eligible|support) ' "$seed_inventory") <(grep -E -e '^(eligible|support) ' tools/coverage/macos-arm64-inventory.txt) &&
  cmp -s <(grep -E -e '^(eligible|support) ' "$seed_inventory") <(grep -E -e '^(eligible|support) ' tools/coverage/windows-x86_64-inventory.txt); then
  ok
else
  bad "seed/arm64/musl/macos/windows inventories missing, non-Rust scope, or out of sync"
fi

# No cross-cell union: renderer, workflows, and docs keep cells separate.
if grep -q -F -e 'no cross-cell union' tools/coverage/coverage_comment.sh &&
  grep -q -F -e 'no union' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'no cross-cell union' docs/testing/strategy-details.md; then
  ok
else
  bad "per-cell no-union record lost (renderer, consumer workflow, or testing README)"
fi

# Per-cell coverage gate scope as built: the two static-musl cells keep
# their own ci.yml jobs (cell label plus `dx coverage --min-coverage 97`
# plus its versioned inventory), while the seed plus arm64 plus macos
# arm64 plus windows x86_64 cells gate through the dogfood self-call's
# per-platform coverage job with the same min_coverage (static musl only,
# ; dynamic musl has no cell; macos arm64 native on macos-14,
# ; windows x86_64 MSVC-compatible native on
# windows-latest,).
if grep -q -F -e 'coverage-musl-x86_64 (dx coverage gate, musl x86_64 cell)' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage-musl-arm64 (dx coverage gate, musl arm64 cell)' .github/workflows/ci.yml &&
  grep -q -F -e 'musl-x86_64 linux_x86_64_musl' .github/workflows/ci.yml &&
  grep -q -F -e 'musl-arm64 linux_arm64_musl' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage --min-coverage 97' .github/workflows/ci.yml &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" .github/workflows/ci.yml &&
  grep -q -F -e 'min_coverage: "97"' .github/workflows/ci.yml &&
  grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the seed/arm64/musl/macos/windows per-cell coverage gate scope"
fi

# Consumer coverage stays per-cell with no union and Codecov opt-in only.
if grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'Codecov stays opt-in only' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost its per-cell no-union coverage shape"
fi

# Each non-seed cell gates its own LCOV: the two musl cells through the
# portable composite (issue #1062: same coverage_bin gate plus same
# coverage_comment.sh shape as the seed, with its versioned inventory
# wired per job, step-summary only), and the arm64 plus macos arm64 plus
# windows x86_64 cells through the consumer coverage job's per-platform
# `dx coverage` gate with its own per-cell render.
if grep -q -F -e 'inventory: tools/coverage/musl-x86_64-inventory.txt' .github/workflows/ci.yml &&
  grep -q -F -e 'inventory: tools/coverage/musl-arm64-inventory.txt' .github/workflows/ci.yml &&
  grep -q -F -e 'uses: ./.github/actions/render-coverage-summary' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage_bin' .github/actions/render-coverage-summary/action.yml &&
  grep -q -F -e 'coverage_comment.sh' .github/actions/render-coverage-summary/action.yml &&
  grep -q -F -e 'extra=(--min-coverage "$DX_MIN_COVERAGE")' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "workflows lost the per-cell own-LCOV gate wiring (portable composite plus versioned inventories, issue #1062)"
fi

# Functional per-cell proof without a full rebuild: two synthetic cell
# LCOVs where a cross-cell union would hide the gap. The gate binary must
# pass the covered cell and fail the uncovered cell with its location, so
# per-cell gating (not unioning) is what decides.
dx_mkscratch scratch
# `bazel coverage` elsewhere leaves `coverage_bin` instrumented
# (`-C instrument-coverage`), and this harness executes it directly 2x from
# the workspace root. With `LLVM_PROFILE_FILE` unset, Rust writes
# `default_%m_%p.profraw` to CWD per invocation. Redirect profiles into the
# auto-cleaned scratch dir so harness runs spill nothing to the checkout.
export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"
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
if grep -q -F -e 'genuine executable-line instrumentation under' docs/testing/strategy-details.md &&
  grep -q -F -e 'behavioral matrix' docs/testing/strategy-details.md; then
  ok
else
  bad "testing README lost its Starlark feasibility plus fallback record"
fi

# Codecov stays opt-in only: no activation, no upload wiring anywhere.
# (Self-excluded: this script names the banned forms in its own patterns;
# the pins fixture records the same banned forms as pins, so its
# path is filtered out and only real wiring can fail this check)
# (hermetic tree search: BSD grep lacks --exclude, issue #1006).
if dx_tree_absent 'codecov-action' -- .github/workflows/ &&
  dx_tree_absent --exclude='coverage_qualification.sh' --allow='tools/coverage/tests/fixtures/per_cell/' 'CODECOV_TOKEN' -- .github/workflows/ tools/ cli/ &&
  dx_tree_absent --exclude='coverage_qualification.sh' --allow='tools/coverage/tests/fixtures/per_cell/' 'codecov upload' -- .github/workflows/ tools/; then
  ok
else
  bad "Codecov upload wiring appeared (action, token, or upload step)"
fi

# Docs select first-party with Codecov at most opt-in and no activation.
if grep -q -F -e 'Codecov stays at most opt-in' docs/testing/strategy-details.md &&
  grep -q -F -e 'No Codecov account' docs/testing/strategy-details.md &&
  grep -q -F -e 'Codecov stays opt-in only' docs/github-ci.md; then
  ok
else
  bad "docs lost the Codecov opt-in-only plus no-activation record"
fi

# Free-tier qualification: only standard runners, no paid services.
# windows-latest is qualified (standard free runner; the Bazel setup
# cache rides setup-bazel plus the shared remote cache); macos-latest
# stays banned (unpinned), as do self-hosted/larger (paid). No
# actions/cache wiring remains anywhere under .github/workflows/ (the
# setup-bazel cache is internal to that action, never workflow YAML)
# (hermetic tree search: host grep -rn variance, issue #1006).
if DX_TREE_RE=1 dx_tree_absent 'runs-on:.*(self-hosted|larger|macos-latest)' -- .github/workflows/ &&
  grep -q -F -e 'runs-on: ubuntu-latest' .github/workflows/ci.yml &&
  grep -q -F -e "'windows-latest'" .github/workflows/reusable-consumer.yml &&
  dx_tree_absent 'actions/cache' -- .github/workflows/; then
  ok
else
  bad "workflows gained a non-standard runner (paid or unqualified) or actions/cache wiring"
fi

# Free-tier quotas stay recorded in the infrastructure budget.
if grep -q -F -e 'standard GitHub-hosted runners is free' docs/testing/strategy-details.md &&
  grep -q -F -e '10 GB' docs/testing/strategy-details.md &&
  grep -q -F -e '500 MB' docs/testing/strategy-details.md &&
  grep -q -F -e 'Larger runners are always charged' docs/testing/strategy-details.md; then
  ok
else
  bad "testing README lost its free-tier quota record (runners, cache, artifact)"
fi

# The shared BuildBuddy remote cache is wired in every job while
# execution stays local: `--remote_cache=grpcs://remote.buildbuddy.io`
# plus the PR no-upload flag are required in both workflows, and no
# remote executor or BES backend flag appears in owned config or
# workflows (hermetic tree search: BSD grep lacks --exclude-dir, issue
# #1006).
if grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' .github/workflows/ci.yml &&
  grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'common --noremote_upload_local_results' .github/workflows/ci.yml &&
  grep -q -F -e 'common --noremote_upload_local_results' .github/workflows/reusable-consumer.yml &&
  dx_tree_absent '--remote_executor' -- .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ &&
  dx_tree_absent '--bes_backend' -- .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/; then
  ok
else
  bad "remote wiring lost (want BuildBuddy remote cache plus PR no-upload in workflows; no remote executor or BES backend)"
fi

# CI header stays local-only with platform qualification owned elsewhere
# (policy notes live in docs/testing/workflow-notes.md after the split).
if grep -q -F -e 'local execution, no remote' docs/testing/workflow-notes.md &&
  grep -q -F -e 'issue #298' docs/testing/workflow-notes.md; then
  ok
else
  bad "workflow-notes lost its local-only plus #298 platform record"
fi

# Remote Tests section states the local-only else branch explicitly.
if grep -q -F -e 'locally sandbox-tested but remote behavior remains unverified' docs/testing/strategy-details.md &&
  grep -q -F -e 'hermeticity is' docs/testing/strategy-details.md; then
  ok
else
  bad "testing README lost its locally-tested-only remote record"
fi

# No remote-correctness claim may appear in docs
# (hermetic tree search: host grep -rn variance, issue #1006).
if dx_tree_absent --allow='Remote cache tests are required' 'remote-cache correctness' -- docs/ &&
  dx_tree_absent --allow='required before declaring' --allow='when a' 'remotely executable' -- docs/; then
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

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$per_cell_expected" && -f "$codecov_remote_expected" ]]; then
  ok
else
  bad "per-cell coverage fixture missing (want $pins plus $pins_build plus per_cell.expected plus codecov_remote.expected)"
fi

# Pins record the per-cell registry with six qualified cells and no union.
if grep -q -F -e 'PER_CELL_COUNT = 6' "$pins" &&
  grep -q -F -e 'qualified seed-linux_x86_64 tools/coverage/seed-inventory.txt' "$pins" &&
  grep -q -F -e 'qualified linux_arm64 tools/coverage/arm64-inventory.txt' "$pins" &&
  grep -q -F -e 'qualified linux_x86_64_musl tools/coverage/musl-x86_64-inventory.txt' "$pins" &&
  grep -q -F -e 'qualified linux_arm64_musl tools/coverage/musl-arm64-inventory.txt' "$pins" &&
  grep -q -F -e 'qualified macos_arm64 tools/coverage/macos-arm64-inventory.txt' "$pins" &&
  grep -q -F -e 'qualified windows_x86_64 tools/coverage/windows-x86_64-inventory.txt' "$pins" &&
  grep -q -F -e 'no cross-cell union' "$pins" &&
  grep -q -F -e 'coverage --min-coverage 97 //...' "$pins"; then
  ok
else
  bad "pins.bzl lost its six-cell registry plus no-union plus rate-gate pins under issue #507"
fi

# Pins record the accepted decision: repo exact gate plus configurable
# per-cell requirement plus Codecov opt-in plus seed inventory plus no union.
if grep -q -F -e 'zero uncovered lines' "$pins" &&
  grep -q -F -e 'configurable requirement for users per-cell' "$pins" &&
  grep -q -F -e 'Codecov stays opt-in only' "$pins" &&
  grep -q -F -e 'never required' "$pins" &&
  grep -q -F -e 'tools/coverage/seed-inventory.txt' "$pins"; then
  ok
else
  bad "pins.bzl lost its accepted-decision pins (exact gate plus configurable per-cell plus Codecov opt-in, issue #507)"
fi

# Pins record the Starlark infeasibility plus behavioral fallback decision.
if grep -q -F -e '0-byte `coverage.dat`' "$pins" &&
  grep -q -F -e 'bazelbuild/bazel#15594' "$pins" &&
  grep -q -F -e '9.2.0' "$pins" &&
  grep -q -F -e 'behavioral matrix' "$pins" &&
  grep -q -F -e 'matrix_validation' "$pins" &&
  grep -q -F -e 'never be presented as source-line' "$pins"; then
  ok
else
  bad "pins.bzl lost its Starlark probe plus fallback pins under issue #507"
fi

# Pins record Codecov opt-in-only with no activation or upload wiring.
if grep -q -F -e 'CODECOV_TOKEN' "$pins" &&
  grep -q -F -e 'codecov upload' "$pins" &&
  grep -q -F -e 'first-party' "$pins" &&
  grep -q -F -e 'Codecov stays at most opt-in' "$pins" &&
  grep -q -F -e 'No Codecov account' "$pins"; then
  ok
else
  bad "pins.bzl lost its Codecov opt-in-only pins under issue #507"
fi

# Pins record the free-tier quota scope with banned paid routes.
if grep -q -F -e 'standard GitHub-hosted runners is free' "$pins" &&
  grep -q -F -e 'runs-on: ubuntu-latest' "$pins" &&
  grep -q -F -e 'runs-on: windows-latest' "$pins" &&
  grep -q -F -e 'actions/cache' "$pins" &&
  grep -q -F -e '10 GB' "$pins" &&
  grep -q -F -e '500 MB' "$pins" &&
  grep -q -F -e 'Larger runners are always charged' "$pins" &&
  grep -q -F -e 'self-hosted' "$pins" &&
  grep -q -F -e 'macos-latest' "$pins"; then
  ok
else
  bad "pins.bzl lost its free-tier quota pins under issue #507"
fi

# Pins record local-only remote with the else branch and no correctness claim.
if grep -q -F -e '--remote_cache' "$pins" &&
  grep -q -F -e '--remote_executor' "$pins" &&
  grep -q -F -e '--bes_backend' "$pins" &&
  grep -q -F -e 'local execution, no remote' "$pins" &&
  grep -q -F -e 'locally sandbox-tested but remote behavior remains unverified' "$pins" &&
  grep -q -F -e 'remote-cache correctness' "$pins"; then
  ok
else
  bad "pins.bzl lost its local-only remote pins under issue #507"
fi

# Pins record the rejected substitutes (seed-only forever stays rejected).
if grep -q -F -e '"seed-only forever"' "$pins" &&
  grep -q -F -e '"cross-cell union"' "$pins" &&
  grep -q -F -e '"averaged percentages"' "$pins" &&
  grep -q -F -e '"rounding up"' "$pins" &&
  grep -q -F -e '"required Codecov"' "$pins" &&
  grep -q -F -e '"remote-cache correctness"' "$pins"; then
  ok
else
  bad "pins.bzl lost its rejected-alternative pins under issue #507"
fi

# Fixture expected texts cover per-cell plus Codecov plus remote gaps.
if grep -q -F -e 'Six required cells' "$per_cell_expected" &&
  grep -q -F -e 'configurable --min-coverage requirement for users per-cell' "$per_cell_expected" &&
  grep -q -F -e 'behavioral matrix' "$per_cell_expected" &&
  grep -q -F -e 'Codecov stays at most opt-in' "$codecov_remote_expected" &&
  grep -q -F -e 'standard GitHub-hosted runners are free' "$codecov_remote_expected" &&
  grep -q -F -e 'remains unverified' "$codecov_remote_expected" &&
  grep -q -F -e 'platform plus consumer plus release evidence' "$per_cell_expected"; then
  ok
else
  bad "per_cell.expected plus codecov_remote.expected lost gap coverage (want cells plus decision plus Codecov plus quotas plus remote, issue #507)"
fi

# Docs own the qualified record with the fixture proof.
if grep -q -F -e 'tools/coverage/tests/fixtures/per_cell/pins.bzl' "$testing_readme" &&
  grep -q -F -e 'qualified by `bazel run //tools/ci:coverage_qualification`' "$testing_readme" &&
  grep -q -F -e 'tools/coverage/tests/fixtures/per_cell/pins.bzl' "$github_ci" &&
  grep -q -F -e 'configurable requirement for users per-cell' "$build_coverage_doc" &&
  grep -q -F -e 'tools/coverage/tests/fixtures/per_cell/pins.bzl' "$build_coverage_doc"; then
  ok
else
  bad "testing README, github-ci, or build-test-coverage lost its #507 pins fixture record"
fi

# Single exact gate plus approval: the repo verdict is one exact per-cell gate,
# the rate flag is user-facing only, and excludes need reviewer approval (issue #964).
if grep -q -F -e 'single exact gate' "$testing_readme" &&
  grep -q -F -e 'informational only' "$testing_readme" &&
  grep -q -F -e 'reviewer approval' "$testing_readme" &&
  grep -q -F -e 'single exact per-cell gate' docs/product/scope.md; then
  ok
else
  bad "testing README or scope lost its single exact gate plus reviewer-approval record (issue #964)"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/coverage/tests/fixtures/per_cell/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "per-cell coverage fixture failed to build (want green on the seed host, issue #507)"
fi

dx_test_summary "coverage qualification harness"
