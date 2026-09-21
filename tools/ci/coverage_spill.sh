#!/usr/bin/env bash
# Coverage-spill containment harness.
#
# `tools/ci/coverage_cell.sh` runs `bazel coverage`, which
# leaves `bazel-bin/tools/coverage/coverage_bin` instrumented
# (`-C instrument-coverage`), then executes it directly from the workspace
# root with `LLVM_PROFILE_FILE` unset. Rust defaults to
# `default_%m_%p.profraw` in CWD, spilling one file per invocation.
# `.gitignore` only hides the spill; this harness pins the root fix.
# `tools/ci/coverage_qualification.sh` plus
# `tools/ci/lcov_accounting_qualification.sh` execute the same instrumented
# helper directly (2x plus 4x); they need the same containment (issue #656).
#
# This harness machine-checks the containment half verifiable on a clean
# tree today: the LLVM_PROFILE_FILE export targets the
# auto-cleaned scratch dir before the first helper execution, the trap
# still cleans scratch, the gate invocations are intact (not deleted to
# fake containment), the gitignore defense-in-depth stays, the shared
# library default contains every harness at source time (issue #953), and the
# seed inventory still exists. It proves
# containment statically; the full `coverage_cell` run remains the
# end-to-end proof under.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_spill`,
# following //tools/ci:coverage_cell.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

cell="tools/ci/coverage_cell.sh"
qual="tools/ci/coverage_qualification.sh"
accounting="tools/ci/lcov_accounting_qualification.sh"

# The harness script exists.
if [[ -f "$cell" ]]; then
  ok
else
  bad "coverage cell harness missing: $cell"
fi

# LLVM_PROFILE_FILE redirects instrumented-helper profiles into scratch,
# preserving the module (%m) and pid (%p) discriminators.
if grep -q -F -e 'export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"' "$cell"; then
  ok
else
  bad "coverage_cell.sh lost the LLVM_PROFILE_FILE scratch export (issue #252)"
fi

# The export lands before the first helper execution, not after the spill.
export_line="$(grep -n -F -e 'export LLVM_PROFILE_FILE=' "$cell" | head -n 1 | cut -d: -f1)"
first_run_line="$(grep -n -F -e '"$check_bin"' "$cell" | head -n 1 | cut -d: -f1)"
if [[ -n "$export_line" && -n "$first_run_line" && "$export_line" -lt "$first_run_line" ]]; then
  ok
else
  bad "LLVM_PROFILE_FILE export is not ordered before the first \$check_bin run"
fi

# Scratch is still auto-cleaned so redirected profiles never accumulate.
# Dedup: coverage_cell.sh uses tools/sh/lib.sh dx_mkscratch
# (EXIT auto-cleanup) instead of a per-file mktemp+trap copy.
if grep -q -F -e 'dx_mkscratch scratch' "$cell"; then
  ok
else
  bad "coverage_cell.sh lost the auto-cleaned scratch dir (want dx_mkscratch, issue #323)"
fi

# Gate invocations stay intact: containment must not delete the 7 helper
# executions to fake a clean tree (static count of "$check_bin" runs).
runs="$(grep -c -F -e '"$check_bin"' "$cell" || true)"
if [[ "$runs" -ge 7 ]]; then
  ok
else
  bad "coverage_cell.sh lost helper invocations (found $runs, want >= 7)"
fi

# The coverage step still instruments the seed scope (the fix redirects
# profiles, it never weakens coverage collection itself).
if grep -q -F -e 'bazel coverage' "$cell" &&
  grep -q -F -e "instrumentation_filter" "$cell"; then
  ok
else
  bad "coverage_cell.sh lost the instrumented bazel coverage step"
fi

# Defense in depth stays: gitignored profraw/profdata so any future
# spill stays out of `git status` even as generation is now contained.
if grep -q -F -e '*.profraw' .gitignore && grep -q -F -e '*.profdata' .gitignore; then
  ok
else
  bad ".gitignore lost the *.profraw/*.profdata defense-in-depth"
fi

# Sibling wrappers need the same containment: coverage_qualification.sh
# executes the instrumented helper 2x from the workspace root (issue #656).
if [[ -f "$qual" ]]; then
  ok
else
  bad "coverage qualification harness missing: $qual"
fi

