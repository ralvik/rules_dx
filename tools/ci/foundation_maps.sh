#!/usr/bin/env bash
# Foundation-mapping guards (issues #7, #8; relates #6, #12).
#
# Rust/Python/JS-TS foundations ship thin wrappers + Gazelle + env plans;
# exact provider/import/lock/tool-graph proofs stay open under #7.
# Vue/Svelte/Astro/MDX ship named adapters over upstream parsers with an
# end-to-end fixture approach; exact parser/provider/region/dependency/
# test/env/quality mappings plus composition evidence stay open under #8.
# Class-to-family taxonomy stays open under #6; native config binding +
# CI scope extension stay open under #12.
#
# This harness machine-checks the boundary half verifiable on a clean
# tree today: owner links, adapter boundaries, provider
# advertisement, fixture markers. Exact mappings
# stay open under their issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:foundation_maps`,
# following //tools/ci:wrapper_sources.
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

# #7: minimum external-consumer example workspaces exist (Rust, Python,
# JS/TS); the rest of #85's per-foundation set builds on these.
if [[ -d "examples/adopt-rust" && -d "examples/adopt-python" \
  && -d "examples/adopt-js-ts" ]]; then
  ok
else
  bad "minimum per-foundation example workspaces missing (adopt-rust/python/js-ts)"
fi

# #7: language wrappers advertise QualitySourcesInfo ( aspects gate on it).
langs_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
  if ! grep -q -F -e 'QualitySourcesInfo' "$lang/rules/defs.bzl" 2>/dev/null; then
    langs_missing="$langs_missing $lang"
  fi
done
if [[ -z "$langs_missing" ]]; then
  ok
else
  bad "language wrappers lost QualitySourcesInfo:$langs_missing"
fi

# #8: all four framework wrappers advertise QualitySourcesInfo.
fw_missing=""
for fw in vue svelte astro mdx; do
  if ! grep -q -F -e 'QualitySourcesInfo' "$fw/rules/defs.bzl" 2>/dev/null; then
    fw_missing="$fw_missing $fw"
  fi
done
if [[ -z "$fw_missing" ]]; then
  ok
else
  bad "framework wrappers lost QualitySourcesInfo:$fw_missing"
fi

# #8: mixed-framework composition fixture stays marked (M21), distinct
# from external-consumer workspaces.
if [[ -d "examples/mixed/hello" ]] \
  && grep -q -F -e 'M21' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "mixed composition fixture lost its M21 marker"
fi

# #12: wrapper-sources pin + ownership audits stay versioned.
if [[ -f "tools/ci/wrapper_sources.sh" && -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "wrapper_sources/code_ownership harnesses missing"
fi

echo "foundation maps harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
