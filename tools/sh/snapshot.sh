#!/usr/bin/env bash
# Shared snapshot helper (issues #322, #450).
#
# Single-sources the snapshot-testing workflow for shell golden harnesses:
# byte-identical snapshot comparison with an UPDATE_EXPECT refresh path,
# plus canonical-JSON comparison and JSON-schema shape checks so brittle
# equality becomes an explicit snapshot or a schema contract.
#
# Guard maintenance owns shared helpers plus snapshot versus grep policy
# under issue #450: this file owns golden-byte asserts (UPDATE_EXPECT
# refresh); `tools/sh/lib.sh` `dx_expect_*` owns fixed-string doc/code
# contract pins (fail-closed, no refresh). Guards must not reimplement
# either shape; `//tools/ci:shell_contract` owns the rule.
#
# Drivers source this file via a runfiles-first bootstrap so both direct
# execution and Bazel `run`/`test` layouts work (`data =
# ["//tools/sh:snapshot"]` carries it in the runfiles forest):
#
#   source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/snapshot.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/snapshot.sh"
#
# (adjust the trailing depth for the source-tree fallback: repo-root
# parity tests use `tools/sh/snapshot.sh`, `.devcontainer/*.sh` use
# `../tools/sh/snapshot.sh`.)
#
# Workflow:
#   UPDATE_EXPECT=1 bazel test //:preset_parity_test --test_env=UPDATE_EXPECT
# refreshes the checked-in golden instead of failing; without the variable
# the harness fails with a unified diff. Under `bazel run` the update writes
# through BUILD_WORKSPACE_DIRECTORY; under `bazel test` it resolves the
# checkout via git and writes the workspace-relative path when given, else
# stages the refreshed bytes under TEST_UNDECLARED_OUTPUTS_DIR with
# copy-paste instructions.
#
# Provides:
#   snapshot_diff <expected> <actual> [<workspace_rel>]
#     byte-identical snapshot assert with UPDATE_EXPECT refresh.
#   snapshot_canonical_json_diff <expected> <actual> [<workspace_rel>]
#     canonical-JSON (sorted keys) snapshot assert with UPDATE_EXPECT.
#   snapshot_json_validates <file>
#     fails unless the file parses as JSON (python3 stdlib only).
#
# Bash-only Linux harness (issues #299, #450): sourced by `sh_binary` /
# `sh_test` drivers carrying `target_compatible_with =
# ["@platforms//os:linux"]`. Bootstrap requires bash by design under issue
# #450 (`BASH_SOURCE` plus the 5-way runfiles fallback never run under
# POSIX `sh`).
set -euo pipefail

# Resolve the checkout root for UPDATE_EXPECT writes: BUILD_WORKSPACE_DIRECTORY
# under `bazel run`, else the enclosing git top-level. Prints nothing and
# fails when neither is available.
_snapshot_workspace_root() {
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY"
    return 0
  fi
  git rev-parse --show-toplevel 2>/dev/null
}

# Stage refreshed bytes when the source cannot be written directly (sandboxed
# `bazel test` without a workspace-relative path). Prints the staged path.
_snapshot_stage_update() {
  local actual="$1" label="$2"
  local out_dir="${TEST_UNDECLARED_OUTPUTS_DIR:-}"
  if [[ -z "$out_dir" ]]; then
    out_dir="${TMPDIR:-/tmp}"
  fi
  mkdir -p "$out_dir"
  local staged="$out_dir/${label}.expected.update"
  cp "$actual" "$staged"
  printf '%s\n' "$staged"
}

