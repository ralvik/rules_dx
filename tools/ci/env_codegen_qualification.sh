#!/usr/bin/env bash
# Environment/codegen qualification harness.
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
# root-candidate tests stay open under with delivered
#   fixtures (bootstrap_test spaces+noop+unmanaged, cli/env lock units,
#   cli/roots frozen baseline plus reference, codegen collector
#   frozen contracts, per-foundation env plans, node pnpm projection)
#   and no false green claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:env_codegen_qualification`,
# following //tools/ci:quality_adapters_parity.
# Seed-host delivery qualified under closed #506; platform plus consumer plus
# release evidence promoting the layer beyond seed-host-delivered qualified
# under issue #787 (per-required-host plus adopt-consumer plus release
# checklist linkage, still seed-executed with static per-host pins).
# Admitted-pairs evolution onboarding qualified under issue #788 (checklist
# plus per-pair fixtures plus qualification coverage for each admitted pair).
# Bare-schema expansion (#751), collision replacement contract (#752), and
# concurrency/NFS/relock (#753) stay out of scope and open.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

env_doc="docs/environments/environment.md"
managed="docs/environments/managed-state.md"
codegen_doc="docs/environments/codegen.md"
node_doc="docs/environments/node.md"
matrix="docs/testing/verification-matrix.md"
pins="env/tests/fixtures/env_codegen/pins.bzl"
pins_build="env/tests/fixtures/env_codegen/BUILD.bazel"
expected="env/tests/fixtures/env_codegen/env_codegen.expected"
roots_bep="env/tests/fixtures/env_codegen/roots_bep.txt"

# Public contribution stays deferred with a PATH-tools-only boundary.
if grep -q -F -e 'PATH-tools-only' "$env_doc" &&
  grep -q -F -e 'EnvironmentInfo` stays PATH-tool-only' "$env_doc"; then
  ok
else
  bad "environment.md lost its PATH-tools-only plus EnvironmentInfo record"
fi

# Third-party language-integration plugins stay deferred past v1.
if grep -q -F -e 'third-party language-integration plugins' "$env_doc" &&
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
if grep -q -F -e 'checksum-only' deploy/install/src/lib.rs &&
  grep -q -F -e '--sha256' deploy/install/src/lib.rs &&
  grep -q -F -e 'dx_install_tools_test' deploy/install/BUILD.bazel; then
  ok
else
  bad "dx_verify lost its checksum-only refusal plus test"
fi

# Functional: dx_verify refuses checksum-only without network.
if bazel run //deploy/install:dx_verify -- --sha256 deadbeef 2>/tmp/dxv309.err; then
  bad "dx_verify accepted --sha256 (must refuse checksum-only)"
else
  if grep -q -F -e 'checksum-only' /tmp/dxv309.err; then
    ok
  else
    bad "dx_verify --sha256 refusal lost its checksum-only diagnostic"
  fi
fi

# Signing trust root stays Sigstore keyless on the TUF root.
if grep -q -F -e 'https://tuf-repo-cdn.sigstore.dev' deploy/install/src/lib.rs &&
  grep -q -F -e 'cosign verify-blob' deploy/install/src/lib.rs &&
  grep -q -F -e 'no checksum-only' deploy/install/src/lib.rs; then
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

# SBOM/provenance/BCR generation stays owner-gated.
if grep -q -F -e 'SBOM/provenance in `deploy/release/sbom.bzl`' "$env_doc" &&
  grep -q -F -e 'BCR submission are implemented' "$env_doc"; then
  ok
else
  bad "environment.md lost its SBOM/provenance/BCR owner-gated record"
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

# Roots frozen baseline plus reference evidence stays pinned.
if grep -q -F -e 'FROZEN_STRATEGY' cli/roots/src/lib.rs &&
  grep -q -F -e 'RecursivePattern' cli/roots/src/lib.rs &&
  grep -q -F -e 'FROZEN_EVIDENCE' cli/roots/src/lib.rs &&
  grep -q -F -e 'INCREMENTALITY_EVIDENCE' cli/roots/src/lib.rs; then
  ok
