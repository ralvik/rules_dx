#!/usr/bin/env bash
# Backlog-contracts harness (env/codegen + docs-pipeline): machine-checks
# the frozen cross-file contracts.
#
# The codegen collector (`cli/codegen`) mirrors the frozen Starlark
# helpers in `//generation:codegen.bzl`: the output-group name, the
# reserved shard suffix, the collecting aspect, and the canonical
# repository selection must stay identical on both sides, or collection
# silently stops matching. The IR identity (`dx.documentation.v1`)
# is stable for the proto.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (5 checks): output-group, shard-suffix, aspect, repository
# selection, and proto IR identity. Functional checks only (no
# docs-prose guards).
#
# Versioned here, run by CI via `bazel run //tools/ci:backlog_contracts`,
# following //tools/ci:registry_singularity.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

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

# The IR identity is stable for the proto.
if grep -q -F -e 'package dx.documentation.v1;' docs/ir/doc_ir.proto; then
  ok
else
  bad "docs/ir/doc_ir.proto lost the dx.documentation.v1 identity"
fi

dx_test_summary "backlog contracts audit"
