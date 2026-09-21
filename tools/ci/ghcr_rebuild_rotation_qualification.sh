#!/usr/bin/env bash
# GHCR rebuild plus signing rotation qualification harness.
#
# Owns the manual on-demand rebuild plus rotation left without an owner:
# - base image: ubuntu:24.04 digest-pinned FROM (resolved 2026-09-17,
#   re-pin deliberately, never latest);
# - Bazelisk: v1.29.0 launcher (per-OS shas in setup-bazelisk/action.yml,
#   Dockerfile tracks the linux-amd64 pair, Bazel 9.2.0 via
#   USE_BAZEL_VERSION);
# - Cosign: v2.4.1 checksum-verified fetch (single-sourced across
#   signing.bzl plus ghcr.yml plus sign_deploy.sh);
# - TUF trust: https://tuf-repo-cdn.sigstore.dev plus GitHub OIDC issuer
#   plus bundle media v0.3 (documented, not self-hosted);
# - cadence: manual on-demand rebuild plus rotation, recorded; triggers on
#   upstream release notice plus on base-image refresh or CVE plus before
#   any gated push; sole maintainer owns every row until delegation;
# - procedure: local docker build, re-pin deliberately with evidence in
#   one reviewed PR, gated push stays workflow_dispatch plus approve:true
#   with cosign sign plus verify;
# - qualification: locally or on demand with customer flows only (bazel
#   run harness plus on-demand local docker build, pushes nothing);
# - rejected: scheduled CI rebuild, push/schedule triggers, extra CI jobs;
#   keep CI customer-only;
# - fixtures: `tools/ci/tests/fixtures/ghcr_rebuild_rotation/` (`pins.bzl`
#   plus `ghcr_rebuild_rotation.expected`) pins base plus Bazelisk plus
#   Cosign plus trust plus cadence plus rejected plus honesty;
# - scope: infra only. Seed only: no Supported claim.
#
# Versioned here, run on demand via `bazel run //tools/ci:ghcr_rebuild_rotation_qualification`,
# following //tools/ci:runner_rotation_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="tools/ci/tests/fixtures/ghcr_rebuild_rotation/pins.bzl"
expected="tools/ci/tests/fixtures/ghcr_rebuild_rotation/ghcr_rebuild_rotation.expected"
fixture_build="tools/ci/tests/fixtures/ghcr_rebuild_rotation/BUILD.bazel"
contract="docs/contributing/devcontainer.md"
runbook="docs/deploy/release-runbook.md"
test_matrix="docs/testing/github-ci.md"
verify="docs/testing/verification-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
dockerfile=".devcontainer/Dockerfile.prebuilt"
action=".github/actions/setup-bazelisk/action.yml"
ghcr=".github/workflows/ghcr.yml"
signing="deploy/release/signing.bzl"
sign_deploy="deploy/release/src/lib.rs"

# Fixture files stay present.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]]; then
  ok
else
  bad "ghcr rebuild rotation fixture missing (want $pins plus ghcr_rebuild_rotation.expected plus BUILD.bazel)"
fi

# Pins record the base-image pin (digest, resolution, never-latest policy).
if grep -q -F -e 'ubuntu:24.04@sha256:69cecf4bbf72d2d44a9eef1b71fb98c7fb973d78af11399deccef19beb008ad9' "$pins" &&
  grep -q -F -e 'resolved 2026-09-17 from tag ubuntu:24.04' "$pins" &&
  grep -q -F -e 're-pin deliberately with evidence, never latest' "$pins"; then
  ok
else
  bad "pins.bzl lost its base-image pin (digest plus resolved 2026-09-17 plus never-latest, issue #647)"
fi

# Pins record the Bazelisk plus Bazel delegation (canonical source noted).
if grep -q -F -e 'v1.29.0 pinned Bazelisk launcher' "$pins" &&
  grep -q -F -e '5a408715e932c0250d28bd84555f12edbf70117de42f9181691c736eacc4a992' "$pins" &&
  grep -q -F -e '9.2.0 via USE_BAZEL_VERSION' "$pins" &&
  grep -q -F -e 'canonical source .github/actions/setup-bazelisk/action.yml' "$pins"; then
  ok
