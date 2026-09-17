#!/usr/bin/env bash
# Examples README audit (issue #85, slice 1): every per-foundation
# external-consumer example workspace records exact commands plus
# expected evidence in its own README, so examples teach and prove
# generation as a consumer.
#
# This harness machine-checks the documentation half verifiable on a
# clean tree today: each examples/adopt-*/ directory carries a
# README.md with a fenced shell block running bazel build and bazel
# test plus an Evidence section, and the examples index points at live
# examples plus the owning issue. Acquisition and laziness fixtures
# (no-install attribution, unused-foundation zero-work) stay open per
# #85 and are recorded as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_readme`,
# following //tools/ci:code_ownership.
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

# The index no longer carries the retired milestone stub: it points at
# live per-foundation examples plus the owning issue.
if grep -q -F -e 'adopt-rust' examples/README.md && grep -q -F -e 'issues/85' examples/README.md; then
  ok
else
  bad "examples/README.md must point at live adopt-* examples plus the owning issue #85"
fi
if grep -q -F -e 'No examples exist yet' examples/README.md; then
  bad "examples/README.md still carries the retired 'No examples exist yet' stub"
else
  ok
fi

# Each per-foundation workspace records commands plus evidence.
for dir in examples/adopt-*/; do
  readme="$dir/README.md"
  name="$(basename "$dir")"
  if [[ ! -f "$readme" ]]; then
    bad "$readme missing (each adopt-* workspace needs its own README)"
    continue
  fi
  ok
  if grep -q -F -e '```sh' "$readme" && grep -q -F -e 'bazel build' "$readme" && grep -q -F -e 'bazel test' "$readme"; then
    ok
  else
    bad "$readme must record exact commands (fenced sh with bazel build + bazel test)"
  fi
  if grep -q -F -e 'Evidence:' "$readme"; then
    ok
  else
    bad "$readme must record expected evidence (Evidence: section)"
  fi
done

echo "examples readme audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
