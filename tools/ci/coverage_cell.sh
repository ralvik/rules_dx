#!/usr/bin/env bash
# Seed-cell coverage gate harness (item 1).
#
# Every required configuration/platform cell gates its own combined LCOV
# report: no cross-platform union, no averaged percentages, no rounding
# up. The seed host plus Linux arm64 native plus the two
# Linux static-musl profiles plus macOS arm64 native (issue
# plus macOS x86_64 best-effort native (non-blocking)
# plus Windows x86_64 MSVC-compatible native qualify;
# this harness wires the seed cell end to end through the versioned
# inventory at tools/coverage/seed-inventory.txt and the `coverage_bin`
# gate CLI (the arm64 twin gates tools/coverage/arm64-inventory.txt in
# the CI `coverage-arm64` job, the musl twins gate
# tools/coverage/musl-*-inventory.txt in the CI `coverage-musl-*` jobs,
# the macOS arm64 twin gates tools/coverage/macos-arm64-inventory.txt
# in the CI `coverage-macos-arm64` job on `macos-14`, the macOS x86_64
# best-effort twin gates tools/coverage/macos-x86_64-inventory.txt in the
# CI `coverage-macos-x86_64` job on `macos-15-intel`, and the Windows x86_64
# twin gates tools/coverage/windows-x86_64-inventory.txt in the CI
# `coverage-windows-x86_64` job on `windows-latest`
# against the same scope):
# - the real scoped `bazel coverage` report passes the real gate,
# - mutated inputs fail closed (missing report, uninventoried source,
#   undeclared eligible source, uncovered line with location,
#   malformed exclusion directive, exclusion without nearby reason),
# - the //... rate gate step is still present in CI.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_cell`,
# following //tools/ci:target_tags.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

check_bin="bazel-bin/tools/coverage/coverage_bin"
inventory="tools/coverage/seed-inventory.txt"
dx_mkscratch scratch

# `bazel coverage` above leaves `coverage_bin` instrumented
# (`-C instrument-coverage`), and the harness executes it directly 7x from
# the workspace root. With `LLVM_PROFILE_FILE` unset, Rust writes
# `default_%m_%p.profraw` to CWD per invocation. Redirect profiles into the
# auto-cleaned scratch dir so harness runs spill nothing to the checkout.
export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"

# The gate binary must exist (built by the build job / on demand).
if [[ ! -x "$check_bin" ]]; then
  bazel build --noshow_progress //tools/coverage:coverage_bin >/dev/null 2>&1
fi

# Real scoped coverage for the seed inventory scope, then the real gate.
# The gate library lives at //cli/lcov:dx_lcov since the promotion;
# the thin binary shim stays at //tools/coverage:coverage_bin. Both scopes
# are covered together so the inventory (library + shim) matches Bazel.
bazel coverage --noshow_progress //cli/lcov/... //tools/coverage/... \
  --combined_report=lcov \
  --instrumentation_filter='^//(cli/lcov|tools/coverage)' >/dev/null 2>&1
bazel query 'kind("source file", deps(//cli/lcov/... + //tools/coverage/...))' 2>/dev/null |
  grep -E '^//(cli/lcov|tools/coverage)[:/].*\.rs$' |
  sed -e 's|^//||' -e 's|:|/|' |
  LC_ALL=C sort -u >"$scratch/sources.txt"
gate_rc=0
gate_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$inventory" --sources "$scratch/sources.txt" --root . 2>&1)" || gate_rc=$?
if [[ "$gate_rc" == "0" ]] && echo "$gate_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "seed cell gate did not pass: rc=$gate_rc out=$gate_out"
fi

# The arm64 cell gates the same first-party scope: the same
# real report passes the arm64 inventory, so all seven versioned
# inventories stay functionally in sync (the CI coverage-arm64 job gates
# the arm64 runner's own report per-cell with no union).
arm64_rc=0
arm64_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/arm64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || arm64_rc=$?
if [[ "$arm64_rc" == "0" ]] && echo "$arm64_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "arm64 cell gate did not pass: rc=$arm64_rc out=$arm64_out"
fi