else
  bad "cli/roots lost its frozen baseline plus reference evidence"
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

# Owned gaps stay listed under with no premature COMPLETED.
if grep -q -F -e 'successors to closed #506' "$env_doc" &&
  grep -q -F -e 'admitted-pairs evolution' "$env_doc" &&
  grep -q -F -e 'root-candidate tests' "$env_doc"; then
  ok
else
  bad "environment.md lost its owned-gap list under #506"
fi

# Verification matrix keeps Env/codegen Open with no Supported claim.
if grep -q -F -e '| Open | Open |' "$matrix" &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' "$matrix" | grep -q .; then
  ok
else
  bad "verification-matrix lost its Env/codegen Open plus no-Supported gate"
fi

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" && -f "$roots_bep" ]]; then
  ok
else
  bad "env codegen fixture missing (want $pins plus $pins_build plus env_codegen.expected plus roots_bep.txt)"
fi

# Pins record protocol plus Windows fallback plus standalone plus signing/trust.
if grep -q -F -e 'PROTOCOL_BOUNDARY = "PATH-tools-only"' "$pins" &&
  grep -q -F -e 'WINDOWS_NO_FALLBACK = "no junction or copy fallback"' "$pins" &&
  grep -q -F -e 'STANDALONE_TARGET = "dx_standalone"' "$pins" &&
  grep -q -F -e 'SIGNING_TUF_ROOT = "https://tuf-repo-cdn.sigstore.dev"' "$pins" &&
  grep -q -F -e 'SIGNING_COSIGN = "cosign verify-blob"' "$pins" &&
  grep -q -F -e 'SIGNING_DRYRUN_TAG = "v0.0.0-dryrun"' "$pins"; then
  ok
else
  bad "pins.bzl lost its protocol plus Windows plus standalone plus signing pins under issue #506"
fi

# Pins record bootstrap plus fidelity plus spaces plus stale plus atomic-commit.
if grep -q -F -e 'BOOTSTRAP_SPACES = "work space"' "$pins" &&
  grep -q -F -e 'BOOTSTRAP_NOOP = "already current"' "$pins" &&
  grep -q -F -e 'BOOTSTRAP_MARKER = ".rules_dx_managed"' "$pins" &&
  grep -q -F -e 'Executable links preserve arguments' "$pins" &&
  grep -q -F -e 'STALE_BAZEL_CLEAN = "bazel clean"' "$pins" &&
  grep -q -F -e 'LOCK_EXCLUSIVE = "lock_exclusive"' "$pins" &&
  grep -q -F -e 'LOCK_ATOMIC_POINTER' "$pins"; then
  ok
else
  bad "pins.bzl lost its bootstrap plus fidelity plus stale plus atomic pins under issue #506"
fi

# Pins record IDE plus BEP plus projection.
if grep -q -F -e 'IDE_SEPARATE_BASE = "Use a separate IDE output base"' "$pins" &&
  grep -q -F -e 'IDE_RUST_FLYCHECK = "flycheck"' "$pins" &&
  grep -q -F -e 'IDE_GO_DRIVER = "GOPACKAGESDRIVER"' "$pins" &&
  grep -q -F -e 'BEP_ONE_STREAM = "one BEP stream"' "$pins" &&
  grep -q -F -e 'BEP_ENV_GROUP = "dx_env_plans"' "$pins" &&
  grep -q -F -e 'BEP_CODEGEN_GROUP = "dx_codegen_plans"' "$pins" &&
  grep -q -F -e 'PROJECTION_SYMLINK_ONLY = "symlink-only"' "$pins" &&
  grep -q -F -e 'PROJECTION_NO_PNPM_INSTALL' "$pins"; then
  ok
else
  bad "pins.bzl lost its IDE plus BEP plus projection pins under issue #506"
fi

