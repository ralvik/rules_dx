#!/usr/bin/env bash
# SBOM plus provenance upload qualification harness.
#
# Owns SBOM plus provenance build plus verify plus upload on CI with fixture
# evidence pinned in `tools/ci/tests/fixtures/sbom_upload/pins.bzl` (plus
# `sbom_upload.expected`), without claiming unqualified support:
# - delivered as-built: SPDX-2.3 plus SLSA v1 via //deploy/release:sbom_demo
#   with subject digest equal to artifact sha256, managed Python toolchain
#   only, //deploy/rules:release_demo_archive as subject fixture, verified
#   via bazel test //deploy/release:sbom_demo_verify;
# - CI upload: ci.yml sbom job builds plus verifies on every push/PR (seed
#   host), stages under RUNNER_TEMP/sbom, uploads sbom-provenance via
#   actions/upload-artifact pinned SHA plus tag, contents read only,
#   publishes nothing, fork-safe;
# - attestation stays owner-gated human-run via //deploy/release:signing_demo
#   (Sigstore keyless cosign sign-blob --bundle plus gh attestation create
#   on the TUF trust root); CI never signs PR code;
# - dry-run only forever rejected, checksum-only rejected; Release only.
# Seed only; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:sbom_upload_qualification`,
# following //tools/ci:promotion_checklist_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

sbom="deploy/release/sbom.bzl"
verify="deploy/release/sbom_verify.sh"
release_build="deploy/release/BUILD.bazel"
ci=".github/workflows/ci.yml"
dryrun=".github/workflows/publish-dry-run.yml"
runbook="docs/deploy/release-runbook.md"
env_doc="docs/environments/environment.md"
authoring="docs/deploy/authoring.md"
pins="tools/ci/tests/fixtures/sbom_upload/pins.bzl"
expected="tools/ci/tests/fixtures/sbom_upload/sbom_upload.expected"
fixture_build="tools/ci/tests/fixtures/sbom_upload/BUILD.bazel"
build="tools/ci/BUILD.bazel"
verify_matrix="docs/testing/verification-matrix.md"

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 via the managed toolchain.
if grep -q -F -e 'SPDX-2.3' "$sbom" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$sbom" &&
  grep -q -F -e 'managed Python' "$sbom"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#612)"
fi

# Provenance binds exact bytes: SPDX plus in-toto v1 plus SLSA subject digest.
if grep -q -F -e '"spdxVersion": "SPDX-2.3"' "$verify" &&
  grep -q -F -e '"_type": "https://in-toto.io/Statement/v1"' "$verify" &&
  grep -q -F -e '"predicateType": "https://slsa.dev/provenance/v1"' "$verify"; then
  ok
else
  bad "sbom_verify.sh lost its SPDX plus in-toto plus SLSA subject-binding checks (#612)"
fi

