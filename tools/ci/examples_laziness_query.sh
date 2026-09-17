#!/usr/bin/env bash
# Examples laziness query proof (issue #85, slice 3): unused-foundation
# zero-work via `bazel query deps` over single-foundation adopt-* examples.
#
# The static harness (examples_laziness.sh) proves no installer commands
# in tool code plus per-foundation wrapper isolation. This harness proves
# the dependency-closure half: each single-foundation example's
# transitive closure contains its own ecosystem repo and none of the
# other foundations' ecosystem repos, so unused foundations contribute
# no targets/repos to that consumer's build.
#
# Covered here (minimum per #85 plus Go follow-up): Rust, Python, JS/TS,
# Go. Markers are
# ecosystem-specific Bazel repos, not shared base toolchains
# (bazel_tools/skylib/platforms/rules_java appear across closures and
# are intentionally not asserted here).
#
# Still open per #85 (recorded as gap, not claimed): runtime attribution
# (process/exec-log proof over external consumers with network denied)
# plus empty-cache action proof via aquery/exec-log. This harness is
# query-closure only.
#
# Run by CI via `bazel run //tools/ci:examples_laziness_query`,
# after //tools/ci:examples_laziness.
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
  local deps
  if ! deps="$(bazel query "deps(//examples/$example/...)" --noshow_progress 2>/dev/null)"; then
    bad "$example: bazel query failed"
    return
  fi
  if [[ "$deps" == *"$want"* ]]; then
    ok
  else
    bad "$example: want marker [$want] in deps closure"
  fi
  local marker
  for marker in "$@"; do
    if [[ "$deps" == *"$marker"* ]]; then
      bad "$example: forbidden marker [$marker] in deps closure (unused foundation leaks)"
    else
      ok
    fi
  done
}

# Rust: owns rules_rust; Python/JS/Go/DotNet contribute nothing.
check_example adopt-rust "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_go" "rules_dotnet"
# Python: owns aspect_rules_py; Rust/JS/Go/DotNet contribute nothing.
check_example adopt-python "aspect_rules_py" "rules_rust" "aspect_rules_js" "rules_go" "rules_dotnet"
# JS/TS: owns aspect_rules_js; Rust/Python/Go/DotNet contribute nothing.
check_example adopt-js-ts "aspect_rules_js" "rules_rust" "aspect_rules_py" "rules_go" "rules_dotnet"
# Go: owns rules_go; Rust/Python/JS/DotNet contribute nothing.
check_example adopt-go "rules_go" "rules_rust" "aspect_rules_py" "aspect_rules_js" "rules_dotnet"

echo "examples laziness query: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