# Pins record roots plus cold-warm plus WP slices plus rejected substitutes.
if grep -q -F -e 'ROOTS_FROZEN_STRATEGY = "FROZEN_STRATEGY"' "$pins" &&
  grep -q -F -e 'ROOTS_BASELINE = "//..."' "$pins" &&
  grep -q -F -e 'COLD_WARM_SCORE = "cold_ms + WARM_WEIGHT"' "$pins" &&
  grep -q -F -e 'COLD_WARM_NO_TIMING_CLAIM' "$pins" &&
  grep -q -F -e 'WP1_ADMITTED_PAIRS = ("protobuf", "rust")' "$pins" &&
  grep -q -F -e '"checksum-only fallback"' "$pins" &&
  grep -q -F -e 'OWNED_GAPS_NOTE' "$pins" &&
  grep -q -F -e 'NO_SUPPORTED_CLAIM' "$pins"; then
  ok
else
  bad "pins.bzl lost its roots plus cold-warm plus WP plus rejected pins under issue #506"
fi

# Fixture expected texts cover gaps plus roots/BEP.
if grep -q -F -e 'Public protocol stays deferred PATH-tools-only' "$expected" &&
  grep -q -F -e 'Windows fallback stays unsupported' "$expected" &&
  grep -q -F -e 'Standalone stays Bazel-first' "$expected" &&
  grep -q -F -e 'Signing plus trust stays implemented-verifier' "$expected" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned' "$expected" &&
  grep -q -F -e 'FROZEN_STRATEGY is RecursivePattern' "$roots_bep" &&
  grep -q -F -e 'one BEP stream' "$roots_bep" &&
  grep -q -F -e 'cold_ms + WARM_WEIGHT' "$roots_bep"; then
  ok
else
  bad "env_codegen.expected plus roots_bep.txt lost gap coverage (want protocol plus Windows plus standalone plus signing plus roots/BEP, issue #506)"
fi

# Docs own the qualified seed-only record with the fixture proof.
if grep -q -F -e 'env/tests/fixtures/env_codegen/pins.bzl' "$env_doc" &&
  grep -q -F -e 'qualified seed-only under closed #506' "$env_doc" &&
  grep -q -F -e 'env_codegen_qualification' "$env_doc" &&
  grep -q -F -e 'env/tests/fixtures/env_codegen/pins.bzl' "$codegen_doc" &&
  grep -q -F -e 'qualified seed-only under closed #506' "$codegen_doc"; then
  ok
else
  bad "environment.md or codegen.md lost its qualified seed-only plus pins fixture record under issue #506"
fi

# Live proof: the fixture plus the WP shard and roots fixtures build green.
if bazel build //env/tests/fixtures/env_codegen/... //env:env_shard_alpha //generation/codegen_shard:codegen_shard //cli/roots:roots_pattern_fixture --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "env codegen fixture plus WP shard plus roots fixture build failed (want green on the seed host, issue #506)"
fi

# Pins record platform plus consumer evidence (issue #787).
if grep -q -F -e 'PLATFORM_HOSTS = ("linux_x86_64", "linux_arm64"' "$pins" &&
  grep -q -F -e 'PLATFORM_SYMLINK_ONLY = "symlink-only on every required host"' "$pins" &&
  grep -q -F -e 'PLATFORM_REFUSAL = "unsupported_platform"' "$pins" &&
  grep -q -F -e 'PLATFORM_WINDOWS_CAPABILITY = "Developer Mode or grant SeBackupPrivilege"' "$pins" &&
  grep -q -F -e 'PLATFORM_NO_FALLBACK = "no junction or copy fallback"' "$pins" &&
  grep -q -F -e 'CONSUMER_ADOPT_RUST = "examples/adopt-rust"' "$pins" &&
  grep -q -F -e 'CONSUMER_WORKFLOW = ".github/workflows/reusable-consumer.yml"' "$pins" &&
  grep -q -F -e 'CONSUMER_TOOL_API = "environment_tool(name, executable, bin_name)"' "$pins"; then
  ok
else
  bad "pins.bzl lost its platform plus consumer pins under issue #787"
fi

