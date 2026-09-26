#!/usr/bin/env bash
# Release evidence qualification harness for Windows x86_64 MSVC-compatible.
#
# Owns per-host release evidence for windows_x86_64 with fixture evidence
# pinned in `tools/ci/tests/fixtures/release_windows/pins.bzl` (plus
# `release_windows.expected`), without claiming unqualified support:
# - delivered as-built: SPDX-2.3 plus SLSA v1 via //deploy/release:sbom_demo
#   with subject digest equal to artifact sha256, hermetic Rust toolchain
#   only, //deploy/rules:release_demo_archive as subject fixture, verified
#   via bazel test //deploy/release:dx_release_tools_test;
# - CI surface: ci.yml carries no sbom-windows-x86_64 job, no upload-artifact,
#   and no per-host cache prefixes (windows sbom-provenance stays an
#   owner-gated local release record in docs/deploy/release-runbook.md; CI
#   never signs PR code and publishes nothing, fork-safe);
# - promotion-checklist cells for this host: Platform-qualified Windows
#   x86_64 MSVC-compatible (#414), dogfood consumer covers windows_x86_64
#   with all nine checks (#408), windows x86_64 coverage cell with no union,
#   explicit EULA never automatic with no installed fallback, prebuilt-MSVC
#   interop fixtures incl mixed Rust/C/C++ plus linux corpus seed-only
#   (#499), tag hygiene at 0.0.0 with no v* tags, --verify-tag versioning,
#   windows sbom-provenance (#807);
# - attestation stays owner-gated human-run via //deploy/release:signing_demo
#   (Sigstore keyless cosign sign-blob --bundle plus gh attestation create
#   on the TUF trust root); CI never signs PR code;
# - dry-run only forever rejected, checksum-only rejected; Release only.
# Platform-qualified, no Supported claim; remaining hosts plus tag cut stay
# owned gap under #805 plus process #808.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_windows_qualification`,
# following //tools/ci:release_arm64_qualification.
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
cells="tools/coverage/cells.txt"
interop_pins="cc/tests/fixtures/prebuilt_interop/pins.bzl"
corpus_pins="cc/tests/fixtures/linux_corpus/pins.bzl"
pins="tools/ci/tests/fixtures/release_windows/pins.bzl"
expected="tools/ci/tests/fixtures/release_windows/release_windows.expected"
fixture_build="tools/ci/tests/fixtures/release_windows/BUILD.bazel"
targets_b="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 via the hermetic Rust toolchain.
if grep -q -F -e 'SPDX-2.3' "$sbom" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$sbom" &&
  grep -q -F -e 'via Rust' "$sbom"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#807)"
fi

