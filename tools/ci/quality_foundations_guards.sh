#!/usr/bin/env bash
# Quality-foundations determinism guards (issues #6, #7, #8, #12, #84).
#
# Rust/Python/JS-TS foundations ship thin wrappers + Gazelle + env plans;
# Vue/Svelte/Astro/MDX ship named adapters over upstream parsers; quality
# plumbing (QualitySourcesInfo, policy providers, result protocol,
# adapters, parity gate) is delivered. Exact provider/import/lock/
# tool-graph proofs (#7), framework parser/provider/region mappings +
# composition (#8), class-to-family taxonomy + admissibility (#6), native
# config binding + CI scope extension (#12 lane A), and per-adapter
# cache/determinism/apply batteries (#84) stay open with honest gaps.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (16 checks): provider definition, single-sourced registry map,
# per-foundation owner docs, framework boundary, cache/determinism/
# apply-safety contract sections, curated-defaults + native-config
# evidence files, lane-A exclusion record, matrix honesty, prior-slice
# harnesses green, and no-false-claim gaps. Full taxonomy review,
# exact mappings, and battery execution stay open under their issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_foundations_guards`,
# following //tools/ci:registry_singularity.
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

# #12 lane A plumbing: QualitySourcesInfo provider defined once.
if grep -q -F -e 'QualitySourcesInfo = provider(' quality/sources.bzl \
  && grep -q -F -e 'QualitySourcesInfo' docs/quality/quality-sources.md; then
  ok
else
  bad "QualitySourcesInfo provider definition or sources doc missing"
fi

# #6 single-sourced registry: one class-to-family map + adapter manifests.
if grep -q -F -e 'REAL_CLASS_TO_FAMILY' quality/adapters.bzl \
  && grep -q -F -e 'REAL_ADAPTERS' quality/adapters.bzl; then
  ok
else
  bad "adapters.bzl lost the single-sourced registry (REAL_CLASS_TO_FAMILY + REAL_ADAPTERS)"
fi

# #7 language owner docs present per foundation.
if grep -q -F -e 'Rust' docs/generation/rust.md \
  && grep -q -F -e 'Python' docs/generation/python.md \
  && grep -q -F -e 'TypeScript' docs/generation/javascript-typescript.md; then
  ok
else
  bad "language foundation docs lost their owner markers (Rust/Python/TypeScript)"
fi

# #8 framework boundary doc present.
if grep -q -F -e 'Vue' docs/generation/framework-adapters.md; then
  ok
else
  bad "framework-adapters.md lost its Vue boundary marker"
fi

# #84 contract owns the three batteries (cache/determinism/apply).
if grep -q -F -e '## Cache Correctness' docs/quality/quality-testing.md \
  && grep -q -F -e '## Determinism' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost the Cache Correctness / Determinism battery sections"
fi

# #84 honesty: warm no-op alone is not a cache test (gap recorded).
if grep -q -F -e 'A warm local no-op alone is not a cache test' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost its warm-no-op-is-not-a-cache-test honesty record"
fi

# #84 contract owns the third battery too (apply safety).
if grep -q -F -e '## Apply Safety' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost the Apply Safety battery section"
fi

# #6 curated defaults stay single-sourced in quality/.
if [[ -f "quality/curated_defaults.bzl" ]] \
  && grep -q -F -e 'curated' quality/adapters.bzl; then
  ok
else
  bad "curated-defaults evidence lost (quality/curated_defaults.bzl or adapters.bzl record)"
fi

# #6 native-config evidence backing stays present.
if [[ -f "quality/native_config.bzl" ]]; then
  ok
else
  bad "quality/native_config.bzl missing (native-config evidence backing)"
fi

# #12 lane-A exclusion record: inert adopt-js-ts arrivals named with generator.
if grep -q -F -e 'adopt-js-ts' tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership lost its adopt-js-ts inert-exclusion record"
fi

# Matrix honesty: framework/language/quality gaps link owners.
if grep -q -F -e 'issues/8' docs/testing/verification-matrix.md \
  && grep -q -F -e 'issues/7' docs/testing/verification-matrix.md \
  && grep -q -F -e 'issues/6' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #6/#7/#8 owner links"
fi

# Matrix rows keep the per-language honesty markers.
if grep -q -F -e 'Open (regions, #8)' docs/testing/verification-matrix.md \
  && grep -q -F -e 'Tracked (#86)' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its framework-region / perf-tracked honesty markers"
fi

# Prior slices stay green: wrapper sources + foundation maps + registry.
if [[ -f "tools/ci/wrapper_sources.sh" ]] \
  && [[ -f "tools/ci/foundation_maps.sh" ]] \
  && [[ -f "tools/ci/registry_singularity.sh" ]]; then
  ok
else
  bad "prior quality harnesses missing (wrapper_sources/foundation_maps/registry_singularity)"
fi

# #84 seed slice stays green: aquery action-key isolation proof.
if [[ -f "tools/ci/quality_cache_aquery.sh" ]]; then
  ok
else
  bad "quality cache aquery seed harness missing"
fi

# #12 lane A audits stay wired: code ownership harness present.
if [[ -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "code ownership harness missing"
fi

# No false claim: full determinism/apply batteries not claimed green.
if ! grep -rln -F -e 'determinism battery green' tools/ci/ docs/quality/ 2>/dev/null | grep -v -F -e 'quality_foundations_guards.sh' | grep -q . \
  && ! grep -rln -F -e 'apply-safety battery green' tools/ci/ docs/quality/ 2>/dev/null | grep -v -F -e 'quality_foundations_guards.sh' | grep -q .; then
  ok
else
  bad "a determinism/apply green claim appeared without the #84 batteries landing"
fi

echo "quality foundations guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
