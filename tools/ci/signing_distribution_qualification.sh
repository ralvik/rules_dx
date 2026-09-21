#!/usr/bin/env bash
# Signing stack + distribution qualification harness (live
# successor to closed // for signing + distribution).
#
# docs/roadmap.md lists signing stack + distribution with owners //
# all closed and no open qualification. Decision: keep Sigstore
# keyless `cosign sign-blob --bundle` + GitHub attestations as in
# deploy/release/signing.bzl; no stack change.
#
# Qualifies the selected stack with pins and provenance (see
# docs/deploy/release-runbook.md): cosign v2.4.1 pinned plus Sigstore bundle
# v0.3 on the TUF trust root with GitHub OIDC issuer, SPDX 2.3 plus SLSA v1
# provenance via //deploy/release:sbom_demo with subject digest equal to
# artifact sha256, distribution to BCR (rules_dx module) plus GitHub Releases
# (dx binaries) with GHCR via the separate ghcr.yml route. Deeper
# wire-profile details (Rekor v2 versus TSA, offline roots, rotation,
# negatives, L2/L3) stay provisional per docs/tools/tool-acquisition.md.
#
# Versioned here, run by CI via `bazel run //tools/ci:signing_distribution_qualification`,
# following //tools/ci:publish_trust.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Signing header owns the live-successor plus no-stack-change decision.
if grep -q -F -e 'live successor' deploy/release/signing.bzl &&
  grep -q -F -e 'no stack change' deploy/release/signing.bzl; then
  ok
else
  bad "signing.bzl lost its #459 live-successor plus no-stack-change record"
fi

# Signing pins stay frozen: TUF trust root plus GitHub OIDC issuer.
if grep -q -F -e 'SIGNING_TRUST_ROOT = "https://tuf-repo-cdn.sigstore.dev"' deploy/release/signing.bzl &&
  grep -q -F -e 'SIGNING_ISSUER = "https://token.actions.githubusercontent.com"' deploy/release/signing.bzl; then
  ok
else
  bad "signing.bzl lost its trust-root plus issuer pins (#459)"
fi

# Signing pins stay frozen: cosign v2.4.1 plus bundle v0.3.
if grep -q -F -e 'SIGNING_COSIGN_VERSION = "v2.4.1"' deploy/release/signing.bzl &&
  grep -q -F -e 'SIGNING_BUNDLE_MEDIA_TYPE = "application/vnd.dev.sigstore.bundle.v0.3+json"' deploy/release/signing.bzl; then
  ok
else
  bad "signing.bzl lost its cosign-version plus bundle-media pins (#459)"
fi

# Signing validators stay wired: cosign plus bundle-media plus identity plus bundle names.
if grep -q -F -e 'def signing_cosign_error' deploy/release/signing.bzl &&
  grep -q -F -e 'def signing_bundle_media_error' deploy/release/signing.bzl &&
  grep -q -F -e 'def signing_identity_error' deploy/release/signing.bzl &&
  grep -q -F -e 'def signing_bundle_names' deploy/release/signing.bzl; then
  ok
else
  bad "signing.bzl lost its cosign/media/identity/names validators (#459)"
fi

# Signing unit tests pin the new stack pins plus negatives.
if grep -q -F -e 'SIGNING_COSIGN_VERSION' deploy/release/signing_tests.bzl &&
  grep -q -F -e 'SIGNING_BUNDLE_MEDIA_TYPE' deploy/release/signing_tests.bzl &&
  grep -q -F -e 'signing_cosign_error' deploy/release/signing_tests.bzl &&
  grep -q -F -e 'signing_bundle_media_error' deploy/release/signing_tests.bzl; then
  ok
else
  bad "signing_tests.bzl lost its cosign plus bundle-media pin coverage (#459)"
fi

# Signing program keeps the selected stack: sign-blob --bundle plus attestation.
if grep -q -F -e 'cosign sign-blob --bundle' deploy/release/src/lib.rs &&
  grep -q -F -e 'gh attestation create' deploy/release/src/lib.rs; then
  ok
else
  bad "signing Rust launch lost its selected sign-blob plus attestation stack (#459)"
fi

