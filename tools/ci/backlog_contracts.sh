#!/usr/bin/env bash
# Backlog-contracts harness (issues #9, #10): machine-checks the frozen
# cross-file contracts plus the honest gap labels of the environment /
# codegen and docs-pipeline backlog tracks.
#
# The codegen collector (`cli/codegen`) mirrors the frozen Starlark
# helpers in `//generation:codegen.bzl`: the output-group name, the
# reserved shard suffix, the collecting aspect, and the canonical
# repository selection must stay identical on both sides, or collection
# silently stops matching. The commit-lock protocol is specified in
# `docs/environments/managed-state.md` with cross-platform correctness
# explicitly disclaimed, and the root-selection candidates stay
# unverified WP4 slices in `docs/environments/codegen.md`. The
# `dx docs` removal plus #10-tracked reintroduction is recorded in
# `docs/cli/commands/docs.md`, the IR identity (`dx.documentation.v1`,
# `schema_major: 1`) is stable across the proto and its contract, and
# the support matrix still promotes no cell to `Supported`.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (11 checks). Bare-schema reverse-dependent projection
# queries, lock platform evidence, `dx clean` reclaimable-bytes
# reporting beyond the implemented measure/render pair, per-language
# docs adapter runs, link/reference completeness, renderer execution,
# guide-step CI wiring, and the timed quickstart proof all stay open
# per #9/#10 and are recorded as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:backlog_contracts`,
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

rust_const() {
  grep -F -e "pub const $1: &str = " "cli/codegen/src/lib.rs" | sed 's/.*= "//; s/";.*//'
}
bzl_const() {
  grep -F -e "$1 = " "generation/codegen.bzl" | head -n 1 | sed 's/.*= "//; s/".*//'
}

# The collector output group matches the Starlark helper on both
# sides, or shard recognition silently stops matching.
if [[ "$(rust_const OUTPUT_GROUP)" == "$(bzl_const DX_CODEGEN_PLAN_OUTPUT_GROUP)" && -n "$(rust_const OUTPUT_GROUP)" ]]; then
  ok
else
  bad "codegen OUTPUT_GROUP drifted from DX_CODEGEN_PLAN_OUTPUT_GROUP"
fi

# The reserved shard suffix matches on both sides.
if [[ "$(rust_const SHARD_SUFFIX)" == "$(bzl_const DX_CODEGEN_SHARD_SUFFIX)" && -n "$(rust_const SHARD_SUFFIX)" ]]; then
  ok
else
  bad "codegen SHARD_SUFFIX drifted from DX_CODEGEN_SHARD_SUFFIX"
fi

# The collecting aspect named in Rust is defined in Starlark.
aspect="$(rust_const CODEGEN_ASPECT)"
if grep -q -F -e 'dx_codegen_plan_aspect = aspect(' generation/codegen.bzl && [[ "$aspect" == *"%dx_codegen_plan_aspect" ]]; then
  ok
else
  bad "codegen CODEGEN_ASPECT names an aspect not defined in generation/codegen.bzl"
fi

# The canonical repository selection exists as a target.
if [[ "$(rust_const REPOSITORY_TARGET)" == "//dx:codegen" ]] && grep -q -F -e 'name = "codegen"' dx/BUILD.bazel; then
  ok
else
  bad "codegen REPOSITORY_TARGET //dx:codegen is missing or renamed"
fi

# The commit-lock contract keeps its cross-platform disclaimer:
# locking is implemented plus unit-tested, never proven portable.
if grep -q -F -e 'does not establish cross-platform correctness' docs/environments/managed-state.md; then
  ok
else
  bad "docs/environments/managed-state.md lost the cross-platform disclaimer"
fi

# Root-selection candidates stay unverified WP4 slices, never silently
# promoted to accepted architecture.
if grep -q -F -e 'remain unverified claims tracked as later WP4 slices' docs/environments/codegen.md; then
  ok
else
  bad "docs/environments/codegen.md lost the WP4-unverified root-candidate note"
fi

# ADR 0018 records the implemented clean evidence (process scan plus
# measured bytes) instead of the stale pending line.
if grep -q -F -e 'cli/clean/src/live.rs' docs/decisions/0018-umbrella-check-fix-cleanup-clean.md && grep -q -F -e 'cli/clean/src/bytes.rs' docs/decisions/0018-umbrella-check-fix-cleanup-clean.md; then
  ok
else
  bad "ADR 0018 lost the implemented clean-evidence status update"
fi

# `dx docs` stays removed with reintroduction tracked in #10: the
# name must not return as a placeholder.
if grep -q -F -e 'issue #31' docs/cli/commands/docs.md && grep -q -F -e 'issues/10' docs/cli/commands/docs.md; then
  ok
else
  bad "docs/cli/commands/docs.md lost the removal + #10-tracking record"
fi

# The check-mode/render/link-completeness gaps stay linked to #10 in
# both pipeline contracts.
if grep -q -F -e 'issues/10' docs/documentation/site.md && grep -q -F -e 'issues/10' docs/documentation/doc-ir.md; then
  ok
else
  bad "docs pipeline contracts lost the #10 gap links"
fi

# The IR identity is stable across the proto and its contract.
if grep -q -F -e 'package dx.documentation.v1;' docs/ir/doc_ir.proto && grep -q -F -e 'dx.documentation.v1' docs/documentation/doc-ir.md && grep -q -F -e 'schema_major: 1' docs/documentation/doc-ir.md; then
  ok
else
  bad "docs IR identity drifted between docs/ir/doc_ir.proto and doc-ir.md"
fi

# No silent promotion: the support matrix still carries no Supported
# cell, so docs-pipeline (or any other) support cannot be implied.
if grep -q -F -e 'no cell is currently `Supported`' docs/product/support-matrix.md; then
  ok
else
  bad "docs/product/support-matrix.md lost the no-Supported-cell record"
fi

echo "backlog contracts audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
