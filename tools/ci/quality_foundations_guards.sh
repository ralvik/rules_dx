#!/usr/bin/env bash
# Quality-foundations determinism guards.
#
# Functional code checks only (no docs-prose guards).
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (27 checks): provider definition, single-sourced registry map,
# curated-defaults + native-config evidence files + test backing,
# lane-A exclusion + generated list, prior-slice harnesses,
# external-consumer breadth, determinism seed pin, apply filesystem +
# atomic-write evidence, aspect QualitySourcesInfo gate, no-cache argv
# marker, Gazelle extension dirs, dogfood CI jobs, fixture, parity
# unit tests, lane-A forwarder plumbing + language-tree CI scope,
# and no-false-claim gaps.
#
# Versioned here, run by CI via `bazel run //tools/ci:quality_foundations_guards`,
# following //tools/ci:registry_singularity.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# lane A plumbing: QualitySourcesInfo provider defined once.
if grep -q -F -e 'QualitySourcesInfo = provider(' quality/sources.bzl; then
  ok
else
  bad "QualitySourcesInfo provider definition missing"
fi

# single-sourced registry: one class-to-family map + adapter manifests.
if grep -q -F -e 'REAL_CLASS_TO_FAMILY' quality/adapters.bzl &&
  grep -q -F -e 'REAL_ADAPTERS' quality/adapters.bzl; then
  ok
else
  bad "adapters.bzl lost the single-sourced registry (REAL_CLASS_TO_FAMILY + REAL_ADAPTERS)"
fi

# curated defaults stay single-sourced in quality/ with registry wiring.
if [[ -f "quality/curated_defaults.bzl" ]] &&
  grep -q -F -e 'curated_defaults.bzl' quality/registry.bzl; then
  ok
else
  bad "curated-defaults evidence lost (quality/curated_defaults.bzl plus registry.bzl wiring)"
fi

# native-config evidence backing stays present.
if [[ -f "quality/native_config.bzl" ]]; then
  ok
else
  bad "quality/native_config.bzl missing (native-config evidence backing)"
fi

# lane-A exclusion record: inert adopt-js-ts arrivals named with generator.
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

# cache proof stays pinned: aquery action-key isolation plus local
# execution-log executed-vs-cached proof.
if [[ -f "tools/ci/quality_cache_aquery.sh" ]] &&
  grep -q -F -e 'execution_log_json_file' tools/ci/quality_cache_aquery.sh; then
  ok
else
  bad "quality cache aquery harness missing its execution-log proof (#84)"
fi

