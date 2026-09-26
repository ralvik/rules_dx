#!/usr/bin/env bash
# Release evidence qualification harness for macOS arm64 native.
#
# Owns per-host release evidence for macos_arm64 with fixture evidence
# pinned in `tools/ci/tests/fixtures/release_macos_arm64/pins.bzl` (plus
# `release_macos_arm64.expected`), without claiming unqualified support:
# - delivered as-built: SPDX-2.3 plus SLSA v1 via //deploy/release:sbom_demo
#   with subject digest equal to artifact sha256, hermetic Rust toolchain
#   only, //deploy/rules:release_demo_archive as subject fixture, verified
#   via bazel test //deploy/release:dx_release_tools_test;
# - CI surface: ci.yml carries no sbom-macos-arm64 job, no upload-artifact,
#   and no per-host cache prefixes (macos_arm64 sbom-provenance stays an
#   owner-gated local release record in docs/deploy/release-runbook.md; CI
#   never signs PR code and publishes nothing, fork-safe); pinned acquired
#   SDK with the hermetic-llvm Apple-SDK backend provisional and no host-installed SDK fallback never approved;
# - promotion-checklist cells for this host: Platform-qualified macOS arm64
#   (#412), dogfood consumer covers macos_arm64 with all nine checks (#408),
#   macos arm64 coverage cell with no union, tag hygiene at 0.0.0 with no
#   v* tags, --verify-tag versioning, macos_arm64 sbom-provenance (#805);
# - attestation stays owner-gated human-run via //deploy/release:signing_demo
#   (Sigstore keyless cosign sign-blob --bundle plus gh attestation create
#   on the TUF trust root); CI never signs PR code;
# - dry-run only forever rejected, checksum-only rejected; Release only.
# Platform-qualified, no Supported claim; remaining hosts plus tag cut stay
# owned gap under #806-#807 plus process #808.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_macos_arm64_qualification`,
# following //tools/ci:release_musl_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

sbom="deploy/release/sbom.bzl"
verify="deploy/release/src/lib.rs"
release_build="deploy/release/BUILD.bazel"
ci=".github/workflows/ci.yml"
consumer=".github/workflows/reusable-consumer.yml"
runbook="docs/deploy/release-runbook.md"
support="docs/product/support-matrix.md"
platform_rs="cli/cli/src/platform.rs"
cells="tools/coverage/cells.txt"
pins="tools/ci/tests/fixtures/release_macos_arm64/pins.bzl"
expected="tools/ci/tests/fixtures/release_macos_arm64/release_macos_arm64.expected"
fixture_build="tools/ci/tests/fixtures/release_macos_arm64/BUILD.bazel"
build="tools/ci/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 via the hermetic Rust toolchain.
if grep -q -F -e 'SPDX-2.3' "$sbom" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$sbom" &&
  grep -q -F -e 'via Rust' "$sbom"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#805)"
fi

# Provenance binds exact bytes: SPDX plus in-toto v1 plus SLSA subject digest.
if grep -q -F -e 'spdxVersion' "$verify" &&
  grep -q -F -e 'https://in-toto.io/Statement/v1' "$verify" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$verify"; then
  ok
else
  bad "Rust launch lost its SPDX plus in-toto plus SLSA subject-binding checks (#805)"
fi