# Pins record release plus out-of-scope evidence (issue #787).
if grep -q -F -e 'RELEASE_SBOM_DEMO = "//deploy/release:sbom_demo"' "$pins" &&
  grep -q -F -e 'RELEASE_SIGNING_DEMO = "//deploy/release:signing_demo"' "$pins" &&
  grep -q -F -e 'RELEASE_BCR_DEMO = "//deploy/release:bcr_demo"' "$pins" &&
  grep -q -F -e 'RELEASE_DRIVER = "//deploy/release:release_driver"' "$pins" &&
  grep -q -F -e 'RELEASE_GATE = "bazel run //tools/ci:supported_evidence_gate"' "$pins" &&
  grep -q -F -e 'OUT_OF_SCOPE_BARE_SCHEMA' "$pins" &&
  grep -q -F -e 'OUT_OF_SCOPE_COLLISION' "$pins" &&
  grep -q -F -e 'OUT_OF_SCOPE_CONCURRENCY' "$pins"; then
  ok
else
  bad "pins.bzl lost its release plus out-of-scope pins under issue #787"
fi

# Platform hosts: symlink-only on every required host with clean refusal.
if grep -q -F -e 'on every host' "$managed" &&
  grep -q -F -e 'unsupported_platform' cli/cli/src/platform.rs &&
  grep -q -F -e 'qualified seed-linux_x86_64' tools/coverage/cells.txt &&
  grep -q -F -e 'qualified windows_x86_64' tools/coverage/cells.txt; then
  ok
else
  bad "platform hosts lost symlink-only plus refusal plus coverage-cell pins (issue #787)"
fi

# Windows host specifics stay pinned in code plus managed state.
if grep -q -F -e 'Developer Mode or grant SeBackupPrivilege' cli/env/src/lib.rs &&
  grep -q -F -e '".exe"' cli/env/src/lib.rs &&
  grep -q -F -e 'to_lowercase' cli/env/src/lib.rs &&
  grep -q -F -e 'There is no launcher, junction, copy' "$managed" &&
  grep -q -F -e 'shell: bash' .github/workflows/ci.yml; then
  ok
else
  bad "Windows host lost its capability plus suffix plus casefold plus shell pins (issue #787)"
fi

# Per-host CI plus coverage cells with no union.
if grep -q -F -e 'build-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'build-musl-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'build-macos-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'build-windows-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'no cross-cell union' docs/testing/strategy-details.md; then
  ok
else
  bad "per-host CI plus coverage no-union linkage lost (issue #787)"
fi

# Platform floors plus routes plus skip budget stay linked.
if [[ -f "tools/ci/deployment_floors_qualification.sh" ]] &&
  [[ -f "tools/ci/cross_routes_qualification.sh" ]] &&
  [[ -f "tools/ci/skip_budget_qualification.sh" ]] &&
  [[ -f "tools/ci/coverage_qualification.sh" ]] &&
  grep -q -F -e 'deployment_floors_qualification' docs/product/promotion-checklist.md &&
  grep -q -F -e 'cross_routes_qualification' docs/product/promotion-checklist.md; then
  ok
else
  bad "platform floors plus routes plus skip plus coverage linkage lost (issue #787)"
fi

# Provisional backends stay provisional, never a qualified claim.
if grep -q -F -e 'provisional-backend exception' docs/product/support-matrix.md &&
  grep -q -F -e 'backends stay provisional' "$expected" &&
  grep -q -F -e 'provisional' cli/cli/src/platform.rs; then
  ok
else
  bad "provisional backends lost their never-qualified record (issue #787)"
fi

# Consumer proof: adopt workspaces plus reusable workflow.
if [[ -d "examples/adopt-rust" ]] &&
  [[ -d "examples/adopt-python" ]] &&
  [[ -d "examples/adopt-js-ts" ]] &&
  [[ -f ".github/workflows/reusable-consumer.yml" ]] &&
  grep -q -F -e 'adopt-rust' "$matrix"; then
  ok
else
  bad "adopt plus reusable-consumer proof lost (issue #787)"
fi

# Consumer workflow plus caller stay pinned with no implicit default.
if grep -q -F -e 'no implicit default' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'rules_dx_version: "0.0.0"' examples/consumer-ci/caller.yml &&
  grep -q -F -e 'environment plans stay present' tools/ci/foundation_maps.sh &&
  grep -q -F -e 'environment_tool' env/defs.bzl; then
  ok
