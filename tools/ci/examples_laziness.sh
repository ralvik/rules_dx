#!/usr/bin/env bash
# Examples laziness audit (issue #85, slice 2): managed-acquisition and
# unused-foundation zero-work, static half verifiable on a clean tree.
#
# The consumer contract (docs/tools/tool-acquisition.md) forbids ecosystem
# installers in the private tool graph (`pip install`, `npm install`,
# `cargo install`, `dotnet tool install`, ...), and the laziness matrix
# (docs/testing/tools.md) requires unused foundations to contribute zero
# targets/actions. Runtime attribution (process logs, aquery/exec-log
# proof over external consumers with network denied) stays open per #85
# and is recorded as a gap, not claimed here.
#
# This harness machine-checks the static half:
#  - no prohibited installer command appears in tool-implementation code
#    (quality/, tools/, language foundations, dx/, cli/);
#  - each single-foundation adopt-* example loads only its own
#    foundation wrapper (plus fixtures/ecosystem locks), never another
#    foundation's wrapper, so unused foundations contribute no targets.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_laziness`,
# following //tools/ci:examples_readme.
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

# No-install attribution, static half: prohibited installer commands must
# not appear in tool-implementation code. Docs legitimately discuss them
# (contract + matrix), and this harness names them as patterns, so both
# are excluded from the search scope.
hits="$(grep -rn -F -e 'pip install' -e 'npm install' -e 'cargo install' -e 'dotnet tool install' -e 'pnpm install' -e 'pnpm add' --include='*.bzl' --include='*.py' --include='*.rs' --include='*.sh' --include='*.js' --include='*.ts' quality/ tools/ rust/ python/ javascript/ typescript/ go/ java/ kotlin/ scala/ csharp/ fsharp/ cc/ dx/ cli/ 2>/dev/null | grep -v -F -e 'tools/ci/examples_laziness.sh' || true)"
if [[ -z "$hits" ]]; then
  ok
else
  bad "prohibited installer invocation in tool code: $(echo "$hits" | head -n 5)"
fi

# Unused-foundation zero-work, static half: each single-foundation
# example must load exactly its own foundation wrapper(s).
check_isolation() { # dir, want-foundation-list...
  local dir="$1"; shift
  local got
  got="$(grep -rh '^load' "$dir" --include='BUILD.bazel' 2>/dev/null | grep -o '@rules_dx//[a-z_]*/' | sed 's|@rules_dx//||; s|/||' | LC_ALL=C sort -u | tr '\n' ' ')"
  local want="$* "
  if [[ "$got" == "$want" ]]; then
    ok
  else
    bad "$dir loads [$got], want [$want]"
  fi
}

check_isolation examples/adopt-rust rust
check_isolation examples/adopt-python python
check_isolation examples/adopt-js-ts javascript typescript
check_isolation examples/adopt-go go
check_isolation examples/adopt-cpp cc
check_isolation examples/adopt-java java
check_isolation examples/adopt-kotlin kotlin
check_isolation examples/adopt-scala scala
check_isolation examples/adopt-csharp csharp
check_isolation examples/adopt-fsharp fsharp
check_isolation examples/adopt-polyglot javascript python rust typescript

echo "examples laziness audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