if grep -q -F -e 'export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"' "$qual"; then
  ok
else
  bad "coverage_qualification.sh lost the LLVM_PROFILE_FILE scratch export (issue #656)"
fi

qual_export_line="$(grep -n -F -e 'export LLVM_PROFILE_FILE=' "$qual" | head -n 1 | cut -d: -f1)"
qual_first_run_line="$(grep -n -F -e '"$check_bin" --report' "$qual" | head -n 1 | cut -d: -f1)"
if [[ -n "$qual_export_line" && -n "$qual_first_run_line" && "$qual_export_line" -lt "$qual_first_run_line" ]]; then
  ok
else
  bad "coverage_qualification.sh LLVM_PROFILE_FILE export is not ordered before the first \$check_bin run"
fi

if grep -q -F -e 'dx_mkscratch scratch' "$qual"; then
  ok
else
  bad "coverage_qualification.sh lost the auto-cleaned scratch dir (want dx_mkscratch, issue #323)"
fi

qual_runs="$(grep -c -F -e '"$check_bin" --report' "$qual" || true)"
if [[ "$qual_runs" -ge 2 ]]; then
  ok
else
  bad "coverage_qualification.sh lost helper invocations (found $qual_runs, want >= 2)"
fi

# Sibling wrappers need the same containment:
# lcov_accounting_qualification.sh executes the instrumented helper 4x
# from the workspace root (issue #656).
if [[ -f "$accounting" ]]; then
  ok
else
  bad "lcov accounting harness missing: $accounting"
fi

if grep -q -F -e 'export LLVM_PROFILE_FILE="$scratch/profraw_%m_%p.profraw"' "$accounting"; then
  ok
else
  bad "lcov_accounting_qualification.sh lost the LLVM_PROFILE_FILE scratch export (issue #656)"
fi

accounting_export_line="$(grep -n -F -e 'export LLVM_PROFILE_FILE=' "$accounting" | head -n 1 | cut -d: -f1)"
accounting_first_run_line="$(grep -n -F -e '"$check_bin" --report' "$accounting" | head -n 1 | cut -d: -f1)"
if [[ -n "$accounting_export_line" && -n "$accounting_first_run_line" && "$accounting_export_line" -lt "$accounting_first_run_line" ]]; then
  ok
else
  bad "lcov_accounting_qualification.sh LLVM_PROFILE_FILE export is not ordered before the first \$check_bin run"
fi

if grep -q -F -e 'dx_mkscratch scratch' "$accounting"; then
  ok
else
  bad "lcov_accounting_qualification.sh lost the auto-cleaned scratch dir (want dx_mkscratch, issue #323)"
fi

accounting_runs="$(grep -c -F -e '"$check_bin" --report' "$accounting" || true)"
if [[ "$accounting_runs" -ge 4 ]]; then
  ok
else
  bad "lcov_accounting_qualification.sh lost helper invocations (found $accounting_runs, want >= 4)"
fi

# Live containment: no profraw/profdata spill in the checkout root.
# Generation is contained via LLVM_PROFILE_FILE above; any file here is a
# regression even though .gitignore hides it from `git status` (issue #656).
if compgen -G "*.profraw" >/dev/null || compgen -G "*.profdata" >/dev/null; then
  bad "profraw/profdata spill present in checkout root (want LLVM_PROFILE_FILE containment, issue #656)"
else
  ok
fi

# Local default containment (issue #953): tools/sh/lib.sh defaults
# LLVM_PROFILE_FILE to an auto-cleaned scratch dir at source time, so every
# harness is contained even before its explicit export above.
if grep -q -F -e 'export LLVM_PROFILE_FILE="$_DX_PROFRAW_DIR/profraw_%m_%p.profraw"' tools/sh/lib.sh &&
  grep -q -F -e 'dx_mkscratch _DX_PROFRAW_DIR' tools/sh/lib.sh; then
  ok
else
  bad "tools/sh/lib.sh lost the default LLVM_PROFILE_FILE scratch containment (issue #953)"
fi

# The seed inventory the cell gates still exists.
if [[ -f "tools/coverage/seed-inventory.txt" ]]; then
  ok
else
  bad "seed inventory missing: tools/coverage/seed-inventory.txt"
fi

dx_test_summary "coverage spill harness"
