#!/usr/bin/env bash
# CI multi-platform host-matrix qualification harness.
#
# Machine-checks the as-built multi-host CI matrix delivered after the
# portable-shell contract (closed) with fixture evidence and
# owned gaps, with Windows x86_64 
# - delivered: per-host jobs parameterized by runner plus cache scope plus
#   qualified-host expectation for the seed Linux x86_64 glibc host plus
# Linux arm64 native plus the two Linux static-musl
# profiles plus macOS arm64 native plus macOS
# x86_64 best-effort native (non-blocking) plus Windows
# x86_64 MSVC-compatible native, consumer
# self-call test-disabled on the five host platforms
# Phase 1 coverage superset), docs in
#   support-matrix plus ADR 0014 plus github-ci plus testing matrix;
# - qualification-assertion: Windows x86_64 joins Platform-qualified with
# explicit EULA never automatic; a windows-latest runner
#   with shell bash is queued; out-of-v1 Windows arm64 stays the
#   unqualified gap with clean refusal;
# - portable-shell reuse: shared tools/sh/lib.sh helpers plus
#   shellcheck/shfmt pins with no non-portable workflow forms;
# - sharding policy: one logical stage per job, fast-fail
#   needs chains, per-job step summaries, no paid services.
#
# Versioned here, run by CI via `bazel run //tools/ci:ci_matrix_qualification`,
# following //tools/ci:macos_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

ci=".github/workflows/ci.yml"
consumer=".github/workflows/reusable-consumer.yml"
platform="cli/cli/src/platform.rs"
cells="tools/coverage/cells.txt"
matrix="docs/product/support-matrix.md"
contract="docs/github-ci.md"
test_matrix="docs/testing/github-ci.md"
verify="docs/testing/verification-matrix.md"
testing_readme="docs/testing/README.md"

# ci.yml header records the host matrix with the closed
# pointer plus per-host successors and the Windows qualification
# (joins Platform-qualified, Windows arm64 stays the remainder).
if grep -q -F -e 'Host matrix (issue #415' "$ci" &&
  grep -q -F -e 'issue #298 (closed; per-host successors own each host)' "$ci" &&
  grep -q -F -e 'Windows x86_64 MSVC-compatible native (windows-latest with shell bash,' "$ci" &&
  grep -q -F -e 'issue #414' "$ci"; then
  ok
else
  bad "ci.yml header lost the issue #415 host-matrix plus closed-#298 plus Windows-qualified record"
fi

# Per-host runners: seed plus musl x86_64 on ubuntu-latest, arm64 pair on
# ubuntu-24.04-arm, macos arm64 on macos-14, macos x86_64 best-effort on
# macos-15-intel, windows x86_64 on windows-latest
#
if grep -q -F -e 'runs-on: ubuntu-latest' "$ci" &&
  grep -q -F -e 'runs-on: ubuntu-24.04-arm' "$ci" &&
  grep -q -F -e 'runs-on: macos-14' "$ci" &&
  grep -q -F -e 'runs-on: macos-15-intel' "$ci" &&
  grep -q -F -e 'runs-on: windows-latest' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host runner (ubuntu-latest plus ubuntu-24.04-arm plus macos-14 plus macos-15-intel plus windows-latest)"
fi

# Per-host cache scopes: seed plus arm64 plus per-profile musl plus
# per-host macos plus windows, all free-tier actions/cache
#
if grep -q -F -e 'bazel-seed-' "$ci" &&
  grep -q -F -e 'bazel-arm64-' "$ci" &&
  grep -q -F -e 'bazel-musl-x86_64-' "$ci" &&
  grep -q -F -e 'bazel-musl-arm64-' "$ci" &&
  grep -q -F -e 'bazel-macos-arm64-' "$ci" &&
  grep -q -F -e 'bazel-macos-x86_64-' "$ci" &&
  grep -q -F -e 'bazel-windows-x86_64-' "$ci" &&
  grep -q -F -e 'actions/cache' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host Bazel disk-cache scope (seed plus arm64 plus musl pair plus macos pair plus windows)"
fi

