#!/usr/bin/env bash
# Stage 4 E2E driver (issue #55): stage one `integration/<scenario>`
# child workspace to scratch and run the parent-built `dx` binary
# against it with the version-pinned Bazel first on `PATH`.
#
# Usage: e2e.sh <scenario> <want-exit> <dx-bin> <bazel-wrapper>
#   <scenario>      directory under integration/ (plain files, never labels:
#                   the tree is .bazelignore'd out of the parent universe).
#   <want-exit>     expected `dx ... --check` exit code in the child.
#   <dx-bin>        parent-built dx binary (runfiles path).
#   <bazel-wrapper> `@build_bazel_bazel_9_2_0//:bazel_binary` wrapper
#                   (runfiles path); symlinked as `bazel` onto PATH so the
#                   child run uses exactly the pinned Bazel, never host state.
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

scenario="${1:?usage: e2e.sh <scenario> <want-exit> <dx-bin> <bazel-wrapper>}"
want_exit="${2:?usage: e2e.sh <scenario> <want-exit> <dx-bin> <bazel-wrapper>}"
dx_rel="${3:?usage: e2e.sh <scenario> <want-exit> <dx-bin> <bazel-wrapper>}"
bazelw_rel="${4:?usage: e2e.sh <scenario> <want-exit> <dx-bin> <bazel-wrapper>}"

workspace="$(dx_e2e_workspace_root)"
if [[ ! -d "$workspace/integration" ]]; then
  echo "FAIL: workspace $workspace has no integration/ (E2E_WORKSPACE mis-set?)" >&2
  exit 1
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# Pin guard: the child Bazel must track //.bazelversion exactly (no
# multi-version matrix, issue #55). The MODULE download pin and the
# version file move together or this fails before staging anything.
pin_version="$(cat .bazelversion)"
[[ "$pin_version" == "9.2.0" ]] || { bad ".bazelversion is $pin_version, want 9.2.0"; }
grep -q -F -e 'bazel_binaries.download(version = "9.2.0")' MODULE.bazel \
  || { bad "MODULE.bazel lost the 9.2.0 bazel_binaries pin"; }

dx_bin="$(dx_resolve_runfile "$dx_rel")" || { bad "dx binary not found: $dx_rel"; }
bazel_wrapper="$(dx_resolve_runfile "$bazelw_rel")" || { bad "bazel wrapper not found: $bazelw_rel"; }
[[ -f "$workspace/integration/$scenario/MODULE.bazel" ]] \
  || { bad "scenario missing: integration/$scenario/MODULE.bazel"; }
[[ "$fail" == "0" ]] || { echo "e2e $scenario: $pass passed, $fail failed"; exit 1; }
ok # pins + inputs resolve

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

cp -r "$workspace/integration/$scenario/." "$scratch/child/"
grep -r -q -F -e "RULES_DX_ROOT" "$scratch/child/MODULE.bazel" \
  || { bad "child MODULE.bazel lost its RULES_DX_ROOT placeholder"; exit 1; }
# Portable in-place edit (issue #299): GNU `sed -i -e` breaks on macOS
# BSD sed; the tmpfile form works on both.
sed -e "s|RULES_DX_ROOT|$workspace|" "$scratch/child/MODULE.bazel" > "$scratch/child/MODULE.bazel.tmp" && mv "$scratch/child/MODULE.bazel.tmp" "$scratch/child/MODULE.bazel"
grep -r -q -F -e "RULES_DX_ROOT" "$scratch/child/" \
  && { bad "RULES_DX_ROOT placeholder survived staging"; exit 1; }
ok # staging rewrites the parent path with none left behind

mkdir -p "$scratch/bin" "$scratch/bazelisk_home" "$scratch/home"
ln -s "$bazel_wrapper" "$scratch/bin/bazel"

out=""
rc=0
# `dx test` takes no `--check` (quality-only flag): bare `dx test //...`
# runs Bazel tests and preserves Bazel's exit code verbatim (0 clean,
# 3 on test failures per the CLI contract). `--check` here would fail
# pre-exec with exit 2 (`option "--check" is not supported by dx test`).
out="$(cd "$scratch/child" && \
  PATH="$scratch/bin:$PATH" \
  BAZELISK_HOME="$scratch/bazelisk_home" \
  HOME="$scratch/home" \
  "$dx_bin" test //... 2>&1)" || rc=$?
if [[ "$rc" == "$want_exit" ]]; then
  ok
else
  bad "dx test //... exited $rc, want $want_exit"
fi
echo "--- child output (tail) ---"
echo "$out" | tail -n 8

echo "e2e $scenario: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