else
  bad "consumer workflow plus caller plus tool-parity linkage lost (issue #787)"
fi

# Release artifacts stay implemented owner-gated with CI upload.
if grep -q -F -e 'name = "sbom_demo"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "signing_demo"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "bcr_demo"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'name = "release_driver"' deploy/release/BUILD.bazel &&
  grep -q -F -e 'sbom-provenance' .github/workflows/ci.yml; then
  ok
else
  bad "release sbom plus signing plus bcr plus driver linkage lost (issue #787)"
fi

# Release hygiene plus gate stay linked with no Supported claim.
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel &&
  [[ -z "$(git tag --list 'v*' || true)" ]] &&
  grep -q -F -e 'bazel run //tools/ci:supported_evidence_gate' docs/product/promotion-checklist.md &&
  [[ -f "tools/ci/supported_evidence_gate.sh" ]] &&
  ! grep -E -e '^\|.*\| *`?Supported`? *\|' docs/product/support-matrix.md | grep -q .; then
  ok
else
  bad "release hygiene plus supported-gate linkage lost (issue #787)"
fi

# Out-of-scope slices stay open under #751 plus #752 plus #753.
if grep -q -F -e '#751' "$codegen_doc" &&
  grep -q -F -e '#752' "$codegen_doc" &&
  grep -q -F -e '#753' "$codegen_doc" &&
  grep -q -F -e 'Out of scope for #787' "$expected" &&
  grep -q -F -e 'OUT_OF_SCOPE_BARE_SCHEMA' "$pins"; then
  ok
else
  bad "out-of-scope #751 plus #752 plus #753 record lost (issue #787)"
fi

# Admitted-pairs evolution onboarding doc owns the checklist (issue #788).
if grep -q -F -e 'Admitted-Pairs Evolution and New Generator Onboarding' "$codegen_doc" &&
  grep -q -F -e 'admitted-pairs evolution checklist' "$codegen_doc" &&
  grep -q -F -e 'DX_CODEGEN_ADMITTED_PAIRS' "$codegen_doc" &&
  grep -q -F -e 'Per-pair fixtures' "$codegen_doc"; then
  ok
else
  bad "codegen.md lost its admitted-pairs evolution onboarding checklist (issue #788)"
fi

# Onboarding covers the five evidence slices beyond the frozen set (issue #788).
if grep -q -F -e 'narrow ruleset-specific adapter' "$codegen_doc" &&
  grep -q -F -e 'dx_codegen_plans' "$codegen_doc" &&
  grep -q -F -e '.dxcodegen.pb' "$codegen_doc" &&
  grep -q -F -e 'symlink-only read-only mirror' "$codegen_doc" &&
  grep -q -F -e 'FROZEN_STRATEGY' "$codegen_doc" &&
  grep -q -F -e 'cold_ms + WARM_WEIGHT' "$codegen_doc"; then
  ok
else
  bad "codegen.md lost its onboarding provider plus BEP plus projection plus roots plus cold-warm slices (issue #788)"
fi

# Pins record the onboarding registry plus checklist linkage (issue #788).
if grep -q -F -e 'ONBOARDING_CHECKLIST = "admitted-pairs evolution checklist"' "$pins" &&
  grep -q -F -e 'ONBOARDING_PROVIDER = "narrow ruleset-specific adapter"' "$pins" &&
  grep -q -F -e 'ONBOARDING_BEP_GROUP = "dx_codegen_plans"' "$pins" &&
  grep -q -F -e 'ONBOARDING_SHARD_SUFFIX = ".dxcodegen.pb"' "$pins" &&
  grep -q -F -e 'ONBOARDING_PROJECTION = "symlink-only read-only mirror"' "$pins" &&
  grep -q -F -e 'ONBOARDING_ROOTS = "FROZEN_STRATEGY"' "$pins" &&
  grep -q -F -e 'ONBOARDING_COLD_WARM = "cold_ms + WARM_WEIGHT"' "$pins" &&
  grep -q -F -e 'ONBOARDING_QUALIFICATION = "bazel run //tools/ci:env_codegen_qualification"' "$pins"; then
  ok
