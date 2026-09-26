#!/usr/bin/env bash
set -euo pipefail

_snapshot_workspace_root() {
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY"
    return 0
  fi
  git rev-parse --show-toplevel 2>/dev/null
}

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

snapshot_canonical_json_diff() {
  local expected="$1" actual="$2" workspace_rel="${3:-}"
  local tmp
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/snapshot.XXXXXX")"
  # LCOV_EXCL_START - reason: trap cleanup, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
  trap 'rm -rf "$tmp"' RETURN
  # LCOV_EXCL_STOP - reason: end trap cleanup, issue: 1055, policy: docs/cli/commands/build-test-coverage.md
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

snapshot_json_validates() {
  local file="$1"
  python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$file"
}