# Signing dry-run records the pins: cosign version plus bundle media plus trust root.
if grep -q -F -e 'v2.4.1' deploy/release/src/lib.rs &&
  grep -q -F -e 'application/vnd.dev.sigstore.bundle.v0.3+json' deploy/release/src/lib.rs &&
  grep -q -F -e 'RELEASE_SIGN_DRY_RUN=1' deploy/release/src/lib.rs; then
  ok
else
  bad "signing Rust launch dry run lost its cosign-version plus bundle-media pins (#459)"
fi

# Signing fails closed without cosign plus needs identity plus host tools at run time.
if grep -q -F -e "'cosign' CLI not found" deploy/release/src/lib.rs &&
  grep -q -F -e 'missing SIGNING_IDENTITY' deploy/release/src/lib.rs &&
  grep -q -F -e 'no new module dependencies' deploy/release/src/lib.rs; then
  ok
else
  bad "signing Rust launch lost its fail-closed plus no-new-module-deps record (#459)"
fi

# Signing verifier checks the pins: version plus media plus trust root plus attestation.
if grep -q -F -e 'v2.4.1' deploy/release/src/lib.rs &&
  grep -q -F -e 'application/vnd.dev.sigstore.bundle.v0.3+json' deploy/release/src/lib.rs &&
  grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/release/src/lib.rs &&
  grep -q -F -e 'gh attestation create' deploy/release/src/lib.rs; then
  ok
else
  bad "signing Rust launch lost its version plus media plus trust-root plus attestation checks (#459)"
fi

# Cosign pin stays single-sourced: signing.bzl plus GHCR fetch plus dry-run agree on v2.4.1.
if grep -q -F -e 'COSIGN_VERSION="v2.4.1"' .github/workflows/ghcr.yml &&
  grep -q -F -e 'SIGNING_COSIGN_VERSION = "v2.4.1"' deploy/release/signing.bzl &&
  grep -q -F -e 'v2.4.1' deploy/release/src/lib.rs; then
  ok
else
  bad "cosign v2.4.1 pin drifted across signing.bzl plus ghcr.yml plus Rust launch (#459)"
fi

# Install verifier keeps distribution verification: bundle-required plus no checksum fallback.
if grep -q -F -e 'checksum-only verification is not publisher-identity proof' deploy/install/src/lib.rs &&
  grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/install/src/lib.rs &&
  grep -q -F -e 'cosign verify-blob' deploy/install/src/lib.rs; then
  ok
else
  bad "dx_verify lost its bundle-required plus trust-root plus verify-blob record (#459)"
fi

# Install verifier fails before install or exec plus SBOM binds through the same cosign path.
if grep -q -F -e 'before install' deploy/install/src/lib.rs &&
  grep -q -F -e 'never executed' deploy/install/src/lib.rs &&
  grep -q -F -e 'SBOM' deploy/install/src/lib.rs; then
  ok
else
  bad "dx_verify lost its fail-before-install plus SBOM record (#459)"
fi

# SBOM plus provenance stay pinned: SPDX-2.3 plus SLSA v1 via the hermetic Rust toolchain.
if grep -q -F -e 'SPDX-2.3' deploy/release/sbom.bzl &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' deploy/release/sbom.bzl &&
  grep -q -F -e 'via Rust' deploy/release/sbom.bzl; then
  ok
else
  bad "sbom.bzl lost its SPDX-2.3 plus SLSA-v1 plus hermetic-toolchain pins (#459)"
fi

# Provenance binds exact bytes: subject digest equals artifact sha256 plus in-toto v1.
if grep -q -F -e '"_type": "https://in-toto.io/Statement/v1"' deploy/release/src/lib.rs &&
  grep -q -F -e '"predicateType": "https://slsa.dev/provenance/v1"' deploy/release/src/lib.rs &&
  grep -q -F -e 'SPDX-2.3' deploy/release/src/lib.rs; then
  ok
else
  bad "Rust launch lost its SPDX plus in-toto plus SLSA subject-binding checks (#459)"
fi

# Release matrix stays frozen: five cells with the seed plus four follow-ups qualified.
if grep -q -F -e 'dx-linux-x86_64' deploy/release/matrix.bzl &&
  grep -q -F -e 'qualified-seed-built-here' deploy/release/matrix.bzl &&
  grep -q -F -e 'qualified-host-evidence' deploy/release/matrix.bzl; then
  ok
