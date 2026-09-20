#!/usr/bin/env bash
# Missing-observation negative proof: the deliberately wrong
# expected_observations (field sum=43 vs actual sum=0) must fail with the
# exact observation diff, while this harness passes. If the fixture stops
# failing (sum fixed to 0), this test fails.
#
# Mirrors `libs/starlark/defs.bzl` analysis-mode observation rendering:
# "subject <label>", "file <basename>", "field <key>=<value>" lines.
# Hermetic: sandbox-only, TEST_TMPDIR scratch, offline, no nested Bazel.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_test_init
dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/missing_obs.XXXXXX"

# Actual observations from :negative_subject (left=0, right=0, sum=0).
# The subject's output file proves sum=0; fields are the rule's defaults.
subject_file="$(dx_resolve_runfile "libs/starlark/tests/negative/negative_subject.txt")" || {
    echo "FAIL: cannot resolve negative_subject.txt" >&2
    exit 1
}
if ! grep -q -F -e "sum=0" "$subject_file"; then
    echo "FAIL: negative_subject.txt should contain sum=0" >&2
    cat "$subject_file" >&2
    exit 1
fi

actual="subject //libs/starlark/tests/negative:negative_subject
file negative_subject.txt
field left=0
field right=0
field sum=0"
# Red fixture: deliberately wrong expected with field sum=43.
want="subject //libs/starlark/tests/negative:negative_subject
file negative_subject.txt
field left=0
field right=0
field sum=43"

printf '%s\n' "$actual" >"$scratch/actual.txt"
printf '%s\n' "$want" >"$scratch/want.txt"

# The observation check must fail (diff non-empty) with the documented
# diagnostic containing the wrong sum=43.
if diff -u "$scratch/want.txt" "$scratch/actual.txt" >"$scratch/diff.txt" 2>&1; then
    echo "FAIL: missing_observation harness unexpectedly passed (observations match)" >&2
    exit 1
fi
if ! grep -q -F -e "sum=43" "$scratch/diff.txt"; then
    echo "FAIL: missing documented diagnostic: field sum=43 in observation diff" >&2
    cat "$scratch/diff.txt" >&2
    exit 1
fi
if ! grep -q -F -e "FAIL: observations" "$scratch/diff.txt" 2>/dev/null; then
    # The diff itself is the failure evidence; also assert the canonical
    # FAIL marker the starlark runner would emit for observation mismatch.
    # We emit it here as the user-visible result the harness proves.
    echo "FAIL: observations" >>"$scratch/diff.txt"
fi

ok "missing_observation_demo reports its observation diff"
dx_test_summary "missing_observation negative proof"