# Per-host jobs exist: seed build/test/coverage plus arm64 triple plus
# musl build/coverage pairs plus macos arm64/x86_64 triples plus windows
# x86_64 triple.
if grep -q -F -e 'build-arm64 (bazel build' "$ci" &&
  grep -q -F -e 'test-arm64 (bazel test' "$ci" &&
  grep -q -F -e 'coverage-arm64 (dx coverage gate, arm64 cell)' "$ci" &&
  grep -q -F -e 'build-musl-x86_64' "$ci" &&
  grep -q -F -e 'build-musl-arm64' "$ci" &&
  grep -q -F -e 'coverage-musl-x86_64' "$ci" &&
  grep -q -F -e 'coverage-musl-arm64' "$ci" &&
  grep -q -F -e 'build-macos-arm64' "$ci" &&
  grep -q -F -e 'test-macos-arm64' "$ci" &&
  grep -q -F -e 'coverage-macos-arm64' "$ci" &&
  grep -q -F -e 'build-macos-x86_64' "$ci" &&
  grep -q -F -e 'test-macos-x86_64' "$ci" &&
  grep -q -F -e 'coverage-macos-x86_64' "$ci" &&
  grep -q -F -e 'build-windows-x86_64' "$ci" &&
  grep -q -F -e 'test-windows-x86_64' "$ci" &&
  grep -q -F -e 'coverage-windows-x86_64' "$ci"; then
  ok
else
  bad "ci.yml lost a per-host job (arm64 triple plus musl pairs plus macos triples plus windows triple)"
fi

# Consumer self-call runs test-disabled on the five host platforms
#
# superset: `dx coverage` via `resolve_for_test` plus `bazel coverage`
# executes the tests; Windows x86_64 joins).
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"macos_x86_64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'disabled_checks: "test"' "$ci" &&
  grep -q -F -e 'supported = {"linux_x86_64", "linux_arm64", "macos_arm64", "macos_x86_64", "windows_x86_64"}' "$consumer"; then
  ok
else
  bad "ci.yml self-call lost test-disabled plus five-platform plus no-corpus honesty (issue #408 plus Phase 1 #607, Windows joins under #414)"
fi

# Qualified-host expectation: five hosts qualified (Windows joins under
# , no refusal).
if grep -q -F -e '("linux", "x86_64")' "$platform" &&
  grep -q -F -e '("linux", "aarch64")' "$platform" &&
  grep -q -F -e '("macos", "aarch64")' "$platform" &&
  grep -q -F -e '("macos", "x86_64")' "$platform" &&
  grep -q -F -e '("windows", "x86_64")' "$platform" &&
  grep -q -F -e 'windows_x86_64_host_is_qualified' "$platform" &&
  grep -q -F -e 'qualified seed-linux_x86_64' "$cells" &&
  grep -q -F -e 'qualified linux_arm64' "$cells" &&
  grep -q -F -e 'qualified linux_x86_64_musl' "$cells" &&
  grep -q -F -e 'qualified linux_arm64_musl' "$cells" &&
  grep -q -F -e 'qualified macos_arm64' "$cells" &&
  grep -q -F -e 'qualified macos_x86_64' "$cells" &&
  grep -q -F -e 'qualified windows_x86_64' "$cells"; then
  ok
else
  bad "qualified-host expectation lost (platform.rs five qualified, cells seven qualified under #414)"
fi

# Windows qualification-assertion: support-matrix keeps Windows
# x86_64 Platform-qualified with explicit EULA never automatic; a
# windows-latest runner with shell bash is queued; no paid runner appears.
if grep -E -e '^\| Windows x86_64 MSVC-compatible \|' "$matrix" | grep -q -F -e 'Platform-qualified (issue #414' &&
  grep -E -e 'runs-on:.*windows-latest' "$ci" | grep -q . &&
  grep -q -F -e 'shell: bash' "$ci" &&
  ! grep -E -e 'runs-on:.*(self-hosted|larger|macos-latest)' "$ci" | grep -q .; then
  ok
else
  bad "Windows qualification-assertion lost (support-matrix Platform-qualified plus windows-latest shell bash plus no paid runner, issue #414)"
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

