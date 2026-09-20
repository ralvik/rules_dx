#!/usr/bin/env bash
# Coverage-spill containment harness.
#
# `tools/ci/coverage_cell.sh` runs `bazel coverage`, which
# leaves `bazel-bin/tools/coverage/coverage_bin` instrumented
# (`-C instrument-coverage`), then executes it directly from the workspace
# root with `LLVM_PROFILE_FILE` unset. Rust defaults to
# `default_%m_%p.profraw` in CWD, spilling one file per invocation.
# `.gitignore` only hides the spill; this harness pins the root fix.
#
# This harness machine-checks the containment half verifiable on a clean
# tree today: the LLVM_PROFILE_FILE export targets the
# auto-cleaned scratch dir before the first helper execution, the trap
# still cleans scratch, the gate invocations are intact (not deleted to
# fake containment), the gitignore defense-in-depth stays, and the
# seed inventory still exists. It proves
# containment statically; the full `coverage_cell` run remains the
# end-to-end proof under.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_spill`,
# following //tools/ci:coverage_cell.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

cell="tools/ci/coverage_cell.sh"

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

# The seed inventory the cell gates still exists.
if [[ -f "tools/coverage/seed-inventory.txt" ]]; then
  ok
else
  bad "seed inventory missing: tools/coverage/seed-inventory.txt"
fi

dx_test_summary "coverage spill harness"
