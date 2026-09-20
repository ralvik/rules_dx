#!/usr/bin/env bash
# Failing-check negative proof: the deliberately wrong
# unit checks below must fail with the exact user-visible diagnostics,
# while this harness itself passes (exit 0). If the fixtures stop
# failing (wrong values fixed), this test fails.
#
# Mirrors `libs/starlark/defs.bzl` `_RUNNER_PRELUDE` check() verbatim:
# PASS prints "PASS: <name>", FAIL prints "FAIL: <name>" plus
# expected/actual and sets fail=1. Hermetic: sandbox-only, TEST_TMPDIR
# scratch, offline, no nested Bazel.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_test_init
dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/failing_check.XXXXXX"

# Verbatim `check` from starlark_test runner (libs/starlark/defs.bzl).
# NOTE: inner_fail (not `fail`) to avoid clobbering lib.sh's harness counter.
inner_fail=0
pass_count=0
fail_count=0
check() {
    name="$1"; expected="$2"; actual="$3"
    if [ "$expected" = "$actual" ]; then
        echo "PASS: $name"
        pass_count=$((pass_count + 1))
    else
        echo "FAIL: $name"
        echo "  expected: $expected"
        echo "  actual:   $actual"
        inner_fail=1
        fail_count=$((fail_count + 1))
    fi
}

# Red fixtures (deliberately wrong, kept verbatim for matrix validation):
# - "deliberately wrong sum": 1 + 1 (=2) vs 3
# - "deliberately wrong product": 2 * 2 (=4) vs 5
# - control passes: 1 + 1 (=2) vs 2
out="$scratch/out.txt"
{
    check "deliberately wrong sum" "3" "2"
    check "deliberately wrong product" "5" "4"
    check "control that still passes" "2" "2"
    echo "starlark_test: $pass_count passed, $fail_count failed"
} >"$out" 2>&1 || true

# The harness itself must observe the documented failure: inner_fail=1
# from the two wrong checks, exact FAIL lines, exact summary.
if [[ "$inner_fail" != "1" ]]; then
    echo "FAIL: failing_check harness did not observe failure (inner_fail=$inner_fail)" >&2
    cat "$out" >&2
    exit 1
fi
if ! grep -q -F -e "FAIL: deliberately wrong sum" "$out"; then
    echo "FAIL: missing documented diagnostic: FAIL: deliberately wrong sum" >&2
    cat "$out" >&2
    exit 1
fi
if ! grep -q -F -e "FAIL: deliberately wrong product" "$out"; then
    echo "FAIL: missing documented diagnostic: FAIL: deliberately wrong product" >&2
    cat "$out" >&2
    exit 1
fi
if ! grep -q -F -e "starlark_test: 1 passed, 2 failed" "$out"; then
    echo "FAIL: missing documented summary: starlark_test: 1 passed, 2 failed" >&2
    cat "$out" >&2
    exit 1
fi
if ! grep -q -F -e "PASS: control that still passes" "$out"; then
    echo "FAIL: control check should still pass" >&2
    cat "$out" >&2
    exit 1
fi

ok "failing_check_demo reports its wrong expects"
dx_test_summary "failing_check negative proof"