else
  bad "pins.bzl lost its onboarding checklist plus slice pins under issue #788"
fi

# Pins record the per-pair fixture labels for each admitted pair (issue #788).
if grep -q -F -e 'ONBOARDING_PAIR_PROTOBUF_RUST = "protobuf/rust via //generation:codegen_prost_fixture"' "$pins" &&
  grep -q -F -e 'ONBOARDING_CHAIN_FIXTURES' "$pins" &&
  grep -q -F -e '//generation:codegen_shard_alpha' "$pins" &&
  grep -q -F -e '//generation:codegen_shard_beta' "$pins" &&
  grep -q -F -e '//generation:codegen_plan_chain_subject' "$pins" &&
  grep -q -F -e '//generation:codegen_plan_prost_subject' "$pins"; then
  ok
else
  bad "pins.bzl lost its per-pair fixture pins for protobuf/rust under issue #788"
fi

# Fixture expected texts cover onboarding plus per-pair fixtures (issue #788).
if grep -q -F -e 'Admitted-pairs evolution onboarding (issue #788)' "$expected" &&
  grep -q -F -e 'admitted-pairs evolution checklist' "$expected" &&
  grep -q -F -e '//generation:codegen_prost_fixture' "$expected" &&
  grep -q -F -e '//generation:codegen_plan_prost_subject' "$expected" &&
  grep -q -F -e 'Admitted-pairs evolution onboarding (issue #788)' "$roots_bep" &&
  grep -q -F -e '//generation:codegen_shard_alpha' "$roots_bep"; then
  ok
else
  bad "env_codegen.expected plus roots_bep.txt lost onboarding plus per-pair coverage (issue #788)"
fi

# Admitted registry matches the pins for every pair (issue #788).
if grep -q -F -e 'DX_CODEGEN_ADMITTED_PAIRS = (' generation/codegen.bzl &&
  grep -q -F -e '("protobuf", "rust")' generation/codegen.bzl &&
  grep -q -F -e 'WP1_ADMITTED_PAIRS = ("protobuf", "rust")' "$pins" &&
  grep -q -F -e 'ONBOARDING_PAIR_PROTOBUF_RUST' "$pins"; then
  ok
else
  bad "admitted-pair registry drifted between generation/codegen.bzl and pins.bzl (issue #788)"
fi

# Per-pair fixtures stay declared in the generation package (issue #788).
if grep -q -F -e 'name = "codegen_shard_alpha"' generation/BUILD.bazel &&
  grep -q -F -e 'name = "codegen_shard_beta"' generation/BUILD.bazel &&
  grep -q -F -e 'name = "codegen_prost_fixture"' generation/BUILD.bazel &&
  grep -q -F -e 'name = "codegen_plan_chain_subject"' generation/BUILD.bazel &&
  grep -q -F -e 'name = "codegen_plan_prost_subject"' generation/BUILD.bazel; then
  ok
else
  bad "generation/BUILD.bazel lost its per-pair onboarding fixtures (issue #788)"
fi

# Live proof: every admitted pair builds its fixtures green (issue #788).
if bazel build //generation:codegen_shard_alpha //generation:codegen_shard_beta //generation:codegen_prost_fixture //generation:codegen_plan_chain_subject //generation:codegen_plan_prost_subject --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "admitted-pair fixtures failed to build (want green protobuf/rust fixtures, issue #788)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'env_codegen_qualification' "$matrix" &&
  grep -q -F -e 'qualified seed-only under closed #506' "$matrix" &&
  grep -q -F -e 'bazel run //tools/ci:env_codegen_qualification' "$matrix" &&
  grep -q -F -e '`env_codegen_qualification` 52/52' "$matrix"; then
  ok
else
  bad "verification-matrix lost its #506 plus #787 plus #788 env codegen qualified record with 52/52"
fi

dx_test_summary "env/codegen qualification harness"
