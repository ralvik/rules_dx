#!/usr/bin/env bash
# SBOM plus provenance upload qualification harness.
#
# Owns SBOM plus provenance as-built plus owner-gated release evidence with
# fixture evidence pinned in `tools/ci/tests/fixtures/sbom_upload/pins.bzl`
# (plus `sbom_upload.expected`), without claiming unqualified support:
# - delivered as-built: SPDX-2.3 plus SLSA v1 via //deploy/release:sbom_demo
#   with subject digest equal to artifact sha256, hermetic Rust toolchain
#   only, //deploy/rules:release_demo_archive as subject fixture, verified
#   via bazel test //deploy/release:dx_release_tools_test;
# - CI surface: ci.yml carries no sbom job, no upload-artifact, and no
#   per-host cache prefixes (SBOM plus provenance stays an owner-gated
#   local release path in docs/deploy/release-runbook.md; CI never signs
#   PR code and publishes nothing, fork-safe);
# - attestation stays owner-gated human-run via //deploy/release:signing_demo
#   (Sigstore keyless cosign sign-blob --bundle plus gh attestation create
#   on the TUF trust root); CI never signs PR code;
# - dry-run only forever rejected, checksum-only rejected; Release only.
# Seed only; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:sbom_upload_qualification`,
# following //tools/ci:supported_evidence_gate.
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
dryrun=".github/workflows/publish-dry-run.yml"
runbook="docs/deploy/release-runbook.md"
authoring="docs/deploy/authoring.md"
pins="tools/ci/tests/fixtures/sbom_upload/pins.bzl"
expected="tools/ci/tests/fixtures/sbom_upload/sbom_upload.expected"
fixture_build="tools/ci/tests/fixtures/sbom_upload/BUILD.bazel"
build="tools/ci/ci_targets_b.bzl"
freshness="tools/ci/dogfood_freshness.sh"

# SBOM wire profile stays pinned: SPDX-2.3 plus SLSA v1 via the hermetic Rust toolchain.
if grep -q -F -e 'SPDX-2.3' "$sbom" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' "$sbom" &&
  grep -q -F -e 'via Rust' "$sbom"; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#612)"
fi

# Provenance binds exact bytes: SPDX plus in-toto v1 plus SLSA subject digest.
# Release BUILD keeps the demo plus its portable Rust verifier.
if grep -q -F -e 'name = "sbom_demo"' "$release_build" &&
  grep -q -F -e 'name = "dx_release_tools_test"' "$release_build" &&
  grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build"; then
  ok
else
  bad "deploy/release/BUILD.bazel lost its sbom_demo plus dx_release_tools_test over release_demo_archive (#612)"
fi

# SBOM subject stays the seed fixture archive (no new binary, exact bytes).
if grep -q -F -e '//deploy/rules:release_demo_archive' "$release_build" &&
  grep -q -F -e 'seed release tarball' "$release_build"; then
  ok
else
  bad "sbom_demo lost its release_demo_archive subject fixture record (#612)"
fi

# CI never builds or verifies SBOM: no sbom job plus no release-tool test
# in ci.yml (SBOM stays an owner-gated local release path, not push/PR).
if ! grep -q -F -e 'sbom' "$ci" &&
  ! grep -q -F -e 'dx_release_tools_test' "$ci"; then
  ok
else
  bad "ci.yml still carries an sbom build plus verify job (want no sbom job and no dx_release_tools_test in CI; SBOM stays owner-gated local release, #612)"
fi

# CI stages no sbom directory and uploads no provenance artifact.
if ! grep -q -F -e 'RUNNER_TEMP/sbom' "$ci" &&
  ! grep -q -F -e 'actions/upload-artifact' "$ci" &&
  ! grep -q -F -e 'sbom-provenance' "$ci"; then
  ok
