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
# examples plus the delivered acquisition/laziness proof, every adopt-* workspace is indexed
# (no silent additions) and every indexed adopt-* link resolves (no
# stale links), and the mixed-framework fixture stays documented as
# non-consumer so it can neither drift into the audit silently nor
# lose its disposition note. Acquisition and laziness fixtures
# (no-install attribution, unused-foundation zero-work) are delivered
# across the readme, static, query, aquery, and runtime harnesses,
# with platform/remote dimensions owned by /.
#
# Versioned here, run by CI via `bazel run //tools/ci:examples_readme`,
# following //tools/ci:code_ownership.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# The index no longer carries the retired milestone stub: it points at
# live per-foundation examples plus the delivered acquisition/laziness proof.
if grep -q -F -e 'adopt-rust' examples/README.md && grep -q -F -e 'acquisition/laziness proof are delivered' examples/README.md; then
  ok
else
  bad "examples/README.md must point at live adopt-* examples plus the delivered acquisition/laziness proof"
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
# workspace (mixed/hello BUILD docstring names the fixture), not
# an external-consumer workspace with commands plus evidence.
if grep -q -F -e 'mixed/hello' examples/README.md && ! grep -q -F -e '](mixed/' examples/README.md; then
  ok
else
  bad "examples/README.md must document mixed/hello as non-consumer without indexing it as an example"
fi
if grep -q -F -e 'Mixed-framework package' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "examples/mixed/hello/BUILD.bazel lost the fixture disposition marker"
fi

# Per-workspace build target counts stay pinned (issue #926): the README
# Evidence claim is the consumer contract, so count drift fails here
# instead of silently teaching stale numbers.
check_target_count() {
  local dir="$1" want="$2"
  if grep -q -F -e "Build covers $want targets" "$dir/README.md" ||
    grep -q -F -e "covers $want targets" "$dir/README.md"; then
    ok
  else
    bad "$dir/README.md lost its build-target pin (want $want targets, issue #926)"
  fi
}
check_target_count "examples/adopt-python" "15"
check_target_count "examples/adopt-rust" "14"
check_target_count "examples/adopt-go" "6"
check_target_count "examples/adopt-cpp" "11"
check_target_count "examples/adopt-csharp" "11"
check_target_count "examples/adopt-fsharp" "11"
check_target_count "examples/adopt-java" "11"
check_target_count "examples/adopt-kotlin" "11"
check_target_count "examples/adopt-scala" "11"
check_target_count "examples/adopt-js-ts" "28"
check_target_count "examples/adopt-polyglot" "25"

# Generator command shape stays pinned per workspace family (issue #926):
# Rust-family workspaces regenerate via `dx generate`, Gazelle-family
# workspaces via per-language `gazelle update`, polyglot via both.
check_generator() {
  local dir="$1" want="$2"
  if grep -q -F -e "$want" "$dir/README.md"; then
    ok
  else
    bad "$dir/README.md lost its generator pin (want $want, issue #926)"
  fi
}
check_generator "examples/adopt-rust" "dx -- generate //examples/adopt-rust/"
check_generator "examples/adopt-python" "//gazelle/python:gazelle"
check_generator "examples/adopt-go" "//gazelle/go:gazelle"
check_generator "examples/adopt-cpp" "//gazelle/cc:gazelle"
check_generator "examples/adopt-csharp" "//gazelle/csharp:gazelle"
check_generator "examples/adopt-fsharp" "//gazelle/fsharp:gazelle"
check_generator "examples/adopt-java" "//gazelle/java:gazelle"
check_generator "examples/adopt-kotlin" "//gazelle/kotlin:gazelle"
check_generator "examples/adopt-scala" "//gazelle/scala:gazelle"
check_generator "examples/adopt-js-ts" "//gazelle/javascript:gazelle"
check_generator "examples/adopt-js-ts" "//gazelle/typescript:gazelle"
check_generator "examples/adopt-polyglot" "dx -- generate //examples/adopt-polyglot/"

dx_test_summary "examples readme audit"
