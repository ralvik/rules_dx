#!/usr/bin/env bash
# Quality-foundations determinism guards (issues #6, #7, #8, #12, #84).
#
# Functional code checks only (no docs-prose guards).
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (27 checks): provider definition, single-sourced registry map,
# curated-defaults + native-config evidence files + test backing,
# lane-A exclusion + generated list, prior-slice harnesses,
# external-consumer breadth, determinism seed pin, apply filesystem +
# atomic-write evidence, aspect QualitySourcesInfo gate, no-cache argv
# marker, Gazelle extension dirs, dogfood CI jobs, M21 fixture, parity
# unit tests, lane-A forwarder plumbing + language-tree CI scope,
# and no-false-claim gaps.
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_foundations_guards`,
# following //tools/ci:registry_singularity.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# #12 lane A plumbing: QualitySourcesInfo provider defined once.
if grep -q -F -e 'QualitySourcesInfo = provider(' quality/sources.bzl; then
  ok
else
  bad "QualitySourcesInfo provider definition missing"
fi

# #6 single-sourced registry: one class-to-family map + adapter manifests.
if grep -q -F -e 'REAL_CLASS_TO_FAMILY' quality/adapters.bzl &&
  grep -q -F -e 'REAL_ADAPTERS' quality/adapters.bzl; then
  ok
else
  bad "adapters.bzl lost the single-sourced registry (REAL_CLASS_TO_FAMILY + REAL_ADAPTERS)"
fi

# #6 curated defaults stay single-sourced in quality/.
if [[ -f "quality/curated_defaults.bzl" ]] &&
  grep -q -F -e 'curated' quality/adapters.bzl; then
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

# Prior slices stay green: wrapper sources + foundation maps + registry.
if [[ -f "tools/ci/wrapper_sources.sh" ]] &&
  [[ -f "tools/ci/foundation_maps.sh" ]] &&
  [[ -f "tools/ci/registry_singularity.sh" ]]; then
  ok
else
  bad "prior quality harnesses missing (wrapper_sources/foundation_maps/registry_singularity)"
fi

# #84 cache proof stays pinned: aquery action-key isolation plus local
# execution-log executed-vs-cached proof.
if [[ -f "tools/ci/quality_cache_aquery.sh" ]] &&
  grep -q -F -e 'execution_log_json_file' tools/ci/quality_cache_aquery.sh; then
  ok
else
  bad "quality cache aquery harness missing its execution-log proof (#84)"
fi

# #12 lane A audits stay wired: code ownership harness present.
if [[ -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "code ownership harness missing"
fi

# #7 external-consumer breadth beyond the minimum: Go + Java adopt
# workspaces carry their own READMEs with commands + evidence.
if [[ -f "examples/adopt-go/README.md" ]] &&
  [[ -f "examples/adopt-java/README.md" ]] &&
  grep -q -F -e 'adopt-go' examples/README.md &&
  grep -q -F -e 'adopt-java' examples/README.md; then
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
if [[ -f "quality/curated_defaults_tests.bzl" ]] &&
  [[ -f "quality/native_config_tests.bzl" ]]; then
  ok
else
  bad "quality curated/native test backing missing (curated_defaults_tests/native_config_tests)"
fi

# #6 parity gate stays versioned: fail-closed parity unit-test entry point.
if grep -q -F -e 'parity_unit_tests' quality/parity_tests.bzl; then
  ok
else
  bad "quality parity gate lost (parity_unit_tests entry)"
fi

# M21 fixture owns the single-owner mixed hello package.
if grep -q -F -e 'M21' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "framework composition record lost (M21 mixed fixture)"
fi

# #12 generated-file exclusion list stays declared with its generator
# (beyond the adopt-js-ts inert arrival record above).
if grep -q -F -e 'generated' tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership lost its generated-file exclusion record (#12)"
fi

# #84 apply-safety filesystem evidence stays pinned: the collected-change
# applier under test (digest/atomicity/interruption batteries).
if grep -q -F -e 'apply_collected_changes' cli/cli/src/exec/quality_apply.rs; then
  ok
else
  bad "quality apply applier evidence lost (apply_collected_changes fn)"
fi

# #7 minimum-consumer index stays pinned: Rust + Python adopt entries
# (foreign Cargo/Python trees via dx generate; broader set above).
if grep -q -F -e 'adopt-rust' examples/README.md &&
  grep -q -F -e 'adopt-python' examples/README.md; then
  ok
else
  bad "examples index lost its minimum Rust/Python consumer entries (#7)"
fi

# #7 minimum-consumer READMEs stay present with commands + evidence
# (beyond-minimum Go/Java slice owns its own check above).
if [[ -f "examples/adopt-rust/README.md" ]] &&
  [[ -f "examples/adopt-python/README.md" ]]; then
  ok
else
  bad "examples lost their minimum Rust/Python consumer READMEs (#7)"
fi

# #84 atomic-write evidence stays pinned: atomic apply of verified
# reads in the collected-change applier plus mode-preserving writes.
if grep -q -F -e 'write_atomic' cli/cli/src/exec/quality_apply.rs &&
  grep -q -F -e 'preserves_existing_mode_on_overwrite' cli/atomic_fs/src/lib.rs; then
  ok
else
  bad "quality apply applier lost its atomic-write evidence (#84)"
fi

# #12 lane-A CI scope stays sharded: freshness, lint, format, and
# typecheck dogfood jobs run our own tools over our own tree.
if grep -q -F -e 'dogfood-freshness' .github/workflows/ci.yml &&
  grep -q -F -e 'dogfood-lint' .github/workflows/ci.yml &&
  grep -q -F -e 'dogfood-format' .github/workflows/ci.yml &&
  grep -q -F -e 'dogfood-typecheck' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its sharded lane-A dogfood jobs (#12)"
fi

# #7 Gazelle language extensions stay present: one extension directory
# per delivered foundation (upstream rules stay the implementation;
# exact import/lock proofs pinned in //tools/ci:foundation_maps).
if [[ -d "gazelle/rust" ]] &&
  [[ -d "gazelle/python" ]] &&
  [[ -d "gazelle/typescript" ]]; then
  ok
else
  bad "Gazelle lost a delivered-foundation extension dir (rust/python/typescript, #7)"
fi

# #8 Gazelle framework extensions stay present alongside the language
# set (named adapters over upstream parsers, no generic fallback).
if [[ -d "gazelle/vue" ]] &&
  [[ -d "gazelle/svelte" ]] &&
  [[ -d "gazelle/astro" ]] &&
  [[ -d "gazelle/mdx" ]]; then
  ok
else
  bad "Gazelle lost a framework extension dir (vue/svelte/astro/mdx, #8)"
fi

# #12 aspect gate stays provider-closed: aspects visit only targets
# carrying QualitySourcesInfo, never a parallel file list.
if grep -q -F -e 'QualitySourcesInfo not in target' quality/aspects.bzl; then
  ok
else
  bad "quality aspects lost their QualitySourcesInfo-only gate (#12)"
fi

# #84 cache-argv marker stays pinned: the runner threads an explicit
# no-cache argv through (full invalidation batteries still open).
if grep -q -F -e '--no-cache' quality/runner/src/real.rs; then
  ok
else
  bad "quality runner lost its --no-cache argv marker (#84)"
fi

# #12 provider-closed aspects stay fallback-free: only direct_sources
# supplies files, never a parallel list (proven language trees now run in
# CI alongside the corpus; broader trees stay open with no false claim).
if grep -q -F -e 'No generic fallback' quality/aspects.bzl; then
  ok
else
  bad "quality aspects lost their no-generic-fallback record (#12)"
fi

# #12 lane-A native-config binding stays forwarder-closed: hints ride the
# public QualitySourcesInfo owner across every wrapper family (shared
# dx_wrap plus the custom binary/test forwarders).
if grep -q -F -e 'aspect_hints' libs/starlark/wrapper.bzl &&
  grep -q -F -e 'aspect_hints' go/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' java/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' python/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' rust/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' javascript/rules/defs.bzl; then
  ok
else
  bad "wrappers lost their lane-A aspect_hints forwarder plumbing (#12)"
fi

# #12 lane-A CI scope covers the proven language trees alongside the
# corpus (enforcing at --fail-on warning).
if grep -q -F -e '//python/...' .github/workflows/ci.yml &&
  grep -q -F -e '//javascript/...' .github/workflows/ci.yml &&
  grep -q -F -e '//rust/tests/fixtures/hello/...' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its lane-A language-tree scope (#12)"
fi

# No false claim: full determinism/apply batteries not claimed green.
if ! grep -rln -F -e 'determinism battery green' tools/ci/ 2>/dev/null | grep -v -F -e 'quality_foundations_guards.sh' | grep -q . &&
  ! grep -rln -F -e 'apply-safety battery green' tools/ci/ 2>/dev/null | grep -v -F -e 'quality_foundations_guards.sh' | grep -q .; then
  ok
else
  bad "a determinism/apply green claim appeared without the #84 batteries landing"
fi

dx_test_summary "quality foundations guards harness"