else
  bad "pins.bzl lost its Bazelisk plus Bazel delegation (v1.29.0 plus sha plus 9.2.0 plus canonical, issue #647)"
fi

# Pins record the Cosign single source plus TUF trust root.
if grep -q -F -e 'v2.4.1 checksum-verified fetch' "$pins" &&
  grep -q -F -e 'single-sourced SIGNING_COSIGN_VERSION plus ghcr.yml plus Rust launch' "$pins" &&
  grep -q -F -e 'https://tuf-repo-cdn.sigstore.dev' "$pins" &&
  grep -q -F -e 'https://token.actions.githubusercontent.com' "$pins" &&
  grep -q -F -e 'application/vnd.dev.sigstore.bundle.v0.3+json' "$pins"; then
  ok
else
  bad "pins.bzl lost its Cosign plus TUF trust pins (v2.4.1 plus single-sourced plus root plus issuer plus media, issue #647)"
fi

# Pins record manual on-demand cadence plus owner plus triggers.
if grep -q -F -e 'manual on-demand rebuild plus rotation, recorded' "$pins" &&
  grep -q -F -e 'on upstream release notice plus on base-image refresh or CVE plus before any gated push' "$pins" &&
  grep -q -F -e 'sole maintainer owns every row until delegation' "$pins"; then
  ok
else
  bad "pins.bzl lost its manual cadence plus triggers plus sole-maintainer owner under issue #647"
fi

# Pins record the manual rebuild procedure (local build plus reviewed PR plus gated push).
if grep -q -F -e 'docker build -f .devcontainer/Dockerfile.prebuilt -t dx-devcontainer:local .' "$pins" &&
  grep -q -F -e 're-pin deliberately with evidence in one reviewed PR' "$pins" &&
  grep -q -F -e 'workflow_dispatch plus approve: true with cosign sign plus verify' "$pins" &&
  grep -q -F -e 'qualified locally or on demand with customer flows only' "$pins" &&
  grep -q -F -e 'bazel run //tools/ci:ghcr_rebuild_rotation_qualification' "$pins"; then
  ok
else
  bad "pins.bzl lost its manual rebuild procedure (local docker build plus reviewed PR plus gated push plus customer flows, issue #647)"
fi

# Pins record the rejected scheduled rebuild plus customer-only boundary.
if grep -q -F -e 'scheduled CI rebuild rejected' "$pins" &&
  grep -q -F -e 'no push or schedule trigger' "$pins" &&
  grep -q -F -e 'no extra CI job' "$pins" &&
  grep -q -F -e 'keep CI customer-only' "$pins"; then
  ok
else
  bad "pins.bzl lost its scheduled-rebuild rejection plus no-push/schedule plus no-extra-job plus customer-only pins under #647"
fi

# Pins record honesty (seed-only, infra only, no Supported).
if grep -q -F -e 'qualified seed-only under issue #647' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins" &&
  grep -q -F -e 'infra only' "$pins"; then
  ok
else
  bad "pins.bzl lost its seed-only plus infra-only plus no-Supported honesty under #647"
fi

# Expected fixture pins base plus Bazelisk plus Cosign plus trust plus manual plus rejected.
if grep -q -F -e 'GHCR rebuild plus signing rotation manual (issue #647)' "$expected" &&
  grep -q -F -e 'ubuntu:24.04@sha256:69cecf4b...' "$expected" &&
  grep -q -F -e 'Bazelisk v1.29.0' "$expected" &&
  grep -q -F -e 'Cosign v2.4.1 checksum-verified fetch' "$expected" &&
  grep -q -F -e 'https://tuf-repo-cdn.sigstore.dev' "$expected" &&
  grep -q -F -e 'docker build -f' "$expected" &&
  grep -q -F -e 'Scheduled CI rebuild rejected' "$expected" &&
  grep -q -F -e 'Qualified seed-only under issue #647' "$expected"; then
  ok
