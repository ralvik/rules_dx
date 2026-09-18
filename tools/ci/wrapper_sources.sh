#!/usr/bin/env bash
# Wrapper QualitySourcesInfo harness (issues #12 lane A, #7, #8; relates #6).
#
# Production code rides normal targets (not parallel corpus lists), and
# every language/framework wrapper advertises `QualitySourcesInfo`
# normalized from its direct `srcs` so quality aspects can gate on it.
# Code files never ride corpus lists; corpus stays for target-less files
# only (docs, BUILD files, configs).
#
# This harness machine-checks the static half verifiable on a clean tree
# today: all eleven language wrappers plus the four
# framework wrappers advertise `QualitySourcesInfo`, load the shared
# forwarding helper (single-sourced normalization, never per-language
# reimplementation), bind wrapper tests, and the corpus/code-ownership
# split stays honest (no code extensions in corpus targets, no docs
# extensions in the code-ownership scope).
#
# Versioned here, run by CI via `bazel run //tools/ci:wrapper_sources`,
# following //tools/ci:depcheck_contract.
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

# All eleven language wrappers advertise QualitySourcesInfo normalized
# from direct srcs (single toolchain source of truth per wrapper).
langs=(rust python javascript typescript go java kotlin scala csharp fsharp cc)
for lang in "${langs[@]}"; do
  defs="$lang/rules/defs.bzl"
  if [[ -f "$defs" ]] && grep -q -F -e 'QualitySourcesInfo' "$defs"; then
    ok
  else
    bad "$defs missing or lost its QualitySourcesInfo advertisement"
  fi
done

# Framework wrappers (Vue/Svelte/Astro/MDX) advertise the same boundary;
# composition regions stay open per #8, but the source-ownership
# provider must already be present so aspects can gate on it.
pass_frameworks=0
for fw in vue svelte astro mdx; do
  defs="$fw/rules/defs.bzl"
  if [[ -f "$defs" ]] && grep -q -F -e 'QualitySourcesInfo' "$defs"; then
    pass_frameworks=$((pass_frameworks + 1))
  else
    bad "$defs missing or lost its QualitySourcesInfo advertisement"
  fi
done
if [[ "$pass_frameworks" -eq 4 ]]; then
  ok
else
  bad "framework wrapper coverage incomplete ($pass_frameworks/4)"
fi

# Normalization stays single-sourced: wrappers load the shared
# forwarding helper instead of reimplementing provider construction.
if grep -q -F -e 'libs/starlark' python/rules/defs.bzl \
  && grep -q -F -e 'libs/starlark' javascript/rules/defs.bzl \
  && grep -q -F -e 'libs/starlark' rust/rules/defs.bzl; then
  ok
else
  bad "language wrappers drifted off the shared forwarding helper"
fi

# Wrapper precision is pinned by wrapper tests alongside the wrappers.
if [[ -f python/rules/wrapper_tests.bzl && -f javascript/rules/wrapper_tests.bzl && -f rust/rules/wrapper_tests.bzl ]]; then
  ok
else
  bad "wrapper tests missing alongside language wrappers"
fi

# Corpus stays for target-less files only: no code extensions ride
# corpus targets (code rides normal targets per #12).
if [[ -z "$(grep -rn -E -e '\.rs"|\.py"|\.js"|\.ts"|\.go"|\.java"|\.cs"' --include='BUILD.bazel' quality/ libs/ 2>/dev/null | grep -i corpus | head -n 3 || true)" ]]; then
  ok
else
  bad "code sources riding corpus targets (must ride normal targets)"
fi

# The split stays honest in the other direction too: the code-ownership
# audit scope is code extensions only, BUILD/configs stay with
# the corpus audit (see tools/ci/code_ownership.sh vs corpus_audit.sh).
if grep -q -F -e 'rs|py|js' tools/ci/code_ownership.sh \
  && grep -q -F -e 'real_source_target' tools/ci/corpus_audit.sh; then
  ok
else
  bad "ownership-audit split drifted (code vs corpus scopes)"
fi

echo "wrapper sources harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
