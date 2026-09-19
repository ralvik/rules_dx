#!/usr/bin/env bash
# Environment/codegen qualification harness (issue #309).
#
# Qualifies the as-built environment/codegen record with fixture evidence
# and owned gaps, without claiming the unproven required tests:
# - public env contribution protocol stays deferred (PATH-tools-only
#   boundary, no third-party plugins; public PATH-tool API pinned),
# - Windows .envrc/junction fallback stays unsupported (manual PATH
#   guidance, symlink-only with failure before mutation, no junction or
#   copy fallback),
# - standalone-without-Bazel path stays Bazel-first (seed-host
#   //cli/cli:dx_standalone plus install-time publisher-identity
#   verification, no checksum-only fallback),
# - signing/trust selection stays implemented-verifier plus deferred
#   generation (Sigstore keyless bundle on the TUF trust root,
#   draft-only publisher ceiling; SBOM/provenance/BCR deferred),
# - required bootstrap/fidelity/spaces/stale/IDE/atomic/BEP/projection/
#   root-candidate/cold-warm tests stay open under #309 with delivered
#   fixtures (bootstrap_test spaces+noop+unmanaged, cli/env lock units,
#   cli/roots frozen baseline plus incrementality, codegen collector
#   frozen contracts, per-foundation env plans, node pnpm projection)
#   and no false green claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:env_codegen_qualification`,
# following //tools/ci:quality_adapters_parity.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

env_doc="docs/environments/environment.md"
managed="docs/environments/managed-state.md"
codegen_doc="docs/environments/codegen.md"
node_doc="docs/environments/node.md"
matrix="docs/testing/verification-matrix.md"

# Public contribution stays deferred with a PATH-tools-only boundary.
if grep -q -F -e 'PATH-tools-only' "$env_doc" &&
  grep -q -F -e 'EnvironmentInfo` stays PATH-tool-only' "$env_doc"; then
  ok
else
  bad "environment.md lost its PATH-tools-only plus EnvironmentInfo record"
fi

# Third-party language-integration plugins stay deferred past v1.
if grep -q -F -e 'third-party language-integration plugins are deferred' "$env_doc" &&
  grep -q -F -e 'not third-party environment plugins' "$env_doc"; then
  ok
else
  bad "environment.md lost its deferred third-party plugin record"
fi

# Public PATH-tool API stays pinned to the same validated constructors.
if grep -q -F -e 'environment_tool(name, executable, bin_name)' "$env_doc" &&
  grep -q -F -e 'EnvironmentInfo' env/defs.bzl &&
  grep -q -F -e 'environment_tool' env/defs.bzl &&
  [[ -f "env/defs_tests.bzl" ]] &&
  grep -q -F -e 'env_defs_unit_tests' env/defs_tests.bzl; then
  ok
else
  bad "public PATH-tool API lost its defs.bzl plus defs_tests pin"
fi

# Windows keeps manual PATH guidance with no junction or copy fallback.
if grep -q -F -e 'no junction or copy' "$env_doc" &&
  grep -q -F -e 'Windows keeps manual `PATH` guidance' "$env_doc" &&
  grep -q -F -e 'failure before mutation' "$env_doc"; then
  ok
else
  bad "environment.md lost its Windows manual-PATH plus no-fallback record"
fi

# Managed state stays symlink-only with no fallback projection mode.
if grep -q -F -e 'There is no launcher, junction, copy' "$managed" &&
  grep -q -F -e 'missing capability fails before mutation' "$managed"; then
  ok
else
  bad "managed-state.md lost its symlink-only plus no-fallback record"
fi

# CLI env implementation carries no junction/copy fallback.
if ! grep -rn -F -e 'junction' cli/env/src/ 2>/dev/null | grep -q . &&
  ! grep -rn -E -e 'copy.*fallback|fallback.*copy' cli/env/src/ 2>/dev/null | grep -q .; then
  ok
else
  bad "cli/env/src gained a junction or copy fallback"
fi

