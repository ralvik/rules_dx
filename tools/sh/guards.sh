#!/usr/bin/env bash
# Table-driven guard library.
#
# Single-sources the `{file, must-contain|must-not-contain, reason/issue}`
# table rows repeated ad-hoc across `tools/ci` guards (`if grep -q ...;
# then ok else bad "...issue #..."` per file). Each helper is one table
# row: file (or tree include), expectation kind (function name), pattern,
# and reason/issue context. Drivers write a vertical list of calls that
# reads as the guard table instead of copying `grep -q` chains.
#
# Requires `tools/sh/lib.sh` counters first (`dx_test_init`, `ok`/`bad`/
# `dx_test_summary`); load both via the single-sourced bootstrap
# (`tools/sh/bootstrap.sh` `dx_bootstrap`, issue #654) with no per-file
# depth adjustment:
#
#   source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
#   dx_bootstrap "tools/sh/lib.sh"
#   dx_bootstrap "tools/sh/guards.sh"
#
# Guard maintenance owns shared helpers plus snapshot versus grep policy
# (successor to issue #450, owned here under issue #653;
# `//tools/ci:shell_contract` owns the rule):
# snapshot (`tools/sh/snapshot.sh` with UPDATE_EXPECT) is for
# byte-identical golden outputs with refresh (whole-file renderer output,
# generated fragments, canonical JSON: reviewer sees the diff and refreshes
# explicitly); `dx_guard_*` fixed-string pins are for doc/code contract
# sentences/symbols (a few literals per file, fail-closed, no refresh, one
# `ok`/`bad` per row or per batch with file:pattern plus reason context).
# Prefer fixed-string (`grep -F -e`) for contract sentences/symbols; use
# the `_re` regex forms (`grep -E -e`) only for shapes (SHA pins, version
# alternatives, anchors). Prefer single-file pins when the location is
# known; use the `tree` forms only for repo-wide presence/absence with an
# `--include` glob (they skip `bazel-*` plus `.git`). Drivers must not
# reimplement guard rows; extend this file instead.
#
# Provides (all always return 0 so the harness collects every failure):
#   dx_guard_file <file> <reason>
#   dx_guard_contains <file> <lit> <reason>
#   dx_guard_absent <file> <lit> <reason>
#   dx_guard_re_contains <file> <re> <reason>
#   dx_guard_re_absent <file> <re> <reason>
#   dx_guards_contains <file> <reason> <lit>...
#   dx_guards_absent <file> <reason> <lit>...
#   dx_guards_re_contains <file> <reason> <re>...
#   dx_guards_re_absent <file> <reason> <re>...
#   dx_guard_tree_contains <include> <lit> <reason>
#   dx_guard_tree_absent <include> <lit> <reason>
#   dx_guard_tree_contains_re <include> <re> <reason>
#   dx_guard_tree_absent_re <include> <re> <reason>
#   dx_guards_tree_contains <include> <reason> <lit>...
#   dx_guards_tree_absent <include> <reason> <lit>...
#   dx_guards_tree_contains_re <include> <reason> <re>...
#   dx_guards_tree_absent_re <include> <reason> <re>...
#
# Bash-only Linux harness: sourced by `sh_binary` /
# `sh_test` drivers carrying `target_compatible_with =
# ["@platforms//os:linux"]`. Bootstrap requires bash by design under issue
# #450 (`BASH_SOURCE`, `[[`, `printf -v` plus the 5-way runfiles fallback
# never run under POSIX `sh`); floor is bash 3.2+. No bare `grep -q`
# chains in drivers; every guard row carries its reason/issue.
# Shellcheck/shfmt clean (`shfmt -i 2 -ci`, `.shellcheckrc` bash + all
# checks).
set -euo pipefail

# Guard-maintenance table rows: one ok/bad per row with file:pattern plus
# reason context, always returning 0 so the harness collects every failure
# before `dx_test_summary`.

dx_guard_file() {
  local file="$1" reason="$2"
  if [[ -f "$file" ]]; then
    ok
  else
    bad "missing file $file ($reason)"
  fi
  return 0
}

dx_guard_contains() {
  local file="$1" lit="$2" reason="$3"
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want literal: $lit)"
    return 0
  fi
  if grep -q -F -e "$lit" -- "$file"; then
    ok
  else
    bad "$file missing literal [$lit] ($reason)"
  fi
  return 0
}

dx_guard_absent() {
  local file="$1" lit="$2" reason="$3"
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want absence of: $lit)"
    return 0
  fi
  if grep -q -F -e "$lit" -- "$file"; then
    bad "$file must not contain [$lit] ($reason)"
  else
    ok
  fi
  return 0
}

dx_guard_re_contains() {
  local file="$1" re="$2" reason="$3"
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want pattern: $re)"
    return 0
  fi
  if grep -q -E -e "$re" -- "$file"; then
    ok
  else
    bad "$file missing pattern [$re] ($reason)"
  fi
  return 0
}