# Release BUILD keeps the demo plus its portable Rust verifier over the seed fixture.
if grep -q -F -e 'name = "sbom_demo"' "$release_build" &&
  grep -q -F -e 'name = "dx_release_tools_test"' "$release_build" &&
  grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its sbom_demo plus dx_release_tools_test over release_demo_archive (#805)"
fi

# CI runs no sbom-macos-arm64 build or verify: no release-tool targets in
# ci.yml (macos arm64 SBOM stays an owner-gated local release record).
if ! grep -q -F -e 'sbom-macos-arm64' "$ci" &&
  ! grep -q -F -e 'sbom_demo' "$ci" &&
  ! grep -q -F -e 'dx_release_tools_test' "$ci"; then
  ok
else
  bad "ci.yml still carries an sbom-macos-arm64 build plus verify job (want no sbom-macos-arm64 job and no sbom_demo/dx_release_tools_test in CI; SBOM stays owner-gated local release, #805)"
fi

# CI stages no macos sbom directory and uploads no provenance artifact.
if ! grep -q -F -e 'RUNNER_TEMP/sbom-macos-arm64' "$ci" &&
  ! grep -q -F -e 'sbom-provenance-macos_arm64' "$ci" &&
  ! grep -q -F -e 'actions/upload-artifact' "$ci"; then
  ok
else
  bad "ci.yml still stages or uploads sbom-provenance-macos_arm64 (want no RUNNER_TEMP/sbom-macos-arm64 stage and no upload-artifact; CI publishes nothing, #805)"
fi

# No upload-artifact pin remains; checkout stays least-privilege plus
# setup-bazel, and the runbook records publishes-nothing plus CI-never-signs.
if ! grep -q -F -e 'actions/upload-artifact' "$ci" &&
  ! grep -q -F -e 'if-no-files-found: error' "$ci" &&
  grep -q -F -e 'persist-credentials: false' "$ci" &&
  grep -q -F -e 'bazel-contrib/setup-bazel' "$ci" &&
  grep -q -F -e 'publishes nothing' "$runbook" &&
  grep -q -F -e 'CI never signs PR code' "$runbook"; then
  ok
else
  bad "ci.yml lost its no-upload plus least-privilege setup-bazel record or runbook lost publishes-nothing plus CI-never-signs (want no upload-artifact for sbom-macos-arm64, #805)"
fi

# No sbom jobs plus no per-host cache prefixes remain in ci.yml (caching is
# setup-bazel bazelisk cache plus BuildBuddy only).
if ! grep -q -F -e 'name: sbom' "$ci" &&
  ! grep -q -F -e 'prefix: bazel-' "$ci" &&
  ! grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml still carries sbom jobs or per-host cache prefixes (want none: all sbom jobs deleted, caching is setup-bazel plus BuildBuddy, #805)"
fi

# Attestation stays owner-gated human-run (fork-safe, no CI signing): the
# signing_demo target plus runbook own the record, ci.yml never signs.
if grep -q -F -e 'name = "signing_demo"' "$release_build" &&
  grep -q -F -e 'CI never signs PR code' "$runbook" &&
  ! grep -q -F -e 'signing_demo' "$ci" &&
  ! grep -q -F -e 'id-token' "$ci"; then
  ok
else
  bad "release BUILD or runbook lost the owner-gated signing_demo attestation record with ci.yml never signing (#805)"
fi

# Runbook records the macos arm64 CI upload plus owner-gated attestation under #805.
if grep -q -F -e 'issue #805' "$runbook" &&
  grep -q -F -e 'sbom-macos-arm64' "$runbook" &&
  grep -q -F -e 'sbom-provenance-macos_arm64' "$runbook" &&
  grep -q -F -e 'owner-gated human-run' "$runbook" &&
  grep -q -F -e '//deploy/release:signing_demo' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its #805 sbom-macos-arm64 sbom-provenance-macos_arm64 plus owner-gated attestation record"
fi

# Per-cell consumer evidence: the dogfood self-call covers macos_arm64 with
# every check enabled (all four platforms, min coverage 97, test enabled
# with no disabled_checks, #408).
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'min_coverage: "97"' "$ci" &&
  ! grep -q -F -e 'disabled_checks' "$ci"; then
  ok
else
  bad "ci.yml lost its dogfood macos_arm64 all-nine-checks per-cell evidence (want four-platform self-call plus min_coverage 97 plus no disabled_checks, #805)"
fi

# Per-cell coverage stays the macos arm64 cell with no union: the dogfood
# matrix coverage job renders the per-cell summary for macos_arm64, and the
# no-union wording stays frozen in strategy-details.
if grep -q -F -e 'qualified macos_arm64' "$cells" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' "$consumer" &&
  grep -q -F -e 'no cross-cell union' docs/testing/strategy-details.md; then
  ok
else
  bad "coverage cells or dogfood consumer lost the macos arm64 per-cell gate with no union (#805)"
fi

# Pinned acquired SDK with provisional Apple-SDK backend; host-installed fallback never approved.
if grep -q -F -e 'host-installed SDK fallback never approved' "$platform_rs" &&
  grep -q -F -e 'backend stays provisional' "$platform_rs"; then
  ok
else
  bad "platform.rs lost the pinned acquired SDK plus provisional backend plus no-host-fallback record (#805)"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "release_macos_arm64_qualification"' "$targets_b" &&
  grep -q -F -e 'release_macos_arm64_qualification.sh' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_macos_arm64_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the release_macos_arm64_qualification wiring (want target plus freshness)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'release_macos_arm64.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "release-macos-arm64 fixture missing (want pins.bzl plus release_macos_arm64.expected plus corpus BUILD)"
fi

# Pins record wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty.
if grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'ci.yml carries no sbom-macos-arm64 job; SBOM stays a local target under #805' "$pins" &&
  grep -q -F -e 'no upload-artifact, no RUNNER_TEMP stage; CI publishes nothing under #805' "$pins" &&
  grep -q -F -e 'no per-host cache scope; disk cache deleted, BuildBuddy remote cache only' "$pins" &&
  grep -q -F -e 'seed sbom job removed with the sbom job deletion, no regression' "$pins" &&
  grep -q -F -e 'Platform-qualified macOS arm64 native under issue #412' "$pins" &&
  grep -q -F -e 'dogfood consumer self-call covers macos_arm64 with full dx test plus dx coverage' "$pins" &&
  grep -q -F -e 'macos arm64 coverage cell with no union' "$pins" &&
  grep -q -F -e 'pinned acquired SDK' "$pins" &&
  grep -q -F -e 'via //deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified under issue #805' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty pins under issue #805"
fi

# Expected fixture pins the no-job plus no-upload plus cells plus rejected plus honesty lines.
if grep -q -F -e 'Release evidence for macOS arm64 (issue #805)' "$expected" &&
  grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$expected" &&
  grep -q -F -e 'ci.yml carries no sbom-macos-arm64 job' "$expected" &&
  grep -q -F -e 'no RUNNER_TEMP stage, no per-host cache scope' "$expected" &&
  grep -q -F -e 'Seed sbom job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'arm64 sbom-arm64 job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'with full dx test plus dx coverage under issue #408' "$expected" &&
  grep -q -F -e 'macos_arm64 sbom-provenance delivered under issue' "$expected" &&
  grep -q -F -e 'CI never signs PR' "$expected" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified under issue #805' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "release_macos_arm64.expected lost its no-CI-upload plus cells plus rejected plus honesty lines under #805"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/release_macos_arm64/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "release-macos-arm64 fixture failed to build (want green on the seed host, issue #805)"
fi

dx_test_summary "release evidence macos arm64 harness"