# Release BUILD keeps the demo plus its verifier.
if grep -q -F -e 'name = "sbom_demo"' "$release_build" &&
  grep -q -F -e 'name = "sbom_demo_verify"' "$release_build" &&
  grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its sbom_demo plus sbom_demo_verify over release_demo_archive (#612)"
fi

# SBOM subject stays the seed fixture archive (no new binary, exact bytes).
if grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build" &&
  grep -q -F -e 'seed release tarball' "$release_build"; then
  ok
else
  bad "sbom_demo lost its release_demo_archive subject fixture record (#612)"
fi

# CI sbom job builds plus verifies on every push/PR (seed host, not dispatch-only).
if grep -q -F -e 'name: sbom (SBOM + provenance build/verify/upload, seed host)' "$ci" &&
  grep -q -F -e 'bazel build --noshow_progress //deploy/release:sbom_demo' "$ci" &&
  grep -q -F -e 'bazel test --noshow_progress //deploy/release:sbom_demo_verify' "$ci"; then
  ok
else
  bad "ci.yml lost its sbom build plus verify on push/PR (want sbom job with sbom_demo plus sbom_demo_verify, #612)"
fi

# CI sbom job stages plus uploads as an artifact for inspection.
if grep -q -F -e 'RUNNER_TEMP/sbom' "$ci" &&
  grep -q -F -e 'actions/upload-artifact@' "$ci" &&
  grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml lost its sbom stage plus upload-artifact sbom-provenance (#612)"
fi

# Upload stays pinned plus fail-closed plus least-privilege, publishes nothing.
if grep -q -F -e 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4' "$ci" &&
  grep -q -F -e 'if-no-files-found: error' "$ci" &&
  grep -q -F -e 'persist-credentials: false' "$ci" &&
  grep -q -F -e 'publishes nothing' "$ci"; then
  ok
else
  bad "ci.yml lost its pinned upload-artifact plus fail-closed plus publishes-nothing record (#612)"
fi

# Dispatch dry-run still exercises the SBOM demo (owner-gated, publishes nothing).
if grep -q -F -e '//deploy/release:sbom_demo' "$dryrun" &&
  grep -q -F -e 'bazel test //deploy/release:all' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its sbom_demo plus release-tests exercise (#612)"
fi

# Runbook records CI upload plus owner-gated attestation under.
if grep -q -F -e 'issue #612' "$runbook" &&
  grep -q -F -e 'sbom-provenance' "$runbook" &&
  grep -q -F -e 'owner-gated human-run' "$runbook" &&
  grep -q -F -e '//deploy/release:signing_demo' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its #612 CI sbom-provenance plus owner-gated attestation record"
fi

# Environment doc records the CI SBOM upload (dispatch vs push split).
if grep -q -F -e 'issue #612' "$env_doc" &&
  grep -q -F -e 'sbom-provenance' "$env_doc"; then
  ok
else
  bad "environment.md lost its #612 CI sbom-provenance upload record"
fi

# Authoring doc records the CI SBOM upload on the release path.
if grep -q -F -e 'issue #612' "$authoring" &&
  grep -q -F -e 'sbom-provenance' "$authoring"; then
  ok
else
  bad "authoring.md lost its #612 CI sbom-provenance upload record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "sbom_upload_qualification"' "$build" &&
  grep -q -F -e 'sbom_upload_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:sbom_upload_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the sbom_upload_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture files stay present with corpus coverage.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]] &&
  grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'sbom_upload.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "sbom-upload fixture missing (want pins.bzl plus sbom_upload.expected plus corpus BUILD)"
fi

# Pins record wire plus CI plus attestation plus rejected plus honesty.
if grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'ci.yml sbom job builds //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'uploads sbom-provenance via actions/upload-artifact' "$pins" &&
  grep -q -F -e 'via //deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #612' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its wire plus CI plus attestation plus rejected plus honesty pins under issue #612"
fi

# Expected fixture pins the upload plus rejected plus honesty lines.
if grep -q -F -e 'SBOM plus provenance upload on CI (issue #612)' "$expected" &&
  grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$expected" &&
  grep -q -F -e 'uploads sbom-provenance' "$expected" &&
  grep -q -F -e 'pinned SHA plus tag' "$expected" &&
  grep -q -F -e 'CI never signs PR' "$expected" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified seed-only' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "sbom_upload.expected lost its upload plus rejected plus honesty lines under #612"
fi

# Verification matrix owns the harness entry as seed-only fixture evidence.
if grep -q -F -e ':sbom_upload_qualification' "$verify_matrix" &&
  grep -q -F -e 'issue #612' "$verify_matrix" &&
  grep -q -F -e '`sbom_upload_qualification` 17/17' "$verify_matrix"; then
  ok
else
  bad "verification-matrix.md lost its sbom_upload_qualification entry with 17/17 under #612"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/sbom_upload/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "sbom-upload fixture failed to build (want green on the seed host, issue #612)"
fi

dx_test_summary "sbom plus provenance upload harness"