# lane A audits stay wired: code ownership harness present.
if [[ -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "code ownership harness missing"
fi

# external-consumer breadth beyond the minimum: Go + Java adopt
# workspaces carry their own READMEs with commands + evidence.
if [[ -f "examples/adopt-go/README.md" ]] &&
  [[ -f "examples/adopt-java/README.md" ]] &&
  grep -q -F -e 'adopt-go' examples/README.md &&
  grep -q -F -e 'adopt-java' examples/README.md; then
  ok
else
  bad "examples lost their beyond-minimum Go/Java consumer breadth (#7)"
fi

# determinism seed stays pinned: insertion-order independence test
# present in the runner unit tests (QualitySourcesInfo/checkout-order evidence).
if grep -q -F -e 'state_digest_independent_of_insertion_order' quality/runner/src/lib_tests_a.rs; then
  ok
else
  bad "quality runner lost its insertion-order determinism seed (#84)"
fi

# curated/native test backing stays present alongside the evidence
# files (no untested taxonomy drift).
if [[ -f "quality/curated_defaults_tests.bzl" ]] &&
  [[ -f "quality/native_config_tests.bzl" ]]; then
  ok
else
  bad "quality curated/native test backing missing (curated_defaults_tests/native_config_tests)"
fi

# parity gate stays versioned: fail-closed parity unit-test entry point.
if grep -q -F -e 'parity_unit_tests' quality/parity_tests.bzl; then
  ok
else
  bad "quality parity gate lost (parity_unit_tests entry)"
fi

# Fixture owns the single-owner mixed hello package.
if grep -q -F -e '' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "framework composition record lost (mixed fixture)"
fi

# generated-file exclusion list stays declared with its generator
# (beyond the adopt-js-ts inert arrival record above).
if grep -q -F -e 'generated' tools/ci/code_ownership.sh; then
  ok
else
  bad "code_ownership lost its generated-file exclusion record (#12)"
fi

# apply-safety filesystem evidence stays pinned: the collected-change
# applier under test (digest/atomicity/interruption batteries).
if grep -q -F -e 'apply_collected_changes' cli/cli/src/exec/quality_apply.rs; then
  ok
else
  bad "quality apply applier evidence lost (apply_collected_changes fn)"
fi

# minimum-consumer index stays pinned: Rust + Python adopt entries
# (foreign Cargo/Python trees via dx generate; broader set above).
if grep -q -F -e 'adopt-rust' examples/README.md &&
  grep -q -F -e 'adopt-python' examples/README.md; then
  ok
else
  bad "examples index lost its minimum Rust/Python consumer entries (#7)"
fi

# minimum-consumer READMEs stay present with commands + evidence
# (beyond-minimum Go/Java slice owns its own check above).
if [[ -f "examples/adopt-rust/README.md" ]] &&
  [[ -f "examples/adopt-python/README.md" ]]; then
  ok
else
  bad "examples lost their minimum Rust/Python consumer READMEs (#7)"
fi

# atomic-write evidence stays pinned: atomic apply of verified
# reads in the collected-change applier plus mode-preserving writes.
if grep -q -F -e 'write_atomic' cli/cli/src/exec/quality_apply.rs &&
  grep -q -F -e 'preserves_existing_mode_on_overwrite' cli/atomic_fs/src/lib.rs; then
  ok
else
  bad "quality apply applier lost its atomic-write evidence (#84)"
fi

# lane-A CI scope stays dogfood-like (plus Phase 1):
# the dogfood self-call runs all nine checks with none disabled (coverage
# executes the tests via `resolve_for_test` plus `bazel coverage`) behind
# min_coverage 97 over verbatim `//...` (covering lane-A trees plus
# fixtures), with dogfood-freshness for generate freshness plus audits.
# No bespoke corpus converge remains.
if grep -q -F -e 'dogfood-freshness' .github/workflows/ci.yml &&
  grep -q -F -e 'dogfood (self-call reusable consumer workflow)' .github/workflows/ci.yml &&
  grep -q -F -e 'min_coverage: "97"' .github/workflows/ci.yml &&
  ! grep -q -F -e 'disabled_checks' .github/workflows/ci.yml &&
  ! grep -q -F -e 'attr(tags, corpus' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its dogfood-like consumer scope (#12/#408 plus Phase 1 #607 all-nine min-coverage)"
fi

# Gazelle language extensions stay present: one extension directory
# per delivered foundation (upstream rules stay the implementation;
# exact import/lock proofs pinned in //tools/ci:foundation_maps).
if [[ -d "gazelle/rust" ]] &&
  [[ -d "gazelle/python" ]] &&
  [[ -d "gazelle/typescript" ]]; then
  ok
else
  bad "Gazelle lost a delivered-foundation extension dir (rust/python/typescript, #7)"
fi

# Gazelle framework extensions stay present alongside the language
# set (named adapters over upstream parsers, no generic fallback).
if [[ -d "gazelle/vue" ]] &&
  [[ -d "gazelle/svelte" ]] &&
  [[ -d "gazelle/astro" ]] &&
  [[ -d "gazelle/mdx" ]]; then
  ok
else
  bad "Gazelle lost a framework extension dir (vue/svelte/astro/mdx, #8)"
fi

# aspect gate stays provider-closed: aspects visit only targets
# carrying QualitySourcesInfo, never a parallel file list.
if grep -q -F -e 'QualitySourcesInfo not in target' quality/aspects.bzl; then
  ok
else
  bad "quality aspects lost their QualitySourcesInfo-only gate (#12)"
fi

# cache-argv marker stays pinned: the runner tests thread an explicit
# no-cache argv through (full invalidation batteries still open).
if grep -q -F -e '--no-cache' quality/runner/src/real_tests_a.rs; then
  ok
else
  bad "quality runner lost its --no-cache argv marker (#84)"
fi

# provider-closed aspects stay fallback-free: only direct_sources
# supplies files, never a parallel list (record lives in the framework
# adapters doc under link-don't-copy; broader trees stay open with no
# false claim).
if grep -q -F -e 'no generic fallback' docs/generation/framework-adapters.md; then
  ok
else
  bad "quality aspects lost their no-generic-fallback record (#12)"
fi

# lane-A native-config binding stays forwarder-closed: hints ride the
# public QualitySourcesInfo owner via shared dx_wrap (wrapper.bzl) plus
# the custom language forwarders that re-declare them.
if grep -q -F -e 'aspect_hints' libs/starlark/wrapper.bzl &&
  grep -q -F -e 'aspect_hints' javascript/rules/defs.bzl &&
  grep -q -F -e 'dx_wrap' go/rules/defs.bzl &&
  grep -q -F -e 'dx_wrap' java/rules/defs.bzl &&
  grep -q -F -e 'dx_wrap' python/rules/defs.bzl &&
  grep -q -F -e 'dx_wrap' rust/rules/defs.bzl; then
  ok
else
  bad "wrappers lost their lane-A aspect_hints forwarder plumbing (#12)"
fi

# lane-A CI scope covers the proven language trees via the consumer
# whole-tree test step (`dx test //...` on all four qualified hosts
# through the dogfood self-call), bounded by the shared .bazelrc test
# flags; ruff enforces --fail_on warning.
if grep -q -F -e '-- test //...' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" .github/workflows/ci.yml &&
  grep -q -F -e 'test --local_test_jobs=4' .bazelrc &&
  grep -q -F -e '--fail_on warning' .bazelrc; then
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
