#!/usr/bin/env bash
# Examples README audit (external-consumer slice 1): every per-foundation
# external-consumer example workspace records exact commands plus
# expected evidence in its own README, so examples teach and prove
# generation as a consumer.
#
# This harness machine-checks the documentation half verifiable on a
# clean tree today: each examples/adopt-*/ directory carries a
# README.md with a fenced shell block running bazel build and bazel
# test plus an Evidence section, the examples index points at live
# examples plus the open acquisition/laziness gap, every adopt-* workspace is indexed
# (no silent additions) and every indexed adopt-* link resolves (no
# stale links), and the mixed-framework fixture stays documented as
# non-consumer so it can neither drift into the audit silently nor
# lose its disposition note. Acquisition and laziness fixtures
# (no-install attribution, unused-foundation zero-work) remain open work
# and are recorded as gaps, not claimed here.
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
# live per-foundation examples plus the open acquisition/laziness gap.
if grep -q -F -e 'adopt-rust' examples/README.md && grep -q -F -e 'acquisition/laziness proof are open' examples/README.md; then
  ok
else
  bad "examples/README.md must point at live adopt-* examples plus the open acquisition/laziness gap"
fi
if grep -q -F -e 'No examples exist yet' examples/README.md; then
  bad "examples/README.md still carries the retired 'No examples exist yet' stub"
else
  ok
fi

# Each per-foundation workspace records commands plus evidence, and
# the index covers exactly the existing workspaces: every adopt-*
# directory is linked (additions cannot land unindexed) and every
# indexed adopt-* link resolves to a real directory (removals cannot
# leave stale links).
for dir in examples/adopt-*/; do
  readme="$dir/README.md"
  name="$(basename "$dir")"
  if [[ ! -f "$readme" ]]; then
    bad "$readme missing (each adopt-* workspace needs its own README)"
    continue
  fi
  ok
  if grep -q -F -e "($name/)" examples/README.md; then
    ok
  else
    bad "examples/$name/ exists but examples/README.md does not index it"
  fi
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

# Every indexed adopt-* link resolves: a removed workspace cannot
# leave a stale index entry behind.
while IFS= read -r link; do
  if [[ -d "examples/$link" ]]; then
    ok
  else
    bad "examples/README.md links [$link] but examples/$link/ does not exist"
  fi
done < <(grep -o -E -e '\]\((adopt-[a-z-]+)/\)' examples/README.md | sed 's/^](//; s|/)$||')

# The mixed-framework fixture is documented as non-consumer and stays
# out of the consumer-example audit: it is a framework-composition
# workspace (mixed/hello BUILD docstring names the M21 fixture), not
# an external-consumer workspace with commands plus evidence.
if grep -q -F -e 'mixed/hello' examples/README.md && ! grep -q -F -e '](mixed/' examples/README.md; then
  ok
else
  bad "examples/README.md must document mixed/hello as non-consumer without indexing it as an example"
fi
if grep -q -F -e 'M21 mixed-framework package' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "examples/mixed/hello/BUILD.bazel lost the M21 fixture disposition marker"
fi

echo "examples readme audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