# snapshot_diff <expected> <actual> [<workspace_rel>]
# Byte-identical snapshot assert. Passes silently (echoes PASS) when equal.
# On mismatch: with UPDATE_EXPECT=1 refreshes the golden and passes;
# otherwise prints `diff -u` and fails with refresh instructions.
snapshot_diff() {
  local expected="$1" actual="$2" workspace_rel="${3:-}"
  if cmp -s "$expected" "$actual"; then
    echo "snapshot PASS: $expected matches"
    return 0
  fi
  if [[ "${UPDATE_EXPECT:-0}" == "1" ]]; then
    local root=""
    root="$(_snapshot_workspace_root 2>/dev/null || true)"
    if [[ -n "$workspace_rel" && -n "$root" && -d "$root" ]]; then
      mkdir -p "$(dirname "$root/$workspace_rel")"
      cp "$actual" "$root/$workspace_rel"
      echo "snapshot UPDATE_EXPECT: refreshed $workspace_rel from actual"
      return 0
    fi
    # Fall back to overwriting the expected path when writable (direct
    # execution outside the Bazel sandbox), else stage for manual copy.
    if [[ -w "$expected" ]] && [[ "$expected" != *"/runfiles/"* ]]; then
      cp "$actual" "$expected"
      echo "snapshot UPDATE_EXPECT: refreshed $expected from actual"
      return 0
    fi
    local staged
    staged="$(_snapshot_stage_update "$actual" "$(basename "$expected")")"
    echo "snapshot UPDATE_EXPECT: staged refreshed golden at $staged" >&2
    if [[ -n "$workspace_rel" ]]; then
      echo "copy it to $workspace_rel in the checkout, then re-run." >&2
    else
      echo "copy it over the expected file, then re-run." >&2
    fi
    return 0
  fi
  diff -u "$expected" "$actual" || true
  echo "snapshot FAIL: $expected differs from actual (see diff above)" >&2
  echo "re-run with UPDATE_EXPECT=1 to refresh the golden (bazel test --test_env=UPDATE_EXPECT), then review the diff before committing." >&2
  return 1
}

# snapshot_canonical_json_diff <expected> <actual> [<workspace_rel>]
# Canonical-JSON snapshot assert: both files must parse as JSON; comparison
# uses sorted keys and 2-space indent so key order and trailing-newline noise
# do not fail the test. UPDATE_EXPECT refreshes with the canonical form.
snapshot_canonical_json_diff() {
  local expected="$1" actual="$2" workspace_rel="${3:-}"
  local tmp
  tmp="$(mktemp -d)"
  # LCOV_EXCL_START - reason: hermetic harness failure path; covered by negative manual runs, not line coverage.
  trap 'rm -rf "$tmp"' RETURN
  # LCOV_EXCL_STOP
  if ! python3 -c 'import json,sys; json.load(open(sys.argv[1])); json.load(open(sys.argv[2]))' "$expected" "$actual"; then
    echo "snapshot FAIL: non-JSON input ($expected vs $actual)" >&2
    return 1
  fi
  python3 -c 'import json,sys; json.dump(json.load(open(sys.argv[1])), open(sys.argv[2],"w"), sort_keys=True, indent=2); open(sys.argv[2],"a").write("\n")' "$expected" "$tmp/expected.canonical"
  python3 -c 'import json,sys; json.dump(json.load(open(sys.argv[1])), open(sys.argv[2],"w"), sort_keys=True, indent=2); open(sys.argv[2],"a").write("\n")' "$actual" "$tmp/actual.canonical"
  if cmp -s "$tmp/expected.canonical" "$tmp/actual.canonical"; then
    echo "snapshot PASS (canonical JSON): $expected matches"
    return 0
  fi
  if [[ "${UPDATE_EXPECT:-0}" == "1" ]]; then
    local root=""
    root="$(_snapshot_workspace_root 2>/dev/null || true)"
    if [[ -n "$workspace_rel" && -n "$root" && -d "$root" ]]; then
      cp "$tmp/actual.canonical" "$root/$workspace_rel"
      echo "snapshot UPDATE_EXPECT: refreshed $workspace_rel (canonical JSON)"
      return 0
    fi
    if [[ -w "$expected" ]] && [[ "$expected" != *"/runfiles/"* ]]; then
      cp "$tmp/actual.canonical" "$expected"
      echo "snapshot UPDATE_EXPECT: refreshed $expected (canonical JSON)"
      return 0
    fi
    local staged
    staged="$(_snapshot_stage_update "$tmp/actual.canonical" "$(basename "$expected")")"
    echo "snapshot UPDATE_EXPECT: staged canonical JSON at $staged" >&2
    return 0
  fi
  diff -u "$tmp/expected.canonical" "$tmp/actual.canonical" || true
  echo "snapshot FAIL (canonical JSON): $expected differs (see diff above)" >&2
  echo "re-run with UPDATE_EXPECT=1 to refresh the golden, then review before committing." >&2
  return 1
}

# snapshot_json_validates <file>
# Fails unless the file parses as JSON. Thin wrapper so harnesses share one
# python3-only schema precondition without reimplementing it.
snapshot_json_validates() {
  local file="$1"
  python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$file"
}