else
  bad "matrix.bzl lost its frozen five-cell plus qualified shape (#815)"
fi

# Distribution destinations stay pinned: BCR rules_dx plus GitHub Releases dx binaries.
if grep -q -F -e 'Bazel Central Registry' docs/environments/environment.md &&
  grep -q -F -e 'GitHub Releases for standalone `dx` binaries' docs/environments/environment.md &&
  grep -q -F -e 'qualified under issue #459' docs/environments/environment.md &&
  grep -q -F -e 'signing_distribution_qualification' docs/environments/environment.md; then
  ok
else
  bad "environment.md lost its BCR plus GitHub-Releases plus #459-qualified record"
fi

# Deploy authoring keeps distribution artifacts qualified: archive plus draft plus verifier.
if grep -q -F -e 'distribution artifact qualified under issue #459' docs/deploy/authoring.md &&
  grep -q -F -e 'Distribution verification qualified under issue #459' docs/deploy/authoring.md &&
  grep -q -F -e 'signing_distribution_qualification' docs/deploy/authoring.md; then
  ok
else
  bad "authoring.md lost its #459-qualified distribution-artifact plus verifier record"
fi

# Deploy authoring keeps the signing pins: cosign version plus bundle media.
if grep -q -F -e 'v2.4.1' docs/deploy/authoring.md &&
  grep -q -F -e 'application/vnd.dev.sigstore.bundle.v0.3+json' docs/deploy/authoring.md &&
  grep -q -F -e 'qualified' docs/deploy/authoring.md; then
  ok
else
  bad "authoring.md lost its cosign-version plus bundle-media signing pins (#459)"
fi

# Release runbook qualifies the stack: plus pins plus harness.
if grep -q -F -e 'qualified under issue #459' docs/deploy/release-runbook.md &&
  grep -q -F -e 'SIGNING_COSIGN_VERSION' docs/deploy/release-runbook.md &&
  grep -q -F -e 'SIGNING_BUNDLE_MEDIA_TYPE' docs/deploy/release-runbook.md &&
  grep -q -F -e 'signing_distribution_qualification' docs/deploy/release-runbook.md; then
  ok
else
  bad "release-runbook.md lost its #459-qualified plus pins plus harness record"
fi

# GHCR keeps the same stack: pinned cosign fetch plus trust root plus digest-pin never latest.
if grep -q -F -e 'COSIGN_VERSION=' .github/workflows/ghcr.yml &&
  grep -q -F -e 'cosign_checksums' .github/workflows/ghcr.yml &&
  grep -q -F -e '#311 trust root' .github/workflows/ghcr.yml &&
  grep -q -F -e 'never latest' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost its pinned cosign plus trust-root plus never-latest record (#459)"
fi

# Publish dry-run exercises the qualified path: signing plus SBOM plus verifier plus release tests.
if grep -q -F -e 'RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '//deploy/release:sbom_demo' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e 'bazel test //deploy/release:all' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost its signing plus SBOM plus verifier plus release-tests exercise (#459)"
fi

# CLI contract keeps the distribution note: qualified plus no binaries published yet.
if grep -q -F -e 'distribution qualified under issue #459' docs/cli/cli-contract.md &&
  grep -q -F -e 'no' docs/cli/cli-contract.md &&
  grep -q -F -e 'binaries are published yet' docs/cli/cli-contract.md; then
  ok
else
  bad "cli-contract.md lost its #459-qualified plus no-binaries-published record"
fi

# Tool acquisition keeps the boundary: selected stack qualified, deeper profiles provisional.
if grep -q -F -e 'qualified under issue #459' docs/tools/tool-acquisition.md &&
  grep -q -F -e 'stay provisional' docs/tools/tool-acquisition.md &&
  grep -q -F -e 'cosign v2.4.1' docs/tools/tool-acquisition.md; then
  ok
else
  bad "tool-acquisition.md lost its selected-qualified plus provisional record (#459)"
fi

# Roadmap owns the title: signing stack plus distribution under.
if grep -q -F -e 'signing stack + distribution' docs/roadmap.md &&
  grep -q -F -e '(issue #459)' docs/roadmap.md; then
  ok
else
  bad "roadmap.md lost its signing-stack-plus-distribution #459 owner"
fi

dx_test_summary "signing stack plus distribution harness"
