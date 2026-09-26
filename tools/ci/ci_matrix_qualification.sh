#!/usr/bin/env bash
# CI multi-platform host-matrix qualification harness.
#
# Machine-checks the as-built lean CI matrix delivered with the shared
# BuildBuddy remote cache (issue #415 successors), with fixture evidence
# and owned gaps:
# - delivered: seed Linux x86_64 glibc plus Linux arm64 glibc plus the
#   two Linux static-musl profiles stay raw bazel cells in ci.yml, while
#   macOS arm64 plus Windows x86_64 MSVC-compatible native run inside
#   the consumer self-call (four platforms, full dx build plus dx test
#   plus dx coverage, no disabled checks); docs in support-matrix plus
#   ADR 0014 plus github-ci plus testing matrix; macOS x86_64 is Not
#   planned per #976 with no jobs;
# - qualification-assertion: Windows x86_64 joins Platform-qualified with
#   explicit EULA never automatic; a windows-latest runner with shell
#   bash is queued via the self-call ternary; out-of-v1 Windows arm64
#   plus Not-planned macOS x86_64 stay refused with clean refusal;
# - portable-shell reuse: shared tools/sh/lib.sh helpers plus
#   shellcheck/shfmt pins with no non-portable workflow forms;
# - sharding policy: one logical stage per job, a single aggregate
#   needs chain behind `if: always()`, per-job step summaries, no paid
#   services.
#
# Versioned here, run by CI via `bazel run //tools/ci:ci_matrix_qualification`,
# following //tools/ci:macos_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

ci=".github/workflows/ci.yml"
consumer=".github/workflows/reusable-consumer.yml"
platform="cli/cli/src/platform.rs"
cells="tools/coverage/cells.txt"
matrix="docs/product/support-matrix.md"
contract="docs/github-ci.md"
test_matrix="docs/testing/github-ci.md"
testing_readme="docs/testing/strategy-details.md"

# ci.yml header records the host matrix with the closed
# pointer plus per-host successors and the Windows qualification
# (joins Platform-qualified, Windows arm64 stays the remainder).
if grep -q -F -e 'Host matrix (issue #415' "$ci" &&
  grep -q -F -e 'issue #298 (closed; per-host successors own each host)' "$ci" &&
  grep -q -F -e 'Windows x86_64 MSVC-compatible native (windows-latest' "$ci" &&
  grep -q -F -e 'issue #414' "$ci"; then
  ok
else
  bad "ci.yml header lost the issue #415 host-matrix plus closed-#298 plus Windows-qualified record"
fi

# Per-host runners: seed plus musl x86_64 on ubuntu-latest, arm64 pair on
# ubuntu-24.04-arm in ci.yml, while macos-14 plus windows-latest run via
# the consumer self-call ternary (macOS x86_64 Not planned per #976, no
# macos-15-intel).
if grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e 'runs-on: ubuntu-24.04-arm' "$ci" &&
  grep -q -F -e 'macos-14' "$consumer" &&
  grep -q -F -e 'windows-latest' "$consumer" &&
  ! grep -q -F -e 'macos-15-intel' "$ci" &&
  ! grep -q -F -e 'macos-15-intel' "$consumer"; then
  ok
else
  bad "ci.yml plus reusable-consumer lost a per-host runner (ubuntu-latest plus ubuntu-24.04-arm in ci.yml, macos-14 plus windows-latest in the self-call; macos-15-intel removed per #976)"
fi

# Per-job cache wiring: every direct-run Bazel job configures the shared
# BuildBuddy remote cache (six configure steps in ci.yml) while the
# deleted disk-cache action plus per-host prefixes stay gone.
if [[ "$(grep -c -F -e 'name: Configure BuildBuddy remote cache' "$ci")" == "6" ]] &&
  dx_tree_absent 'restore-bazel-cache' -- .github/workflows/ &&
  dx_tree_absent 'prefix: bazel-' -- .github/workflows/; then
  ok
else
  bad "ci.yml lost its BuildBuddy configure steps or a deleted disk-cache reference resurfaced (want six configure steps, no restore-bazel-cache, no per-host prefixes)"
fi

