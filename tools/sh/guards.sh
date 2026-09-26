#!/usr/bin/env bash
set -euo pipefail

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
  if dx_hermetic_grep contains "$file" --fixed -- "$lit" >/dev/null 2>&1; then
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
  if dx_hermetic_grep contains "$file" --fixed -- "$lit" >/dev/null 2>&1; then
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
  if dx_hermetic_grep contains "$file" --re -- "$re" >/dev/null 2>&1; then
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
  if dx_hermetic_grep contains "$file" --re -- "$re" >/dev/null 2>&1; then
    bad "$file must not match [$re] ($reason)"
  else
    ok
  fi
  return 0
}

dx_guards_contains() {
  local file="$1" reason="$2"
  shift 2
  local missing="" lit
  if [[ ! -f "$file" ]]; then
    bad "missing file $file ($reason; want literals: $*)"
    return 0
  fi
  for lit in "$@"; do
    if ! dx_hermetic_grep contains "$file" --fixed -- "$lit" >/dev/null 2>&1; then
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
    if dx_hermetic_grep contains "$file" --fixed -- "$lit" >/dev/null 2>&1; then
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
    if ! dx_hermetic_grep contains "$file" --re -- "$re" >/dev/null 2>&1; then
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
    if dx_hermetic_grep contains "$file" --re -- "$re" >/dev/null 2>&1; then
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

dx_guard_tree_contains() {
  local include="$1" lit="$2" reason="$3"
  if dx_hermetic_grep tree-contains --fixed --include "$include" --roots . -- "$lit" >/dev/null 2>&1; then
    ok
  else
    bad "tree missing literal [$lit] in $include ($reason)"
  fi
  return 0
}

dx_guard_tree_absent() {
  local include="$1" lit="$2" reason="$3"
  if dx_hermetic_grep tree-contains --fixed --include "$include" --roots . -- "$lit" >/dev/null 2>&1; then
    bad "tree must not contain [$lit] in $include ($reason)"
  else
    ok
  fi
  return 0
}

dx_guard_tree_contains_re() {
  local include="$1" re="$2" reason="$3"
  if dx_hermetic_grep tree-contains --re --include "$include" --roots . -- "$re" >/dev/null 2>&1; then
    ok
  else
    bad "tree missing pattern [$re] in $include ($reason)"
  fi
  return 0
}

dx_guard_tree_absent_re() {
  local include="$1" re="$2" reason="$3"
  if dx_hermetic_grep tree-contains --re --include "$include" --roots . -- "$re" >/dev/null 2>&1; then
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
    if ! dx_hermetic_grep tree-contains --fixed --include "$include" --roots . -- "$lit" >/dev/null 2>&1; then
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
    if dx_hermetic_grep tree-contains --fixed --include "$include" --roots . -- "$lit" >/dev/null 2>&1; then
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
    if ! dx_hermetic_grep tree-contains --re --include "$include" --roots . -- "$re" >/dev/null 2>&1; then
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
    if dx_hermetic_grep tree-contains --re --include "$include" --roots . -- "$re" >/dev/null 2>&1; then
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
