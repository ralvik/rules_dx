#!/usr/bin/env bash
# Stage 4 E2E format-roundtrip driver (issue #55): stage
# `integration/format-roundtrip` to scratch and pin the `dx format`
# exit + mutation contract in a consumer-shaped child workspace.
#
# Usage: e2e_format.sh <dx-bin> <bazel-wrapper>
#   <dx-bin>        parent-built dx binary (runfiles path).
#   <bazel-wrapper> `@build_bazel_bazel_9_2_0//:bazel_binary` wrapper
#                   (runfiles path); symlinked as `bazel` onto PATH so the
#                   child run uses exactly the pinned Bazel, never host state.
#
# Round-trip (mirrors docs/cli/commands/quality.md: `dx format --check`
# never writes; any proposed change fails check mode):
#   1. `dx format --check //...` exits 1 on the dirty `x=1` source.
#   2. `dx format //...` exits 0 and rewrites it to `x = 1`.
#   3. `dx format --check //...` exits 0 on the fixed tree.
# Paired with the clean/dirty `dx test` pins, this proves the suite is
# not theater for the quality surface: a detector stuck always-pass
# would fail step 1, stuck always-fail would fail step 3, and a
# non-mutating fixer would fail step 2's content check.
#
# Staging happens under scratch, never in the checkout (mirrors the
# publish dry-run staging discipline): the `RULES_DX_ROOT` placeholder
# in the child MODULE.bazel is rewritten to the absolute parent path.
# Run by CI via explicit `bazel test //tools/ci:e2e` (drivers are
# `manual`: wildcard suites never run them).
set -euo pipefail

dx_rel="${1:?usage: e2e_format.sh <dx-bin> <bazel-wrapper>}"
bazelw_rel="${2:?usage: e2e_format.sh <dx-bin> <bazel-wrapper>}"
scenario="format-roundtrip"

if [[ -n "${E2E_WORKSPACE:-}" ]]; then
  workspace="$E2E_WORKSPACE"
elif [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -d "${BUILD_WORKSPACE_DIRECTORY}/integration" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  # `bazel run` from the checkout: git is available and cwd is the
  # workspace. Under `bazel test` this fails (sandbox has no .git and
  # BUILD_WORKSPACE_DIRECTORY is unset) — fall through to the
  # actionable error below instead of exiting 128 with bare git noise.
  if workspace="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    :
  else
    echo "FAIL: cannot locate parent workspace (no \$E2E_WORKSPACE, no usable \$BUILD_WORKSPACE_DIRECTORY, git rev-parse failed)" >&2
    echo "Run explicitly as: E2E_WORKSPACE=\$PWD bazel test //tools/ci:e2e --test_env=E2E_WORKSPACE" >&2
    echo "CI passes E2E_WORKSPACE=\$GITHUB_WORKSPACE with --test_env=E2E_WORKSPACE (issue #55)." >&2
    exit 1
  fi
fi
if [[ ! -d "$workspace/integration" ]]; then
  echo "FAIL: workspace $workspace has no integration/ (E2E_WORKSPACE mis-set?)" >&2
  exit 1
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

resolve_runfile() { # rel -> absolute; runfiles layouts differ, so probe.
  # `$(rootpath)` for main-workspace targets is `cli/cli/dx` but the
  # runfiles tree nests it under `_main/` (`$TEST_SRCDIR/_main/cli/cli/dx`);
  # external repos arrive as `../<repo>/<path>` (execroot-relative) but live
  # at `$TEST_SRCDIR/<repo>/<path>`. Probe all layouts plus the
  # workspace `bazel-bin/` fallback for `bazel run` invocations.
  local rel="$1" stripped base candidates=()
  stripped="$(printf '%s' "$rel" | sed -e 's|^\(\.\./\)*||')"
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    candidates+=("$TEST_SRCDIR/_main/$rel" "$TEST_SRCDIR/$rel" "$TEST_SRCDIR/$stripped")
  fi
  # `bazel run` fallback: cwd is the workspace after the cd above, and
  # bazel-bin holds the built binaries.
  candidates+=("$(pwd)/bazel-bin/$rel" "$(pwd)/bazel-bin/$stripped" "$(pwd)/$rel")
  for base in "${candidates[@]}"; do
    if [[ -e "$base" ]]; then
      echo "$base"
      return 0
    fi
  done
  return 1
}

# Pin guard: the child Bazel must track //.bazelversion exactly (no
# multi-version matrix, issue #55). The MODULE download pin and the
# version file move together or this fails before staging anything.
pin_version="$(cat .bazelversion)"
[[ "$pin_version" == "9.2.0" ]] || { bad ".bazelversion is $pin_version, want 9.2.0"; }
grep -q -F -e 'bazel_binaries.download(version = "9.2.0")' MODULE.bazel \
  || { bad "MODULE.bazel lost the 9.2.0 bazel_binaries pin"; }

dx_bin="$(resolve_runfile "$dx_rel")" || { bad "dx binary not found: $dx_rel"; }
bazel_wrapper="$(resolve_runfile "$bazelw_rel")" || { bad "bazel wrapper not found: $bazelw_rel"; }
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

# The staged child must start dirty (`x=1`): otherwise the round-trip
# proves nothing (step 1 would pass on an already-clean tree).
if grep -q -F -e "x=1" "$scratch/child/dirty.bzl"; then
  ok
else
  bad "staged dirty.bzl lost its dirty content (want 'x=1')"
fi

mkdir -p "$scratch/bin" "$scratch/bazelisk_home" "$scratch/home"
ln -s "$bazel_wrapper" "$scratch/bin/bazel"

child_env() {
  env PATH="$scratch/bin:$PATH" \
    BAZELISK_HOME="$scratch/bazelisk_home" \
    HOME="$scratch/home" \
    "$@"
}

# Step 1: check mode fails closed on the dirty source (exit 1, no writes).
out=""
rc=0
out="$(cd "$scratch/child" && child_env "$dx_bin" format --check //... 2>&1)" || rc=$?
if [[ "$rc" == "1" ]]; then
  ok
else
  bad "dx format --check //... exited $rc, want 1 (dirty 'x=1')"
fi
echo "--- check-dirty output (tail) ---"
echo "$out" | tail -n 8
if grep -q -F -e "x=1" "$scratch/child/dirty.bzl"; then
  ok
else
  bad "check mode mutated dirty.bzl (check must never write)"
fi

# Step 2: default mode fixes the source (exit 0) and rewrites `x=1` to `x = 1`.
out=""
rc=0
out="$(cd "$scratch/child" && child_env "$dx_bin" format //... 2>&1)" || rc=$?
if [[ "$rc" == "0" ]]; then
  ok
else
  bad "dx format //... exited $rc, want 0"
fi
echo "--- format output (tail) ---"
echo "$out" | tail -n 8
if grep -q -F -e "x = 1" "$scratch/child/dirty.bzl"; then
  ok
else
  bad "dx format did not rewrite dirty.bzl to 'x = 1'; content: $(cat "$scratch/child/dirty.bzl")"
fi

# Step 3: check mode passes on the fixed tree (exit 0).
out=""
rc=0
out="$(cd "$scratch/child" && child_env "$dx_bin" format --check //... 2>&1)" || rc=$?
if [[ "$rc" == "0" ]]; then
  ok
else
  bad "dx format --check //... after fix exited $rc, want 0"
fi
echo "--- check-clean output (tail) ---"
echo "$out" | tail -n 8

echo "e2e $scenario: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
