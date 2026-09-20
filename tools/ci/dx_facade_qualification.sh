#!/usr/bin/env bash
# Dx facade qualification harness.
#
# Machine-checks the placeholder-vs-real state for the `//dx:config` and
# `//dx:codegen` facade labels so the tree cannot silently drift: both stay
# empty filegroups (Accepted selection identity/default, not provisional),
# while the real typed sections and plan collection live in their frozen
# owners (`//quality:policy.bzl` for `//quality:sources.bzl` for
# `//generation:codegen.bzl` plus `//cli/codegen` for, `//cli/roots`
# for the `//...` baseline). Docs assert the same Accepted state.
#
# Versioned here, run by CI via `bazel run //tools/ci:dx_facade_qualification`,
# following //tools/ci:backlog_contracts.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Both facade labels stay empty filegroups with no srcs: adding sources
# would build a central eager dependency (codegen) or silently disable
# every capability (config with empty families).
if grep -E -e 'name = "codegen"' -A2 dx/BUILD.bazel | grep -q -F -e 'srcs = [],' &&
  grep -E -e 'name = "config"' -A2 dx/BUILD.bazel | grep -q -F -e 'srcs = [],'; then
  ok
else
  bad "//dx:codegen and //dx:config must stay empty filegroups with srcs = []"
fi

# The facade docstring records the Accepted hygiene decision, not a
# provisional placeholder awaiting typed sections.
if grep -q -F -e 'Accepted' dx/BUILD.bazel &&
  grep -q -F -e '//tools/ci:dx_facade_qualification' dx/BUILD.bazel; then
  ok
else
  bad "dx/BUILD.bazel lost its Accepted facade record"
fi

# No PROVISIONAL open-decision marker may remain on the facade labels.
if ! grep -E -e 'PROVISIONAL \(open decision' dx/BUILD.bazel | grep -q -E -e '|issue #506|issue #506'; then
  ok
else
  bad "dx/BUILD.bazel still marks //dx:config or //dx:codegen PROVISIONAL"
fi

# The workspace label flag keeps its resolvable facade default.
if grep -q -F -e 'build_setting_default = "//dx:config"' config/BUILD.bazel; then
  ok
else
  bad "config/BUILD.bazel lost its //dx:config workspace default"
fi

# The canonical repository selection still resolves to the facade label.
if grep -q -F -e 'pub const REPOSITORY_TARGET: &str = "//dx:codegen";' cli/codegen/src/lib.rs &&
  grep -q -F -e 'pub const CODEGEN_REPOSITORY_TARGET: &str = "//dx:codegen";' cli/setup/src/lib.rs; then
  ok
else
  bad "CLI REPOSITORY_TARGET drifted from //dx:codegen"
fi

# Rust and Starlark codegen constants still agree (frozen).
rust_group="$(grep -F -e 'pub const OUTPUT_GROUP' cli/codegen/src/lib.rs | sed 's/.*= "//; s/";.*//')"
bzl_group="$(grep -F -e 'DX_CODEGEN_PLAN_OUTPUT_GROUP = ' generation/codegen.bzl | head -n 1 | sed 's/.*= "//; s/".*//')"
rust_suffix="$(grep -F -e 'pub const SHARD_SUFFIX' cli/codegen/src/lib.rs | sed 's/.*= "//; s/";.*//')"
bzl_suffix="$(grep -F -e 'DX_CODEGEN_SHARD_SUFFIX = ' generation/codegen.bzl | head -n 1 | sed 's/.*= "//; s/".*//')"
if [[ -n "$rust_group" && "$rust_group" == "$bzl_group" && -n "$rust_suffix" && "$rust_suffix" == "$bzl_suffix" ]]; then
  ok
else
  bad "codegen Rust/Starlark constants drifted (group=$rust_group/$bzl_group suffix=$rust_suffix/$bzl_suffix)"
fi

# Typed policy sections stay frozen in their owner: fail closed if
# the constructor moves without this harness moving with it.
if grep -q -F -e 'quality_family = rule(' quality/policy.bzl &&
  grep -q -F -e 'workspace_policy = rule(' quality/policy.bzl &&
  grep -q -F -e 'QualityPolicyInfo' quality/policy.bzl; then
  ok
else
  bad "quality/policy.bzl lost its frozen typed sections"
fi

# Source-ownership boundary stays frozen in its owner.
if grep -q -F -e 'QualitySourcesInfo = provider(' quality/sources.bzl &&
  grep -q -F -e 'KNOWN_SEMANTIC_FILE_CLASSES = [' quality/sources.bzl; then
  ok
else
  bad "quality/sources.bzl lost its frozen ownership boundary"
fi

# Admitted generator/language pairs stay frozen with at least the first
# pair: an empty registry would silently admit nothing.
if grep -q -F -e 'DX_CODEGEN_ADMITTED_PAIRS = (' generation/codegen.bzl &&
  grep -q -F -e '("protobuf", "rust")' generation/codegen.bzl; then
  ok
else
  bad "generation/codegen.bzl lost its issue #506 admitted-pair freeze"
fi

# effective roots stay on the frozen baseline.
if grep -q -F -e 'pub const FROZEN_STRATEGY' cli/roots/src/lib.rs &&
  grep -q -F -e 'RecursivePattern' cli/roots/src/lib.rs; then
  ok
else
  bad "cli/roots lost its frozen issue #506 baseline"
fi

# Architecture facade twins read Accepted with the owning guard.
if grep -q -F -e 'Accepted (issue #423' docs/architecture/README.md &&
  grep -q -F -e '//tools/ci:dx_facade_qualification' docs/architecture/README.md &&
  ! grep -E -e '`//dx:(codegen|config)`.*Provisional' docs/architecture/README.md | grep -q .; then
  ok
else
  bad "docs/architecture/README.md facade rows must read Accepted (issue #423)"
fi

# ADR 0011 keeps the / ownership record with the resolution.
if grep -q -F -e 'open decisions' docs/decisions/0011-configuration-composition.md &&
  grep -q -F -e 'issue #423' docs/decisions/0011-configuration-composition.md; then
  ok
else
  bad "docs/decisions/0011 lost its / plus issue #423 record"
fi

# Generation contracts reference the frozen behind the facade identity.
if grep -q -F -e 'issue #506' docs/environments/codegen.md &&
  grep -q -F -e '//dx:codegen' docs/environments/codegen.md; then
  ok
else
  bad "docs/environments/codegen.md lost its issue #506 plus //dx:codegen record"
fi

dx_test_summary "dx facade qualification harness"