else
  bad "ghcr_rebuild_rotation.expected lost its base plus Bazelisk plus Cosign plus trust plus manual plus rejected lines under #647"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'ghcr_rebuild_rotation.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "ghcr_rebuild_rotation BUILD.bazel lost its pins plus expected exports with corpus under #647"
fi

# Contract owns the manual rebuild plus rotation with fixture proof.
if grep -q -F -e 'GHCR Rebuild Plus Signing Rotation' "$contract" &&
  grep -q -F -e 'Manual on-demand rebuild plus rotation' "$contract" &&
  grep -q -F -e 'Scheduled CI rebuild rejected' "$contract" &&
  grep -q -F -e 'keep CI customer-only' "$contract" &&
  grep -q -F -e 'tools/ci/tests/fixtures/ghcr_rebuild_rotation/pins.bzl' "$contract" &&
  grep -q -F -e 'bazel run //tools/ci:ghcr_rebuild_rotation_qualification' "$contract" &&
  grep -q -F -e 'issue #647' "$contract"; then
  ok
else
  bad "devcontainer.md lost its GHCR rebuild plus signing rotation manual cadence plus rejected record with fixture proof under #647"
fi

# Runbook links the GHCR step to the rotation contract (no copy).
if grep -q -F -e 'GHCR rebuild plus signing rotation' "$runbook" &&
  grep -q -F -e 'contributing/devcontainer.md#ghcr-rebuild-plus-signing-rotation' "$runbook" &&
  grep -q -F -e 'issue #647' "$runbook"; then
  ok
else
  bad "deploy/release-runbook.md lost its GHCR rebuild rotation link to the contract under #647"
fi

# Test matrix records the on-demand customer-flows-only qualification.
if grep -q -F -e 'GHCR Rebuild Plus Signing Rotation' "$test_matrix" &&
  grep -q -F -e 'issue #647' "$test_matrix" &&
  grep -q -F -e 'bazel run //tools/ci:ghcr_rebuild_rotation_qualification' "$test_matrix"; then
  ok
else
  bad "testing/github-ci.md lost its #647 rebuild rotation on-demand qualification record"
fi

# BUILD owns the harness target plus CI wires it plus matrix records seed-only evidence.
if grep -q -F -e 'name = "ghcr_rebuild_rotation_qualification"' "$build" &&
  grep -q -F -e 'ghcr_rebuild_rotation_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:ghcr_rebuild_rotation_qualification' "$ci" &&
  grep -q -F -e ':ghcr_rebuild_rotation_qualification' "$verify" &&
  grep -q -F -e 'issue #647' "$verify"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml or verification-matrix lost the ghcr_rebuild_rotation_qualification wiring (want target plus audit step plus matrix)"
fi

# As-built pins stay single-sourced with no push/schedule CI.
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' "$dockerfile" &&
  grep -q -F -e 'USE_BAZEL_VERSION=9.2.0' "$dockerfile" &&
  grep -q -F -e 'sha256sum -c' "$dockerfile" &&
  grep -q -F -e 'default: "1.29.0"' "$action" &&
  grep -q -F -e 'COSIGN_VERSION="v2.4.1"' "$ghcr" &&
  grep -q -F -e 'SIGNING_COSIGN_VERSION = "v2.4.1"' "$signing" &&
  grep -q -F -e 'v2.4.1' "$sign_deploy" &&
  grep -q -F -e 'SIGNING_TRUST_ROOT = "https://tuf-repo-cdn.sigstore.dev"' "$signing" &&
  ! grep -q -E -e '^  (push|schedule):' "$ghcr"; then
  ok
else
  bad "as-built base plus Bazelisk plus Cosign plus TUF pins drifted or ghcr.yml gained a push/schedule trigger (want digest plus 9.2.0 plus 1.29.0 plus v2.4.1 plus trust root plus no push/schedule, issue #647)"
fi

# Live proof: customer build flow only, no push, no schedule, no new job.
if bazel build //tools/ci/tests/fixtures/ghcr_rebuild_rotation/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "ghcr-rebuild-rotation fixture failed to build (want green via customer build flow, issue #647)"
fi

dx_test_summary "ghcr rebuild rotation qualification harness"