# The static-musl cells gate the same first-party scope:
# the same real report passes both musl inventories, so all seven
# versioned inventories stay functionally in sync (the CI
# coverage-musl-x86_64 plus coverage-musl-arm64 jobs gate their own
# runners' reports per-cell with no union; dynamic musl stays out of
# scope and has no inventory).
musl_x86_rc=0
musl_x86_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/musl-x86_64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || musl_x86_rc=$?
if [[ "$musl_x86_rc" == "0" ]] && echo "$musl_x86_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "musl x86_64 cell gate did not pass: rc=$musl_x86_rc out=$musl_x86_out"
fi
musl_arm64_rc=0
musl_arm64_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/musl-arm64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || musl_arm64_rc=$?
if [[ "$musl_arm64_rc" == "0" ]] && echo "$musl_arm64_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "musl arm64 cell gate did not pass: rc=$musl_arm64_rc out=$musl_arm64_out"
fi

# The macOS arm64 cell gates the same first-party scope:
# the same real report passes the macos-arm64 inventory, so all seven
# versioned inventories stay functionally in sync (the CI
# coverage-macos-arm64 job gates its own macos-14 runner report per-cell
# with no union; host-installed SDK fallback is never approved).
macos_arm64_rc=0
macos_arm64_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/macos-arm64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || macos_arm64_rc=$?
if [[ "$macos_arm64_rc" == "0" ]] && echo "$macos_arm64_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "macos arm64 cell gate did not pass: rc=$macos_arm64_rc out=$macos_arm64_out"
fi

# The macOS x86_64 best-effort cell gates the same first-party scope
# (non-blocking): the same real report passes the
# macos-x86_64 inventory, so all seven versioned inventories stay
# functionally in sync (the CI coverage-macos-x86_64 job gates its own
# macos-15-intel runner report per-cell with no union; gaps never block
# required-host release; host-installed SDK fallback is never approved).
macos_x86_64_rc=0
macos_x86_64_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/macos-x86_64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || macos_x86_64_rc=$?
if [[ "$macos_x86_64_rc" == "0" ]] && echo "$macos_x86_64_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "macos x86_64 cell gate did not pass: rc=$macos_x86_64_rc out=$macos_x86_64_out"
fi

# The Windows x86_64 cell gates the same first-party scope:
# the same real report passes the windows-x86_64 inventory, so all seven
# versioned inventories stay functionally in sync (the CI
# coverage-windows-x86_64 job gates its own windows-latest runner report
# per-cell with no union; toolchains_msvc backend stays provisional with
# explicit EULA acceptance never automatic, installed Build Tools fallback
# never approved).
windows_x86_64_rc=0
windows_x86_64_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory tools/coverage/windows-x86_64-inventory.txt --sources "$scratch/sources.txt" --root . 2>&1)" || windows_x86_64_rc=$?
if [[ "$windows_x86_64_rc" == "0" ]] && echo "$windows_x86_64_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "windows x86_64 cell gate did not pass: rc=$windows_x86_64_rc out=$windows_x86_64_out"
fi

# Missing report file fails the gate (exit 1), never a usage error.
missing_rc=0
missing_out="$("$check_bin" --report "$scratch/absent.lcov" \
  --inventory "$inventory" --sources "$scratch/sources.txt" --root . 2>&1)" || missing_rc=$?
if [[ "$missing_rc" == "1" ]] && echo "$missing_out" | grep -q 'missing report file'; then
  ok
else
  bad "missing report did not fail closed: rc=$missing_rc out=$missing_out"
fi

# An instrumented source dropped from the inventory fails the gate.
grep -v -F -e 'tools/coverage/src/main.rs' "$inventory" >"$scratch/no-main-inventory.txt"
uninventoried_rc=0
uninventoried_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$scratch/no-main-inventory.txt" --sources "$scratch/sources.txt" \
  --root . 2>&1)" || uninventoried_rc=$?
