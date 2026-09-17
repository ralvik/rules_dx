#!/usr/bin/env bash
# Examples laziness aquery proof (issue #85, slice 4): unused-foundation
# zero-work via `bazel aquery` action graph over single-foundation adopt-*
# examples.
#
# The query harness (examples_laziness_query.sh) proves the
# dependency-closure half via `bazel query deps`. This harness proves the
# action-graph half: each single-foundation example's declared actions
# mention its own ecosystem repo and none of the other foundations'
# ecosystem repos, so unused foundations contribute zero actions to that
# consumer's build.
#
# Covered here (minimum per #85): Rust, Python, JS/TS. Markers are
# ecosystem-specific repo strings as they appear in aquery output
# (rules_rust / aspect_rules_py / aspect_rules_js / rules_go /
# rules_dotnet), not shared base toolchains.
#
# Still open per #85 (recorded as gap, not claimed): network-denied
# runtime attribution (process/exec-log proof over external consumers)
# plus empty-cache remote-cache execution proof. This harness is
# action-graph only, no execution.
#
# Run by CI via `bazel run //tools/ci:examples_laziness_aquery`,
# after //tools/ci:examples_laziness_query.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

check_example() { # example, want-marker, forbidden-markers...
  local example="$1" want="$2"; shift 2
  local actions
  if ! actions="$(bazel aquery "//examples/$example/..." --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel aquery failed"
    return
  fi
  if [[ -z "$actions" ]]; then
    bad "$example: empty aquery output"
    return
  fi
  if [[ "$actions" == *"$want"* ]]; then
    ok
  else
    bad "$example: want marker [$want] in aquery actions"
  fi
  local marker
  for marker in "$@"; do
    if [[ "$actions" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in aquery actions (unused foundation leaks)"
    else
      ok
    fi
  done
}

# Rust: owns rules_rust; Python/JS/Go/DotNet contribute no actions.
check_example adopt-rust "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet"
# Python: owns aspect_rules_py; Rust/JS/Go/DotNet contribute no actions.
check_example adopt-python "aspect_rules_py" "rules_rust" "aspect_rules_js" "rules_go" "rules_dotnet"
# JS/TS: owns aspect_rules_js; Rust/Python/Go/DotNet contribute no actions.
check_example adopt-js-ts "aspect_rules_js" "rules_rust" "aspect_rules_py" "rules_go" "rules_dotnet"

echo "examples laziness aquery: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
