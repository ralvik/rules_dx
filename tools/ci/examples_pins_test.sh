#!/usr/bin/env bash
# Example caller pin-sync harness.
#
# Both example callers (consumer-ci, docs-ci) pin their reusable workflow
# at one shared reviewed commit: drift between them fails the gate, so a
# deliberate bump moves both pins in one reviewed change. Each caller also
# carries a runnable shape (`branches: [main]`, full-SHA pin, matching
# `rules_dx_version`) with no `$default-branch` placeholder or floating
# tag. The closing negative controls prove sensitivity: a drifted pin, a
# floating tag, and a placeholder branch must each fail.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

consumer="$1"
docs="$2"
module="$3"

dx_test_init

sha_of() { # caller, workflow-file-name -> sha or empty
  grep -o -E -e "uses: rules_dx/\.github/workflows/$2@[0-9a-f]{40}" "$1" | head -1 | cut -d@ -f2
}

consumer_sha="$(sha_of "$consumer" 'reusable-consumer\.yml')"
docs_sha="$(sha_of "$docs" 'reusable-docs\.yml')"

if [[ -n "$consumer_sha" ]]; then
  ok
else
  bad "consumer-ci caller must reference reusable-consumer.yml by full SHA exactly once"
fi
if [[ -n "$docs_sha" ]]; then
  ok
else
  bad "docs-ci caller must reference reusable-docs.yml by full SHA exactly once"
fi
if [[ -n "$consumer_sha" && "$consumer_sha" == "$docs_sha" ]]; then
  ok
else
  bad "caller pins drifted: consumer-ci @$consumer_sha vs docs-ci @$docs_sha (bump both together)"
fi

# No floating `uses: owner/repo@tag` in either caller.
if grep -E -e 'uses: [^ ]+@(v[0-9]|main|master|latest)' "$consumer" "$docs" >/dev/null; then
  bad "floating action tag found in example caller"
else
  ok
fi

# `rules_dx_version` in both callers matches the module() version.
module_version="$(awk '/^module\(/,/^\)/' "$module" | grep -o -E -e 'version = "[^"]+"' | head -1 | cut -d'"' -f2)"
for caller in "$consumer" "$docs"; do
  caller_version="$(grep -o -E -e 'rules_dx_version: "[^"]+"' "$caller" | head -1 | cut -d'"' -f2)"
  if [[ -n "$caller_version" && "$caller_version" == "$module_version" ]]; then
    ok
  else
    bad "rules_dx_version ($caller_version) in $caller must match module() version ($module_version)"
  fi
done

# Runnable trigger shape: `branches: [main]`, no `$default-branch` placeholder.
for caller in "$consumer" "$docs"; do
  if grep -F -e '$default-branch' "$caller" >/dev/null; then
    bad "\$default-branch placeholder in $caller (use [main])"
  else
    ok
  fi
  if grep -E -e 'branches: \[main\]' "$caller" >/dev/null; then
    ok
  else
    bad "$caller must trigger push on branches: [main]"
  fi
done

# Sensitivity negatives: each mutation must fail a fresh run of this script.
# Guarded by DX_PINS_SKIP_NEGATIVES so nested runs terminate (else exponential recursion).
if [[ "${DX_PINS_SKIP_NEGATIVES:-0}" != "1" ]]; then
  dx_mkscratch scratch
  mutated="$scratch/caller.yml"
  expect_fail() { # description, mutated-docs-file
    if DX_PINS_SKIP_NEGATIVES=1 "$0" "$consumer" "$2" "$module" >/dev/null 2>&1; then
      bad "negative control passed but must fail: $1"
    else
      ok
    fi
  }
  sed "s/$docs_sha/ffffffffffffffffffffffffffffffffffffffff/" "$docs" >"$mutated"
  expect_fail "drifted docs pin" "$mutated"
  cp "$docs" "$mutated"
  printf '      - uses: actions/checkout@v4\n' >>"$mutated"
  expect_fail "floating tag in docs caller" "$mutated"
  sed 's/branches: \[main\]/branches: [$default-branch]/' "$docs" >"$mutated"
  expect_fail "placeholder branch in docs caller" "$mutated"
fi

dx_test_summary "examples_pins"