else
  bad "ci.yml still stages or uploads sbom-provenance (want no RUNNER_TEMP/sbom stage and no upload-artifact; CI publishes nothing, #612)"
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
  bad "ci.yml lost its no-upload plus least-privilege setup-bazel record or runbook lost publishes-nothing plus CI-never-signs (want no upload-artifact in CI, #612)"
fi

# Dispatch dry-run still exercises the SBOM demo (owner-gated, publishes nothing).
if grep -q -F -e '//deploy/release:sbom_demo' "$dryrun" &&
  grep -q -F -e 'bazel test //deploy/release:all' "$dryrun"; then
  ok
else
  bad "publish-dry-run.yml lost its sbom_demo plus release-tests exercise (#612)"
fi

# Runbook records the CI upload plus owner-gated attestation under.
if grep -q -F -e 'issue #612' "$runbook" &&
  grep -q -F -e 'sbom-provenance' "$runbook" &&
  grep -q -F -e 'owner-gated human-run' "$runbook" &&
  grep -q -F -e '//deploy/release:signing_demo' "$runbook"; then
  ok
else
  bad "release-runbook.md lost its #612 CI sbom-provenance plus owner-gated attestation record"
fi

# Authoring doc records the CI SBOM upload on the release path
# (environment.md dropped the duplicate record under #985; runbook plus
# authoring own the as-built text).
if grep -q -F -e 'issue #612' "$authoring" &&
  grep -q -F -e 'sbom-provenance' "$authoring"; then
  ok
else
  bad "authoring.md lost its #612 CI sbom-provenance upload record"
fi

# BUILD owns the harness target plus dogfood-freshness wires it
# (targets live in the ci_targets_b shard after the BUILD split, #652;
# ci.yml invokes dogfood_freshness, which lists this harness).
if grep -q -F -e 'name = "sbom_upload_qualification"' "$build" &&
  grep -q -F -e 'sbom_upload_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:sbom_upload_qualification' "$freshness" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:dogfood_freshness' "$ci"; then
  ok
else
  bad "ci_targets_b.bzl or dogfood_freshness.sh lost the sbom_upload_qualification wiring (want target plus freshness)"
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

# Pins record wire plus no-CI-upload plus attestation plus rejected plus honesty.
if grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'https://slsa.dev/provenance/v1 via //deploy/release:sbom_demo' "$pins" &&
  grep -q -F -e 'ci.yml carries no sbom job; SBOM stays a local target under #612' "$pins" &&
  grep -q -F -e 'no upload-artifact, no RUNNER_TEMP stage; CI publishes nothing under #612' "$pins" &&
  grep -q -F -e 'no per-host cache scope; disk cache deleted, BuildBuddy remote cache only' "$pins" &&
  grep -q -F -e 'via //deploy/release:signing_demo' "$pins" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$pins" &&
  grep -q -F -e 'Compatibility: Release only' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #612' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its wire plus no-CI-upload plus attestation plus rejected plus honesty pins under issue #612"
fi

# Expected fixture pins the no-job plus no-upload plus rejected plus honesty lines.
if grep -q -F -e 'SBOM plus provenance with no CI upload (issue #612)' "$expected" &&
  grep -q -F -e 'SPDX-2.3 via //deploy/release:sbom_demo' "$expected" &&
  grep -q -F -e 'ci.yml carries no sbom job, no upload-artifact' "$expected" &&
  grep -q -F -e 'no RUNNER_TEMP stage, no per-host cache scope' "$expected" &&
  grep -q -F -e 'CI never signs PR' "$expected" &&
  grep -q -F -e 'Dry-run only forever is rejected' "$expected" &&
  grep -q -F -e 'Compatibility: Release only' "$expected" &&
  grep -q -F -e 'Qualified seed-only' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "sbom_upload.expected lost its no-CI-upload plus rejected plus honesty lines under #612"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/sbom_upload/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "sbom-upload fixture failed to build (want green on the seed host, issue #612)"
fi

dx_test_summary "sbom plus provenance upload harness"
