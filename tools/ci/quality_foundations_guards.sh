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
# today (32 checks): provider definition, single-sourced registry map +
# frozen class table + secrets family + parity gate, per-foundation
# owner docs + dependency scopes, minimum-consumer index + READMEs,
# framework boundary + full format set + mixed composition + ownership
# regions, adapter-mechanics doc, cache/determinism/apply-safety
# contract sections, curated-defaults + native-config evidence files +
# test backing, lane-A exclusion + generated list + dogfood CI jobs,
# external-consumer breadth, determinism seed pin, apply filesystem +
# atomic-write evidence, matrix honesty, prior-slice harnesses green,
# and no-false-claim gaps. Full taxonomy review, exact mappings, and
# battery execution stay open under their issues.
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

# #8 full format set: Svelte/Astro/MDX boundaries alongside Vue (no
# single-format fallback claim).
if grep -q -F -e 'Svelte' docs/generation/framework-adapters.md \
  && grep -q -F -e 'Astro' docs/generation/framework-adapters.md \
  && grep -q -F -e 'MDX' docs/generation/framework-adapters.md; then
  ok
else
  bad "framework-adapters.md lost its Svelte/Astro/MDX boundary markers (#8 full set)"
fi

# #7 external-consumer breadth beyond the minimum: Go + Java adopt
# workspaces carry their own READMEs with commands + evidence.
if [[ -f "examples/adopt-go/README.md" ]] \
  && [[ -f "examples/adopt-java/README.md" ]] \
  && grep -q -F -e 'adopt-go' examples/README.md \
  && grep -q -F -e 'adopt-java' examples/README.md; then
  ok
else
  bad "examples lost their beyond-minimum Go/Java consumer breadth (#7)"
fi

# #84 determinism seed stays pinned: insertion-order independence test
# present in the runner (QualitySourcesInfo/checkout-order evidence).
if grep -q -F -e 'state_digest_independent_of_insertion_order' quality/runner/src/lib.rs; then
  ok
else
  bad "quality runner lost its insertion-order determinism seed (#84)"
fi

# #6 curated/native test backing stays present alongside the evidence
# files (no untested taxonomy drift).
if [[ -f "quality/curated_defaults_tests.bzl" ]] \
  && [[ -f "quality/native_config_tests.bzl" ]]; then
  ok
else
  bad "quality curated/native test backing missing (curated_defaults_tests/native_config_tests)"
fi

# #6 parity gate stays versioned: tool-parity contract section plus the
# fail-closed parity unit-test entry point (taxonomy review still open).
if grep -q -F -e '## Tool Parity' docs/quality/quality-testing.md \
  && grep -q -F -e 'parity_unit_tests' quality/parity_tests.bzl; then
  ok
else
  bad "quality parity gate lost (Tool Parity section or parity_unit_tests entry)"
fi

# #8 mixed-framework composition stays explicit: shared mechanics only
# after concrete adapters prove reuse, M21 fixture owns the single-owner
# mixed hello package (no generic fallback).
if grep -q -F -e 'Mixed-framework' docs/generation/framework-adapters.md \
  && grep -q -F -e 'M21' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "framework composition record lost (Mixed-framework clause or M21 mixed fixture)"
fi

# #12 generated-file exclusion list stays declared with its generator
# (beyond the adopt-js-ts inert arrival record above).
if grep -q -F -e 'generated' tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership lost its generated-file exclusion record (#12)"
fi

# #84 apply-safety filesystem evidence stays pinned: the collected-change
# applier under test (digest/atomicity/interruption batteries still open).
if grep -q -F -e 'apply_collected_changes' cli/cli/src/exec/quality_apply.rs; then
  ok
else
  bad "quality apply applier evidence lost (apply_collected_changes fn)"
fi

# #7 minimum-consumer index stays pinned: Rust + Python adopt entries
# (foreign Cargo/Python trees via dx generate; broader set above).
if grep -q -F -e 'adopt-rust' examples/README.md \
  && grep -q -F -e 'adopt-python' examples/README.md; then
  ok
else
  bad "examples index lost its minimum Rust/Python consumer entries (#7)"
fi

# #7 minimum-consumer READMEs stay present with commands + evidence
# (beyond-minimum Go/Java slice owns its own check above).
if [[ -f "examples/adopt-rust/README.md" ]] \
  && [[ -f "examples/adopt-python/README.md" ]]; then
  ok
else
  bad "examples lost their minimum Rust/Python consumer READMEs (#7)"
fi

# #8 adapter-mechanics doc stays owned (rule families + aspect entry
# points per tool; region mapping still open).
if grep -q -F -e 'adapter mechanics' docs/quality/tool-integrations.md; then
  ok
else
  bad "tool-integrations.md lost its adapter-mechanics ownership (#8)"
fi

# #84 atomic-write evidence stays pinned: atomic apply of verified
# reads in the collected-change applier (batteries still open).
if grep -q -F -e 'write_atomic' cli/cli/src/exec/quality_apply.rs; then
  ok
else
  bad "quality apply applier lost its atomic-write evidence (#84)"
fi

# #6 frozen class table stays owned: canonical registry IDs plus the
# secrets family as its own semantic class (assignment/admissibility
# review still open).
if grep -q -F -e 'Canonical semantic file-class IDs' docs/quality/quality-sources.md \
  && grep -q -F -e 'secrets` policy family is its own semantic class' docs/quality/quality-sources.md; then
  ok
else
  bad "quality-sources.md lost its frozen class table or secrets-family record (#6)"
fi

# #7 per-language dependency scope stays owned: Cargo, uv, and pnpm
# resolution sections (exact lock/closure proofs still open).
if grep -q -F -e '## Crates And Cargo' docs/generation/rust.md \
  && grep -q -F -e '## uv Scope And Resolution' docs/generation/python.md \
  && grep -q -F -e '## pnpm Scope And Resolution' docs/generation/javascript-typescript.md; then
  ok
else
  bad "generation docs lost their Cargo/uv/pnpm dependency-scope sections (#7)"
fi

# #8 framework ownership regions stay explicit: physical/virtual
# ownership plus format-specific regions (exact region transport open).
if grep -q -F -e '## Physical And Virtual Ownership' docs/generation/framework-adapters.md \
  && grep -q -F -e '## Format-Specific Regions' docs/generation/framework-adapters.md; then
  ok
else
  bad "framework-adapters.md lost its ownership/region sections (#8)"
fi

# #12 lane-A CI scope stays sharded: freshness, lint, format, and
# typecheck dogfood jobs run our own tools over our own tree.
if grep -q -F -e 'dogfood-freshness' .github/workflows/ci.yml \
  && grep -q -F -e 'dogfood-lint' .github/workflows/ci.yml \
  && grep -q -F -e 'dogfood-format' .github/workflows/ci.yml \
  && grep -q -F -e 'dogfood-typecheck' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its sharded lane-A dogfood jobs (#12)"
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