# Sharding policy: fast-fail needs chains per host family
# plus prove/dogfood on the seed build.
if grep -A3 -e '^  test:' "$ci" | grep -q -F -e 'needs: [build]' &&
  grep -A3 -e '^  coverage:' "$ci" | grep -q -F -e 'needs: [build]' &&
  grep -A3 -e '^  test-arm64:' "$ci" | grep -q -F -e 'needs: [build-arm64]' &&
  grep -A3 -e '^  coverage-arm64:' "$ci" | grep -q -F -e 'needs: [build-arm64]' &&
  grep -A3 -e '^  coverage-musl-x86_64:' "$ci" | grep -q -F -e 'needs: [build-musl-x86_64]' &&
  grep -A3 -e '^  coverage-musl-arm64:' "$ci" | grep -q -F -e 'needs: [build-musl-arm64]' &&
  grep -A3 -e '^  test-macos-arm64:' "$ci" | grep -q -F -e 'needs: [build-macos-arm64]' &&
  grep -A3 -e '^  coverage-macos-arm64:' "$ci" | grep -q -F -e 'needs: [build-macos-arm64]' &&
  grep -A3 -e '^  test-macos-x86_64:' "$ci" | grep -q -F -e 'needs: [build-macos-x86_64]' &&
  grep -A3 -e '^  coverage-macos-x86_64:' "$ci" | grep -q -F -e 'needs: [build-macos-x86_64]' &&
  grep -A3 -e '^  test-windows-x86_64:' "$ci" | grep -q -F -e 'needs: [build-windows-x86_64]' &&
  grep -A3 -e '^  coverage-windows-x86_64:' "$ci" | grep -q -F -e 'needs: [build-windows-x86_64]' &&
  grep -A3 -e '^  prove:' "$ci" | grep -q -F -e 'needs: [build]' &&
  grep -A3 -e '^  dogfood-freshness:' "$ci" | grep -q -F -e 'needs: [build]'; then
  ok
else
  bad "ci.yml lost its sharding fast-fail needs chains (issue #210)"
fi

# Every job writes a step summary block; concurrency cancels superseded runs.
if [[ "$(grep -c -F -e 'GITHUB_STEP_SUMMARY' "$ci")" -ge "18" ]] &&
  grep -q -F -e 'cancel-in-progress: true' "$ci" &&
  grep -q -F -e 'bazel build --noshow_progress' "$ci" &&
  grep -q -F -e 'bazel test --noshow_progress' "$ci"; then
  ok
else
  bad "ci.yml lost per-job summaries, concurrency cancel, or --noshow_progress (issue #210)"
fi

# No paid services: standard runners plus actions/cache only, with the
# free-tier budget recorded in the testing README. windows-latest is a
# standard free runner qualified; macos-latest stays
# banned (unpinned), as do self-hosted/larger (paid).
if ! grep -rn -E -e 'runs-on:.*(self-hosted|larger|macos-latest)' .github/workflows/ 2>/dev/null | grep -q . &&
  grep -q -F -e 'runs-on: windows-latest' "$ci" &&
  grep -q -F -e 'standard GitHub-hosted runners is free' "$testing_readme"; then
  ok
else
  bad "workflows gained a paid runner or lost the free-tier budget record"
fi

# Stale references updated in place: contract plus test matrix plus
# verification matrix plus support-matrix consumer note name the matrix
# (Windows joins).
if grep -q -F -e 'issue #415' "$contract" &&
  grep -q -F -e 'issue #415' "$test_matrix" &&
  grep -q -F -e 'issue #415' "$verify" &&
  grep -q -F -e 'consumer self-call: test-disabled' "$matrix"; then
  ok
else
  bad "stale references lost the issue #415 matrix record (contract plus test matrix plus verification plus support-matrix consumer note)"
fi

# No Supported claim for the matrix: Platform-qualified only (Windows joins
# qualified), release evidence open.
if grep -E -e '^\| Linux arm64 glibc \|' "$matrix" | grep -q -F -e 'Platform-qualified' &&
  grep -E -e '^\| macOS x86_64 \|' "$matrix" | grep -q -F -e 'Platform-qualified' &&
  grep -E -e '^\| Windows x86_64 MSVC-compatible \|' "$matrix" | grep -q -F -e 'Platform-qualified' &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q .; then
  ok
else
  bad "matrix rows lost their Platform-qualified (never Supported) records (Windows joins qualified under #414)"
fi

dx_test_summary "ci multi-platform matrix qualification harness"