# Job inventory: dogfood self-call plus the musl build/coverage pairs
# plus freshness plus devcontainer plus the aggregate gate (macOS plus
# Windows run only inside the reusable consumer; per-host raw triples
# left with the deleted jobs, x86_64 removed per #976).
if grep -q -F -e 'dogfood:' "$ci" &&
  grep -q -F -e 'build-musl-x86_64:' "$ci" &&
  grep -q -F -e 'build-musl-arm64:' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64:' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64:' "$ci" &&
  grep -q -F -e 'dogfood-freshness:' "$ci" &&
  grep -q -F -e 'devcontainer-check:' "$ci" &&
  grep -q -F -e 'if: ${{ always() }}' "$ci" &&
  ! grep -q -F -e 'build-arm64:' "$ci" &&
  ! grep -q -F -e 'build-macos-arm64:' "$ci" &&
  ! grep -q -F -e 'build-windows-x86_64:' "$ci"; then
  ok
else
  bad "ci.yml lost a lean-shape job (dogfood plus musl pairs plus freshness plus devcontainer plus aggregate; per-host triples removed per #415 successors)"
fi

# Consumer self-call runs the full stack on the four host platforms:
# dx build plus dx test plus dx coverage (min 97) with no disabled
# checks, and the supported set matches (issue #408 plus Phase 1 #607
# coverage superset, Windows joins under #414; x86_64 removed per #976).
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'min_coverage: "97"' "$ci" &&
  ! grep -q -F -e 'disabled_checks' "$ci" &&
  grep -q -F -e 'supported = {"linux_x86_64", "linux_arm64", "macos_arm64", "windows_x86_64"}' "$consumer"; then
  ok
else
  bad "ci.yml self-call lost full-tests plus four-platform plus no-corpus honesty (issue #408 plus Phase 1 #607, Windows joins under #414; x86_64 removed per #976)"
fi

# Qualified-host expectation: four hosts qualified (Windows joins under
# , macOS x86_64 Not planned per #976 with refusal)
# (hermetic context search, issue #1006).
if grep -q -F -e '("linux", "x86_64")' "$platform" &&
  grep -q -F -e '("linux", "aarch64")' "$platform" &&
  dx_context_contains "$platform" 'pub fn qualified_hosts' -A 8 '("macos", "aarch64")' &&
  ! dx_context_contains "$platform" 'pub fn qualified_hosts' -A 8 '("macos", "x86_64")' &&
  grep -q -F -e '("windows", "x86_64")' "$platform" &&
  grep -q -F -e 'windows_x86_64_host_is_qualified' "$platform" &&
  grep -q -F -e 'qualified seed-linux_x86_64' "$cells" &&
  grep -q -F -e 'qualified linux_arm64' "$cells" &&
  grep -q -F -e 'qualified linux_x86_64_musl' "$cells" &&
  grep -q -F -e 'qualified linux_arm64_musl' "$cells" &&
  grep -q -F -e 'qualified macos_arm64' "$cells" &&
  ! grep -q -F -e 'qualified macos_x86_64' "$cells" &&
  grep -q -F -e 'qualified windows_x86_64' "$cells"; then
  ok
else
  bad "qualified-host expectation lost (platform.rs four qualified, cells six qualified; x86_64 removed per #976)"
fi

# Windows qualification-assertion: support-matrix keeps Windows
# x86_64 Platform-qualified with explicit EULA never automatic; the
# self-call queues a windows-latest runner with shell bash; no paid
# runner appears.
if grep -E -e '^\| Windows x86_64 MSVC-compatible \|' "$matrix" | grep -q -F -e 'Platform-qualified (issue #414' &&
  grep -E -e 'runs-on:.*windows-latest' "$consumer" | grep -q . &&
  grep -q -F -e 'shell: bash' "$consumer" &&
  ! grep -E -e 'runs-on:.*(self-hosted|larger|macos-latest)' "$consumer" | grep -q .; then
  ok
else
  bad "Windows qualification-assertion lost (support-matrix Platform-qualified plus self-call windows-latest shell bash plus no paid runner, issue #414)"
fi

# Portable-shell reuse: shared lib helpers plus
# shellcheck/shfmt pins, and the prove shell_contract step stays wired.
# Note: the timing pattern below is split to avoid the literal definition
# form (shell_contract requires it lives once in tools/sh/lib.sh).
if grep -q -F -e 'dx_realpath() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_sha256_file() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_replace() {' tools/sh/lib.sh &&
  grep -q -F -e 'dx_mkscratch' tools/sh/lib.sh &&
  grep -q -F -e 'dx_test_init' tools/sh/lib.sh &&
  grep -q -F -e 'dx_now_''secs() {' tools/sh/lib.sh &&
  grep -q -F -e 'shell=bash' .shellcheckrc &&
  grep -q -F -e 'shfmt -i 2 -ci' .shellcheckrc &&
  grep -q -F -e '//tools/ci:shell_contract' "$ci"; then
  ok
