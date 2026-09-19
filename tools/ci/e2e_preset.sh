#!/usr/bin/env bash
# Stage 4 E2E preset-stale driver (issue #332): stage
# `integration/preset-stale` to scratch and pin the `dx update --check`
# exit + regeneration contract in a consumer-shaped child workspace.
#
# Usage: e2e_preset.sh <dx-bin>
#   <dx-bin>  parent-built dx binary (runfiles path).
#
# Round-trip (mirrors docs/cli/commands/audit-update-bazel.md#dx-update:
# `dx update --check` never writes; any stale fragment fails check mode):
#   1. `dx update --check` exits 1 on the dirty fragment.
#   2. `dx update go` exits 0 and regenerates the reviewed inventory
#      (`go` is a no-op backend, so no Bazel launch is needed).
#   3. `dx update --check` exits 0 on the fixed tree.
# Paired with the `//:preset_parity_test` snapshot, this proves the suite
# is not theater for the preset surface: a detector stuck always-pass
# would fail step 1, stuck always-fail would fail step 3, and a
# non-mutating fixer would fail step 2's content check.
#
# Staging happens under scratch, never in the checkout (mirrors the
# publish dry-run staging discipline): the `RULES_DX_ROOT` placeholder
# in the child MODULE.bazel is rewritten to the absolute parent path.
# Run by CI via explicit `bazel test //tools/ci:e2e` (drivers are
# `manual`: wildcard suites never run them).
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_rel="${1:?usage: e2e_preset.sh <dx-bin>}"
scenario="preset-stale"

workspace="$(dx_e2e_workspace_root)"
if [[ ! -d "$workspace/integration" ]]; then
  echo "FAIL: workspace $workspace has no integration/ (E2E_WORKSPACE mis-set?)" >&2
  exit 1
fi
cd "$workspace"

dx_test_init

dx_bin="$(dx_resolve_runfile "$dx_rel")" || { bad "dx binary not found: $dx_rel"; }
[[ -f "$workspace/integration/$scenario/MODULE.bazel" ]] ||
  { bad "scenario missing: integration/$scenario/MODULE.bazel"; }
[[ "$fail" == "0" ]] || {
  echo "e2e $scenario: $pass passed, $fail failed"
  exit 1
}
ok # pins + inputs resolve

dx_mkscratch scratch

cp -r "$workspace/integration/$scenario/." "$scratch/child/"
grep -r -q -F -e "RULES_DX_ROOT" "$scratch/child/MODULE.bazel" ||
  {
    bad "child MODULE.bazel lost its RULES_DX_ROOT placeholder"
    exit 1
  }
# Portable in-place edit via dx_replace (issues #299, #323): GNU `sed -i -e`
# breaks on macOS BSD sed; the tmpfile form works on both.
dx_replace "s|RULES_DX_ROOT|$workspace|" "$scratch/child/MODULE.bazel"
grep -r -q -F -e "RULES_DX_ROOT" "$scratch/child/" &&
  {
    bad "RULES_DX_ROOT placeholder survived staging"
    exit 1
  }
ok # staging rewrites the parent path with none left behind

# The staged child must start dirty: otherwise the round-trip proves
# nothing (step 1 would pass on an already-clean tree).
if grep -q -F -e "# dirty" "$scratch/child/tools/bazelrc/preset.bazelrc"; then
  ok
else
  bad "staged preset.bazelrc lost its dirty content"
fi

# Step 1: check mode fails closed on the dirty fragment (exit 1, no writes).
out=""
rc=0
out="$(cd "$scratch/child" && "$dx_bin" --workspace "$scratch/child" update --check 2>&1)" || rc=$?
if [[ "$rc" == "1" ]]; then
  ok
else
  bad "dx update --check exited $rc, want 1 (dirty fragment)"
fi
echo "--- check-dirty output (tail) ---"
echo "$out" | tail -n 8
if grep -q -F -e "# dirty" "$scratch/child/tools/bazelrc/preset.bazelrc"; then
  ok
else
  bad "check mode mutated preset.bazelrc (check must never write)"
fi

# Step 2: default mode fixes the fragment (exit 0) via the no-op `go`
# backend plus atomic preset regeneration (no Bazel launch needed).
out=""
rc=0
out="$(cd "$scratch/child" && "$dx_bin" --workspace "$scratch/child" update go 2>&1)" || rc=$?
if [[ "$rc" == "0" ]]; then
  ok
else
  bad "dx update go exited $rc, want 0"
fi
echo "--- update output (tail) ---"
echo "$out" | tail -n 8
if grep -q -F -e "common --enable_bzlmod" "$scratch/child/tools/bazelrc/preset.bazelrc" &&
  ! grep -q -F -e "# dirty" "$scratch/child/tools/bazelrc/preset.bazelrc"; then
  ok
else
  bad "dx update did not regenerate preset.bazelrc; content: $(cat "$scratch/child/tools/bazelrc/preset.bazelrc")"
fi

# Step 3: check mode passes on the fixed tree (exit 0).
out=""
rc=0
out="$(cd "$scratch/child" && "$dx_bin" --workspace "$scratch/child" update --check 2>&1)" || rc=$?
if [[ "$rc" == "0" ]]; then
  ok
else
  bad "dx update --check after fix exited $rc, want 0"
fi
echo "--- check-clean output (tail) ---"
echo "$out" | tail -n 8

dx_test_summary "e2e $scenario"