# Direnv snippet stays PATH-only with watch plus missing-dir guidance.
if grep -q -F -e 'PATH_add' "$env_doc" &&
  grep -q -F -e 'watch_file' "$env_doc" &&
  grep -q -F -e 'never invokes Bazel' "$env_doc" &&
  grep -q -F -e 'init_must_refuse' cli/adopt/src/scaffold.rs; then
  ok
else
  bad "direnv PATH-only plus watch plus absent-only scaffold record lost"
fi

# Bazel-first installation stays the supported path with standalone staged.
if grep -q -F -e 'bazel run //dx:env' "$env_doc" &&
  grep -q -F -e 'dx_standalone' cli/cli/BUILD.bazel &&
  grep -q -F -e 'until releases are cut' "$env_doc"; then
  ok
else
  bad "Bazel-first plus dx_standalone staged record lost"
fi

# Install-time verification refuses checksum-only inputs.
if grep -q -F -e 'checksum-only' deploy/install/dx_verify.sh &&
  grep -q -F -e '--sha256' deploy/install/dx_verify.sh &&
  [[ -f "deploy/install/dx_verify_test.sh" ]]; then
  ok
else
  bad "dx_verify lost its checksum-only refusal plus test"
fi

# Functional: dx_verify refuses checksum-only without network.
if bash deploy/install/dx_verify.sh --sha256 deadbeef 2>/tmp/dxv309.err; then
  bad "dx_verify accepted --sha256 (must refuse checksum-only)"
else
  if grep -q -F -e 'checksum-only' /tmp/dxv309.err; then
    ok
  else
    bad "dx_verify --sha256 refusal lost its checksum-only diagnostic"
  fi
fi

# Signing trust root stays Sigstore keyless on the TUF root.
if grep -q -F -e 'https://tuf-repo-cdn.sigstore.dev' deploy/install/dx_verify.sh &&
  grep -q -F -e 'cosign verify-blob' deploy/install/dx_verify.sh &&
  grep -q -F -e 'no checksum-only' deploy/install/dx_verify.sh; then
  ok
else
  bad "dx_verify lost its Sigstore TUF plus cosign trust record"
fi

# Draft-only publisher ceiling stays machine-checked.
if grep -q -F -e 'draft != True' deploy/rules/github.bzl &&
  grep -q -F -e 'draft = True' "$env_doc" &&
  grep -q -F -e 'v0.0.0-dryrun' "$env_doc" &&
  [[ -f ".github/workflows/publish-dry-run.yml" ]]; then
  ok
else
  bad "draft-only publisher ceiling lost (github.bzl gate, docs, dry-run)"
fi

# SBOM/provenance/BCR generation stays deferred.
if grep -q -F -e 'stay deferred as platforms qualify' "$env_doc" &&
  grep -q -F -e 'BCR submission stay deferred' "$env_doc"; then
  ok
else
  bad "environment.md lost its SBOM/provenance/BCR deferred record"
fi

# Bootstrap fixture evidence: spaces path, noop, unmanaged refusal, marker.
if grep -q -F -e 'work space' cli/env/bootstrap_test.sh &&
  grep -q -F -e 'already current' cli/env/bootstrap_test.sh &&
  grep -q -F -e 'unmanaged' cli/env/bootstrap_test.sh &&
  grep -q -F -e '.rules_dx_managed' cli/env/bootstrap_test.sh; then
  ok
else
  bad "bootstrap_test lost its spaces plus noop plus unmanaged plus marker proof"
fi

# Lock/concurrency evidence stays pinned in cli/env plus atomic_fs units.
if grep -q -F -e 'lock_exclusive' cli/atomic_fs/src/lib.rs &&
  grep -q -F -e 'file.try_lock()' cli/atomic_fs/src/lib.rs &&
  grep -q -F -e 'acquire_lock' cli/env/src/lib.rs &&
  grep -q -F -e 'ten seconds' "$managed" &&
  grep -q -F -e 'File::try_lock' "$managed"; then
  ok
else
  bad "cli/env lock evidence lost (try_lock plus ten-second deadline)"