else
  bad "portable-shell reuse lost (lib.sh helpers plus shellcheck/shfmt plus shell_contract wiring, issue #323)"
fi

# ci.yml stays portable-shell clean: no non-portable forms in the workflow.
# Note: the sed pattern below is split to avoid a literal banned form in
# this file (shell_contract bans sed -i in executable code).
if ! grep -e 'realpath' "$ci" | grep -v -F -e 'dx_realpath' | grep -q . &&
  ! grep -e 'sed -''i' "$ci" | grep -q . &&
  ! grep -e 'cp -''a' "$ci" | grep -q . &&
  ! grep -F -e 'EPOCHREALTIME' "$ci" | grep -q . &&
  ! grep -F -e 'sha256sum' "$ci" | grep -q .; then
  ok
else
  bad "ci.yml introduced a non-portable shell form (issue #323)"
fi

# Sharding policy: one logical stage per job with exactly one needs
# chain, the aggregate gate behind if: always() (skipped needs are
# non-failing). The per-host fast-fail chains left with the deleted
# triples (hermetic context search: host grep -A separators diverge,
# issue #1006).
if [[ "$(grep -c -F -e 'needs:' "$ci")" == "1" ]] &&
  DX_CONTEXT_ANCHOR_RE=1 dx_context_contains "$ci" '^    needs:$' -A 10 'devcontainer-check,' &&
  grep -q -F -e 'if: ${{ always() }}' "$ci" &&
  dx_tree_absent 'needs: [build' -- .github/workflows/; then
  ok
else
  bad "ci.yml lost its single aggregate needs chain plus always-gate (issue #210)"
fi

# Every job writes a step summary block; concurrency cancels superseded runs.
if [[ "$(grep -c -F -e 'GITHUB_STEP_SUMMARY' "$ci")" -ge "16" ]] &&
  grep -q -F -e 'cancel-in-progress: true' "$ci" &&
  grep -q -F -e 'bazel build --noshow_progress' "$ci" &&
  grep -q -F -e 'bazel test --noshow_progress' "$ci"; then
  ok
else
  bad "ci.yml lost per-job summaries, concurrency cancel, or --noshow_progress (issue #210)"
fi

# No paid services: standard runners only, with the free-tier budget
# recorded in the testing README. windows-latest is a standard free
# runner qualified via the self-call; macos-latest stays banned
# (unpinned), as do self-hosted/larger (paid)
# (hermetic tree search: host grep -rn variance, issue #1006).
if DX_TREE_RE=1 dx_tree_absent 'runs-on:.*(self-hosted|larger|macos-latest)' -- .github/workflows/ &&
  grep -q -F -e 'windows-latest' "$consumer" &&
  grep -q -F -e 'standard GitHub-hosted runners is free' "$testing_readme"; then
  ok
else
  bad "workflows gained a paid runner or lost the free-tier budget record"
fi

# Stale references updated in place: contract plus test matrix plus
# support-matrix consumer note name the matrix (full-tests self-call,
# Windows joins).
if grep -q -F -e 'issue #415' "$contract" &&
  grep -q -F -e 'issue #415' "$test_matrix" &&
  grep -q -F -e 'consumer self-call: dx test plus dx coverage' "$matrix"; then
  ok
else
  bad "stale references lost the issue #415 plus full-tests self-call record (contract plus test matrix plus support-matrix consumer note)"
fi

# No Supported claim for the matrix: Platform-qualified only (Windows joins
# qualified; macOS x86_64 Not planned per #976), release evidence open.
if grep -E -e '^\| Linux arm64 glibc \|' "$matrix" | grep -q -F -e 'Platform-qualified' &&
  grep -E -e '^\| macOS x86_64 \|' "$matrix" | grep -q -F -e 'Not planned' &&
  grep -E -e '^\| Windows x86_64 MSVC-compatible \|' "$matrix" | grep -q -F -e 'Platform-qualified' &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q .; then
  ok
else
  bad "matrix rows lost their Platform-qualified plus Not-planned (never Supported) records (Windows joins qualified under #414; x86_64 Not planned per #976)"
fi

dx_test_summary "ci multi-platform matrix qualification harness"