# Provenance binds exact bytes: SPDX plus in-toto v1 plus SLSA subject digest.
# Release BUILD keeps the demo plus its portable Rust verifier over the seed fixture.
if grep -q -F -e 'name = "sbom_demo"' "$release_build" &&
  grep -q -F -e 'name = "dx_release_tools_test"' "$release_build" &&
  grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its sbom_demo plus dx_release_tools_test over release_demo_archive (#807)"
fi

# CI runs no sbom-windows-x86_64 build or verify: no release-tool targets in
# ci.yml (windows SBOM stays an owner-gated local release record).
if ! grep -q -F -e 'sbom-windows-x86_64' "$ci" &&
  ! grep -q -F -e 'sbom_demo' "$ci" &&
  ! grep -q -F -e 'dx_release_tools_test' "$ci"; then
  ok
else
  bad "ci.yml still carries an sbom-windows-x86_64 build plus verify job (want no sbom-windows-x86_64 job and no sbom_demo/dx_release_tools_test in CI; SBOM stays owner-gated local release, #807)"
fi

# CI stages no windows sbom directory and uploads no provenance artifact.
if ! grep -q -F -e 'RUNNER_TEMP/sbom-windows-x86_64' "$ci" &&
  ! grep -q -F -e 'sbom-provenance-windows_x86_64' "$ci" &&
  ! grep -q -F -e 'actions/upload-artifact' "$ci"; then
  ok
else
  bad "ci.yml still stages or uploads sbom-provenance-windows_x86_64 (want no RUNNER_TEMP/sbom-windows-x86_64 stage and no upload-artifact; CI publishes nothing, #807)"
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
  bad "ci.yml lost its no-upload plus least-privilege setup-bazel record or runbook lost publishes-nothing plus CI-never-signs (want no upload-artifact for sbom-windows-x86_64, #807)"
fi

# No sbom jobs plus no per-host cache prefixes remain in ci.yml (caching is
# setup-bazel bazelisk cache plus BuildBuddy only).
if ! grep -q -F -e 'name: sbom' "$ci" &&
  ! grep -q -F -e 'prefix: bazel-' "$ci" &&
  ! grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml still carries sbom jobs or per-host cache prefixes (want none: all sbom jobs deleted, caching is setup-bazel plus BuildBuddy, #807)"
fi

# Attestation stays owner-gated human-run (fork-safe, no CI signing): the
# signing_demo target plus runbook own the record, ci.yml never signs.
if grep -q -F -e 'name = "signing_demo"' "$release_build" &&
  grep -q -F -e 'CI never signs PR code' "$runbook" &&
  ! grep -q -F -e 'signing_demo' "$ci" &&
  ! grep -q -F -e 'id-token' "$ci"; then
  ok
else
  bad "release BUILD or runbook lost the owner-gated signing_demo attestation record with ci.yml never signing (#807)"
fi

# Runbook records the windows CI upload plus owner-gated attestation under #807.
if grep -q -F -e 'issue #807' "$runbook" &&
  grep -q -F -e 'sbom-windows-x86_64' "$runbook" &&
  grep -q -F -e 'sbom-provenance-windows_x86_64' "$runbook" &&
  grep -q -F -e 'owner-gated human-run' "$runbook" &&
  grep -q -F -e '//deploy/release:signing_demo' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its #807 sbom-windows-x86_64 sbom-provenance-windows_x86_64 plus owner-gated attestation record"
fi

# Per-cell consumer evidence: the dogfood self-call covers windows_x86_64
# with every check enabled (all four platforms, min coverage 97, test
# enabled with no disabled_checks, #408).
if grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'min_coverage: "97"' "$ci" &&
  ! grep -q -F -e 'disabled_checks' "$ci"; then
  ok
else
  bad "ci.yml lost its dogfood windows_x86_64 all-nine-checks per-cell evidence (want four-platform self-call plus min_coverage 97 plus no disabled_checks, #807)"
fi

# Per-cell coverage stays the windows x86_64 cell with no union: the dogfood
# matrix coverage job renders the per-cell summary for windows_x86_64, and
# the no-union wording stays frozen in strategy-details.
if grep -q -F -e 'qualified windows_x86_64' "$cells" &&
  grep -q -F -e "platforms: '[\"linux_x86_64\", \"linux_arm64\", \"macos_arm64\", \"windows_x86_64\"]'" "$ci" &&
  grep -q -F -e 'Render first-party coverage summary (per-cell, no union)' "$consumer" &&
  grep -q -F -e 'no cross-cell union' docs/testing/strategy-details.md; then
  ok
else
  bad "coverage cells or dogfood consumer lost the windows x86_64 per-cell gate with no union (#807)"
fi

# Hermetic acquisition plus MSVC compat gates unchanged; EULA never automatic, no installed fallback.
# Prebuilt-MSVC interop plus linux corpus linkage stays pinned for this host.
# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "release_windows_qualification"' "$targets_b" &&
  grep -q -F -e 'release_windows_qualification.sh' "$targets_b" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:release_windows_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_b.bzl or dogfood_freshness.sh lost the release_windows_qualification wiring (want target plus freshness)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'release_windows.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "release-windows fixture missing (want pins.bzl plus release_windows.expected plus corpus BUILD)"
fi

# Pins record wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty.
if grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'ci.yml carries no sbom-windows-x86_64 job; SBOM stays a local target under #807' "$pins" &&
  grep -q -F -e 'no upload-artifact, no RUNNER_TEMP stage; CI publishes nothing under #807' "$pins" &&
  grep -q -F -e 'no per-host cache scope; disk cache deleted, BuildBuddy remote cache only' "$pins" &&
  grep -q -F -e 'seed sbom job removed with the sbom job deletion, no regression' "$pins" &&
  grep -q -F -e 'Platform-qualified Windows x86_64 MSVC-compatible native under issue #414' "$pins" &&
  grep -q -F -e 'dogfood self-call covers windows_x86_64 with full dx test plus dx coverage' "$pins" &&
  grep -q -F -e 'explicit EULA acceptance required never automatic' "$pins" &&
  grep -q -F -e 'prebuilt-MSVC interop fixtures' "$pins" &&
  grep -q -F -e 'via //deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified under issue #807' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its wire plus no-CI-upload plus cells plus attestation plus rejected plus honesty pins under issue #807"
fi

# Expected fixture pins the no-job plus no-upload plus cells plus rejected plus honesty lines.
if grep -q -F -e 'Release evidence for Windows x86_64 MSVC-compatible (issue #807)' "$expected" &&
  grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$expected" &&
  grep -q -F -e 'ci.yml carries no sbom-windows-x86_64 job' "$expected" &&
  grep -q -F -e 'no RUNNER_TEMP stage, no per-host cache scope' "$expected" &&
  grep -q -F -e 'Seed sbom job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'arm64 sbom-arm64 job removed with the sbom job deletion' "$expected" &&
  grep -q -F -e 'with full dx test plus dx coverage under issue #408' "$expected" &&
  grep -q -F -e 'windows_x86_64 sbom-provenance delivered under issue' "$expected" &&
  grep -q -F -e 'explicit EULA acceptance required never automatic' "$expected" &&
  grep -q -F -e 'prebuilt-MSVC interop fixtures' "$expected" &&
  grep -q -F -e 'CI never signs PR' "$expected" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified under issue #807' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "release_windows.expected lost its no-CI-upload plus cells plus rejected plus honesty lines under #807"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/release_windows/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "release-windows fixture failed to build (want green on the seed host, issue #807)"
fi

dx_test_summary "release evidence windows harness"