if [[ "$uninventoried_rc" == "1" ]] && echo "$uninventoried_out" | grep -q 'not in inventory'; then
  ok
else
  bad "uninventoried source did not fail closed: rc=$uninventoried_rc out=$uninventoried_out"
fi

# An eligible inventory entry dropped from the Bazel source list fails.
grep -v -F -e 'cli/lcov/src/lib.rs' "$scratch/sources.txt" >"$scratch/no-lib-sources.txt"
undeclared_rc=0
undeclared_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$inventory" --sources "$scratch/no-lib-sources.txt" --root . 2>&1)" || undeclared_rc=$?
if [[ "$undeclared_rc" == "1" ]] && echo "$undeclared_out" | grep -q 'not declared by Bazel'; then
  ok
else
  bad "undeclared eligible source did not fail closed: rc=$undeclared_rc out=$undeclared_out"
fi

# A zero-hit eligible line fails with its location, not just a rate dip.
printf 'SF:cli/lcov/src/lib.rs\nDA:1,0\nend_of_record\n' >"$scratch/uncovered.lcov"
printf 'eligible cli/lcov/src/lib.rs\n' >"$scratch/uncovered-inventory.txt"
printf 'cli/lcov/src/lib.rs\n' >"$scratch/uncovered-sources.txt"
uncovered_rc=0
uncovered_out="$("$check_bin" --report "$scratch/uncovered.lcov" \
  --inventory "$scratch/uncovered-inventory.txt" --sources "$scratch/uncovered-sources.txt" \
  --root . 2>&1)" || uncovered_rc=$?
if [[ "$uncovered_rc" == "1" ]] && echo "$uncovered_out" | grep -q 'uncovered: cli/lcov/src/lib.rs:1'; then
  ok
else
  bad "uncovered line did not fail with location: rc=$uncovered_rc out=$uncovered_out"
fi

# A malformed exclusion directive fails closed with its location.
mkdir -p "$scratch/badroot"
printf 'fn f() {}\n// LCOV_EXCL_RANGE - reason: typo.\n' >"$scratch/badroot/bad.rs"
printf 'SF:bad.rs\nDA:1,1\nend_of_record\n' >"$scratch/bad.lcov"
printf 'eligible bad.rs\n' >"$scratch/bad-inventory.txt"
printf 'bad.rs\n' >"$scratch/bad-sources.txt"
bad_rc=0
bad_out="$("$check_bin" --report "$scratch/bad.lcov" \
  --inventory "$scratch/bad-inventory.txt" --sources "$scratch/bad-sources.txt" \
  --root "$scratch/badroot" 2>&1)" || bad_rc=$?
if [[ "$bad_rc" == "1" ]] && echo "$bad_out" | grep -q 'unrecognized'; then
  ok
else
  bad "malformed exclusion did not fail closed: rc=$bad_rc out=$bad_out"
fi

# An exclusion without a nearby reason fails closed.
mkdir -p "$scratch/noreasonroot"
printf 'fn f() {}\n// LCOV_EXCL_LINE\n' >"$scratch/noreasonroot/noreason.rs"
printf 'SF:noreason.rs\nDA:1,1\nend_of_record\n' >"$scratch/noreason.lcov"
printf 'eligible noreason.rs\n' >"$scratch/noreason-inventory.txt"
printf 'noreason.rs\n' >"$scratch/noreason-sources.txt"
noreason_rc=0
noreason_out="$("$check_bin" --report "$scratch/noreason.lcov" \
  --inventory "$scratch/noreason-inventory.txt" --sources "$scratch/noreason-sources.txt" \
  --root "$scratch/noreasonroot" 2>&1)" || noreason_rc=$?
if [[ "$noreason_rc" == "1" ]] && echo "$noreason_out" | grep -q 'reason'; then
  ok
else
  bad "reason-less exclusion did not fail closed: rc=$noreason_rc out=$noreason_out"
fi

# The //... rate gate still guards the whole tree in CI.
if grep -q -F -e 'coverage --min-coverage' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the dx coverage --min-coverage rate gate"
fi

dx_test_summary "coverage cell harness"