dx_guard_re_absent() {
  local file="$1" re="$2" reason="$3"
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want absence of pattern: $re)"
    return 0
  fi
  if grep -q -E -e "$re" -- "$file"; then
    bad "$file must not match [$re] ($reason)"
  else
    ok
  fi
  return 0
}

# Batch rows: same file plus same reason, many literals/patterns, one
# ok/bad. Collapses `grep -q ... && grep -q ...` chains.

dx_guards_contains() {
  local file="$1" reason="$2"
  shift 2
  local missing="" lit
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want literals: $*)"
    return 0
  fi
  for lit in "$@"; do
    if ! grep -q -F -e "$lit" -- "$file"; then
      missing="$missing [$lit]"
    fi
  done
  if [[ -z "$missing" ]]; then
    ok
  else
    bad "$file missing literals:$missing ($reason)"
  fi
  return 0
}

dx_guards_absent() {
  local file="$1" reason="$2"
  shift 2
  local present="" lit
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want absence of: $*)"
    return 0
  fi
  for lit in "$@"; do
    if grep -q -F -e "$lit" -- "$file"; then
      present="$present [$lit]"
    fi
  done
  if [[ -z "$present" ]]; then
    ok
  else
    bad "$file must not contain:$present ($reason)"
  fi
  return 0
}

dx_guards_re_contains() {
  local file="$1" reason="$2"
  shift 2
  local missing="" re
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want patterns: $*)"
    return 0
  fi
  for re in "$@"; do
    if ! grep -q -E -e "$re" -- "$file"; then
      missing="$missing [$re]"
    fi
  done
  if [[ -z "$missing" ]]; then
    ok
  else
    bad "$file missing patterns:$missing ($reason)"
  fi
  return 0
}

dx_guards_re_absent() {
  local file="$1" reason="$2"
  shift 2
  local present="" re
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want absence of patterns: $*)"
    return 0
  fi
  for re in "$@"; do
    if grep -q -E -e "$re" -- "$file"; then
      present="$present [$re]"
    fi
  done
  if [[ -z "$present" ]]; then
    ok
  else
    bad "$file must not match:$present ($reason)"
  fi
  return 0
}

# Tree rows: repo-wide presence/absence with an `--include` glob, skipping
# `bazel-*` plus `.git` outputs. Prefer single-file pins when the location
# is known.

dx_guard_tree_contains() {
  local include="$1" lit="$2" reason="$3"
  if grep -rn -F --include="$include" -e "$lit" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
    ok
  else
    bad "tree missing literal [$lit] in $include ($reason)"
  fi
  return 0
}

dx_guard_tree_absent() {
  local include="$1" lit="$2" reason="$3"
  if grep -rn -F --include="$include" -e "$lit" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
    bad "tree must not contain [$lit] in $include ($reason)"
  else
    ok
  fi
  return 0
}

dx_guard_tree_contains_re() {
  local include="$1" re="$2" reason="$3"
  if grep -rn -E --include="$include" -e "$re" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
    ok
  else
    bad "tree missing pattern [$re] in $include ($reason)"
  fi
  return 0
}

dx_guard_tree_absent_re() {
  local include="$1" re="$2" reason="$3"
  if grep -rn -E --include="$include" -e "$re" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
    bad "tree must not match [$re] in $include ($reason)"
  else
    ok
  fi
  return 0
}

dx_guards_tree_contains() {
  local include="$1" reason="$2"
  shift 2
  local missing="" lit
  for lit in "$@"; do
    if ! grep -rn -F --include="$include" -e "$lit" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
      missing="$missing [$lit]"
    fi
  done
  if [[ -z "$missing" ]]; then
    ok
  else
    bad "tree missing literals in $include:$missing ($reason)"
  fi
  return 0
}

dx_guards_tree_absent() {
  local include="$1" reason="$2"
  shift 2
  local present="" lit
  for lit in "$@"; do
    if grep -rn -F --include="$include" -e "$lit" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
      present="$present [$lit]"
    fi
  done
  if [[ -z "$present" ]]; then
    ok
  else
    bad "tree must not contain in $include:$present ($reason)"
  fi
  return 0
}

dx_guards_tree_contains_re() {
  local include="$1" reason="$2"
  shift 2
  local missing="" re
  for re in "$@"; do
    if ! grep -rn -E --include="$include" -e "$re" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
      missing="$missing [$re]"
    fi
  done
  if [[ -z "$missing" ]]; then
    ok
  else
    bad "tree missing patterns in $include:$missing ($reason)"
  fi
  return 0
}

dx_guards_tree_absent_re() {
  local include="$1" reason="$2"
  shift 2
  local present="" re
  for re in "$@"; do
    if grep -rn -E --include="$include" -e "$re" --exclude-dir='bazel-*' --exclude-dir='.git' . >/dev/null 2>&1; then
      present="$present [$re]"
    fi
  done
  if [[ -z "$present" ]]; then
    ok
  else
    bad "tree must not match in $include:$present ($reason)"
  fi
  return 0
}