fi

# Roots frozen baseline plus measured evidence stays pinned.
if grep -q -F -e 'FROZEN_STRATEGY' cli/roots/src/lib.rs &&
  grep -q -F -e 'RecursivePattern' cli/roots/src/lib.rs &&
  grep -q -F -e 'FROZEN_EVIDENCE' cli/roots/src/lib.rs &&
  grep -q -F -e 'INCREMENTALITY_EVIDENCE' cli/roots/src/lib.rs; then
  ok
else
  bad "cli/roots lost its frozen baseline plus incrementality evidence"
fi

# Codegen collector frozen contracts stay pinned on both sides.
if [[ -f "tools/ci/backlog_contracts.sh" ]] &&
  grep -q -F -e 'pub const OUTPUT_GROUP' cli/codegen/src/lib.rs &&
  grep -q -F -e 'pub const SHARD_SUFFIX' cli/codegen/src/lib.rs &&
  grep -q -F -e 'DX_CODEGEN_PLAN_OUTPUT_GROUP' generation/codegen.bzl; then
  ok
else
  bad "codegen collector frozen contracts lost (backlog_contracts plus consts)"
fi

# Functional: Rust and Starlark codegen constants still agree.
rust_group="$(grep -F -e 'pub const OUTPUT_GROUP' cli/codegen/src/lib.rs | sed 's/.*= "//; s/";.*//')"
bzl_group="$(grep -F -e 'DX_CODEGEN_PLAN_OUTPUT_GROUP = ' generation/codegen.bzl | head -n 1 | sed 's/.*= "//; s/".*//')"
rust_suffix="$(grep -F -e 'pub const SHARD_SUFFIX' cli/codegen/src/lib.rs | sed 's/.*= "//; s/";.*//')"
bzl_suffix="$(grep -F -e 'DX_CODEGEN_SHARD_SUFFIX = ' generation/codegen.bzl | head -n 1 | sed 's/.*= "//; s/".*//')"
if [[ -n "$rust_group" && "$rust_group" == "$bzl_group" && -n "$rust_suffix" && "$rust_suffix" == "$bzl_suffix" ]]; then
  ok
else
  bad "codegen Rust/Starlark constants drifted (group=$rust_group/$bzl_group suffix=$rust_suffix/$bzl_suffix)"
fi

# Per-foundation env plans stay present and pinned by foundation_maps.
env_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
  if [[ ! -f "$lang/env/plan.bzl" || ! -f "$lang/env/plan_tests.bzl" ]]; then
    env_missing="$env_missing $lang"
  fi
done
if [[ -z "$env_missing" ]] &&
  grep -q -F -e 'environment plans stay present' tools/ci/foundation_maps.sh; then
  ok
else
  bad "per-foundation env plans missing or unpinned:$env_missing"
fi

# Node projection stays pnpm-selected without a second resolution.
if grep -q -F -e 'never invokes `pnpm install`' "$node_doc" &&
  grep -q -F -e 'without' "$node_doc"; then
  ok
else
  bad "node.md lost its pnpm-selected without-second-resolution record"
fi

# CLI never implements a second remote-cache downloader.
if grep -q -F -e 'never' "$codegen_doc" &&
  grep -q -F -e 'second remote-cache downloader' "$codegen_doc"; then
  ok
else
  bad "codegen.md lost its no-second-downloader record"
fi

# Owned gaps stay listed under #309 with no premature COMPLETED.
if grep -q -F -e 'under issue #309' "$env_doc" &&
  grep -q -F -e 'public env contribution protocol' "$env_doc" &&
  grep -q -F -e 'cold/warm benchmark' "$env_doc"; then
  ok
else
  bad "environment.md lost its owned-gap list under #309"
fi

# Verification matrix keeps Env/codegen Open with no Supported claim.
if grep -q -F -e '| Open | Open |' "$matrix" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q .; then
  ok
else
  bad "verification-matrix lost its Env/codegen Open plus no-Supported gate"
fi

dx_test_summary "env/codegen qualification harness"
