#!/usr/bin/env bash
# Manual negative-demo harness (issue #99 item 2).
#
# The expected-fail demonstrations in libs/starlark/tests/negative plus
# the markdown-no-config subject carry `tags = ["manual"]`, so
# `bazel test //...` never executes them and they would rot silently
# (a demo that starts passing, or fails for a new reason, looks the
# same as a healthy demo to wildcard suites). This script runs each
# demo explicitly and asserts it still fails FOR ITS DOCUMENTED
# REASON, then guards the enumeration: every remaining manual test
# must be a private `*_upstream` implementation detail (exercised via
# its public forwarding wrapper), so a newly manual test forces
# explicit classification here.
#
# Hermetic promotion of the documented manual invocations (see
# docs/testing/starlark.md): versioned here, run by CI via
# `bazel run //tools/ci:manual_negatives`, following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
cd "$workspace"

pass=0
fail=0
expect_fail() { # name, then want-substrings, then --, then bazel args
  local name="$1"
  shift
  local wants=()
  while [[ "$1" != "--" ]]; do
    wants+=("$1")
    shift
  done
  shift
  local out rc=0
  out="$(bazel "$@" 2>&1)" || rc=$?
  if [[ "$rc" == "0" ]]; then
    echo "FAIL: $name unexpectedly passed" >&2
    fail=$((fail + 1))
    return
  fi
  local want ok=1
  for want in "${wants[@]}"; do
    if [[ "$out" != *"$want"* ]]; then
      echo "FAIL: $name missing documented diagnostic: $want" >&2
      echo "--- captured output:" >&2
      echo "$out" >&2
      ok=0
    fi
  done
  if [[ "$ok" == "1" ]]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
  fi
}

common=(--noshow_progress --nocache_test_results)
expect_fail "failing_check_demo reports its wrong expects" \
  "FAIL: deliberately wrong sum" "starlark_test: 1 passed, 2 failed" \
  -- test "${common[@]}" --test_output=all //libs/starlark/tests/negative:failing_check_demo
expect_fail "missing_observation_demo reports its observation diff" \
  "FAIL: observations" "field sum=43" \
  -- test "${common[@]}" --test_output=all //libs/starlark/tests/negative:missing_observation_demo
expect_fail "missing_fragment_demo reports its absent substring" \
  "is missing substring 1/1" "substring: this substring is absent" \
  -- test "${common[@]}" --test_output=all //libs/starlark/tests/negative:missing_fragment_demo
expect_fail "wrong_phase_demo fails analysis on the mode violation" \
  "starlark_test (load mode): subjects must be empty" \
  -- build --noshow_progress //libs/starlark/tests/negative:wrong_phase_demo
expect_fail "markdown-no-config subject fails on the missing Vale config" \
  "applicable Vale requires declared config" \
  -- build --noshow_progress --nobuild //quality/testdata:fixture_real_markdown_no_config_subject

# Enumeration guard: the four starlark demos above are the only manual
# tests allowed outside the private `*_upstream` implementation-detail
# family (file targets such as `*.pytest_paths` share the suffix).
demos=(
  "//libs/starlark/tests/negative:failing_check_demo"
  "//libs/starlark/tests/negative:missing_fragment_demo"
  "//libs/starlark/tests/negative:missing_observation_demo"
  "//libs/starlark/tests/negative:wrong_phase_demo"
)
# Stage 4 E2E drivers (issue #55) are `manual` (+`exclusive`) so the
# slow nested-Bazel suite runs only via explicit `bazel test //tools/ci:e2e`,
# never under wildcards. They are behavior pins, not failure demos: no
# expected-failure proof here, just enumeration so a new manual test
# still forces explicit classification below.
e2e_tests=(
  "//tools/ci:e2e"
  "//tools/ci:e2e_clean"
  "//tools/ci:e2e_dirty"
  "//tools/ci:e2e_format_roundtrip"
)
while IFS= read -r target; do
  [[ -n "$target" ]] || continue
  if [[ "$target" == *_upstream* ]]; then
    pass=$((pass + 1))
    continue
  fi
  known=0
  demo=""
  for demo in "${demos[@]}"; do
    if [[ "$target" == "$demo" ]]; then
      known=1
      break
    fi
  done
  e2e=""
  for e2e in "${e2e_tests[@]}"; do
    if [[ "$target" == "$e2e" ]]; then
      known=1
      break
    fi
  done
  if [[ "$known" == "1" ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: unclassified manual test $target (prove its failure here or keep it in the *_upstream family)" >&2
    fail=$((fail + 1))
  fi
done < <(bazel query 'attr(tags, manual, kind(test, //...))' 2>/dev/null | LC_ALL=C sort -u)

echo "manual negatives harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
