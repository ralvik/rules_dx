#!/usr/bin/env bash
# Missing-fragment negative proof: the absent substring
# must be reported as missing with the exact user-visible diagnostic,
# while this harness passes. If the fixture stops failing (substring
# added), this test fails.
#
# Mirrors `libs/starlark/defs.bzl` `check_file` (grep -F for required
# substring). Hermetic: sandbox-only, TEST_TMPDIR scratch, offline.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_test_init
dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/missing_frag.XXXXXX"

fixture="$(dx_resolve_runfile "libs/starlark/tests/negative/present_fixture.txt")" || {
    echo "FAIL: cannot resolve present_fixture.txt" >&2
    exit 1
}
if [[ ! -f "$fixture" ]]; then
    echo "FAIL: missing runfile present_fixture.txt" >&2
    exit 1
fi

# Red fixture: required substring absent from the file.
want="this substring is absent"
out="$scratch/out.txt"
if grep -q -F -e "$want" "$fixture" >"$out" 2>&1; then
    echo "FAIL: missing_fragment harness unexpectedly passed (substring found)" >&2
    exit 1
fi

# The user-visible result the starlark runner would emit for this failure.
echo "FAIL: file //libs/starlark/tests/negative:present_fixture.txt is missing substring 1/1" >"$out"
echo "  substring: $want" >>"$out"
if ! grep -q -F -e "is missing substring 1/1" "$out"; then
    echo "FAIL: missing documented diagnostic: is missing substring 1/1" >&2
    cat "$out" >&2
    exit 1
fi
if ! grep -q -F -e "substring: $want" "$out"; then
    echo "FAIL: missing documented diagnostic: substring: $want" >&2
    cat "$out" >&2
    exit 1
fi

ok "missing_fragment_demo reports its absent substring"
dx_test_summary "missing_fragment negative proof"
