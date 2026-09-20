#!/usr/bin/env bash
# GHCR prebuilt-image hygiene (live successor to closed 
# for the GHCR route; trust root shared with).
#
# The prebuilt devcontainer image removes per-create feature-install cost,
# but publication is gated: separate workflow from releases per owner
# decision, build on PR, push only on workflow_dispatch + approve:true,
# signing owner-gated after push (cosign sign --yes <image>@<digest> keyless
# on the trust root, same as //deploy/release:signing_demo, plus
# cosign verify / gh attestation; dry-run would-sign on PRs). Image tags
# track the single-version dx == module pin (0.0.0-sha-<sha>); the scaffold
# `image:` digest pin (never latest) lands with the first push, quota is
# qualified in docs (container free for public, 1-month notice) and exact
# bytes are recorded on push; build-only PRs push nothing.
#
# This harness machine-checks the gate half verifiable on a clean tree
# today (22 checks): separate ghcr.yml route, PR-paths build, dispatch +
# default-closed approve gate, typed approve, push run-gate explicit, no push/tag/schedule trigger, digest-pinned
# base + Bazelisk delegation + no ambient toolchains in Dockerfile.prebuilt,
# no docker/* or sigstore/* actions with checkout SHA-pinned, no-secrets checkout plus
# non-cancelling concurrency, cosign sign/verify + attestation with pinned
# fetch, version-tracked tags, id-token keyless scope, trust root,
# scaffold-digest procedure + qualified quota, scaffold still on mcr (switch follows first push).
#
# Versioned here, run by CI via `bazel run //tools/ci:ghcr_hygiene`,
# following //tools/ci:examples_laziness_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Separate workflow from releases per owner decision (never fold GHCR
# into publish-dry-run.yml; different lifecycle per-scaffold-change).
if [[ -f .github/workflows/ghcr.yml && -f .github/workflows/publish-dry-run.yml ]]; then
  ok
else
  bad "ghcr.yml or publish-dry-run.yml missing (workflows must stay separate)"
fi

# Build on PR over the scaffold inputs only.
if grep -q -F -e '.devcontainer/Dockerfile.prebuilt' .github/workflows/ghcr.yml && grep -q -F -e '.github/workflows/ghcr.yml' .github/workflows/ghcr.yml && grep -q -F -e '.devcontainer/devcontainer.json' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the PR-paths build scope (Dockerfile/workflow/devcontainer.json)"
fi

# Push gated on explicit owner approval: workflow_dispatch + approve
# default false; PRs build only, nothing pushes uninvited.
if grep -q -F -e 'workflow_dispatch:' .github/workflows/ghcr.yml && grep -q -F -e 'approve:' .github/workflows/ghcr.yml && grep -q -F -e 'default: false' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the dispatch + default-closed approve gate"
fi

# The approve gate stays typed (parity with): boolean
# input type so overlapping dispatches queue via typed approval instead
# of stringly-typed approval.
if grep -q -F -e 'type: boolean' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the typed approve gate (approve must be type boolean)"
fi

# Never push/tag/schedule-triggered: only pull_request (build-only) and
# workflow_dispatch (gated push) may appear as top-level triggers.
if ! grep -q -E -e '^  (push|schedule):' .github/workflows/ghcr.yml && grep -q -E -e '^  pull_request:' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml gained a push/schedule trigger or lost pull_request build"
fi

# Push run-gate explicit: the push step must check event_name is
# workflow_dispatch and APPROVE is true, so PR builds never push even if
# the inputs block is edited; default stays build-only.
if grep -q -F -e 'github.event_name' .github/workflows/ghcr.yml && grep -q -F -e 'workflow_dispatch' .github/workflows/ghcr.yml && grep -q -F -e '"$APPROVE" != "true"' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the explicit push run-gate (dispatch + APPROVE true, build-only by default)"
fi

# Pinned base digest, never latest/bare tag (admissibility gate).
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' .devcontainer/Dockerfile.prebuilt && ! grep -qi -E -e 'FROM [^ ]+:latest([ @]|$)' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost the digest-pinned FROM (never latest)"
fi

# Pre-installed Bazel delegation: Bazelisk fetch + sha256 check +
# Bazel 9.2.0 via USE_BAZEL_VERSION.
if grep -q -F -e 'bazelisk-linux-amd64' .devcontainer/Dockerfile.prebuilt && grep -q -F -e 'sha256sum -c' .devcontainer/Dockerfile.prebuilt && grep -q -F -e 'USE_BAZEL_VERSION=9.2.0' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost Bazelisk delegation (fetch + sha check + 9.2.0)"
fi

# No ambient language toolchains: tools resolve via Bazel at runtime.
if ! grep -qi -E -e 'pip install|uv pip|npm install -g|pnpm add -g|cargo install|dotnet tool install|go install' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt bakes in a language toolchain (must resolve via Bazel)"
fi

# No docker/* or sigstore/* installer actions: plain `docker build`/`push`
# plus a pinned `curl` cosign fetch keeps the push gate explicit in `run:`
# steps; the sole third-party action (checkout) stays pinned to a commit
# SHA per.
if ! grep -q -F -e 'uses: docker/' .github/workflows/ghcr.yml && ! grep -q -F -e 'uses: sigstore/' .github/workflows/ghcr.yml && grep -q -E -e 'uses: actions/checkout@[0-9a-f]{40}' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml gained a docker/* or sigstore/* action or lost the checkout SHA pin (plain build/push + curl cosign only)"
fi

# No-secrets checkout plus non-cancelling concurrency: persist-credentials
# false keeps the token out of the build, cancel-in-progress false queues
# overlapping dispatches instead of cancelling the gated push.
if grep -q -F -e 'persist-credentials: false' .github/workflows/ghcr.yml && grep -q -F -e 'cancel-in-progress: false' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost no-secrets checkout or non-cancelling concurrency"
fi

# Signing-second selected explicitly per: cosign <digest> on the
# trust root (same as //deploy/release:signing_demo), dry-run
# would-sign without approval.
if grep -q -F -e 'cosign sign <digest>' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the cosign-sign record on the #311 trust root"
fi

# Gated push signs + verifies for real (not echo-only): cosign sign --yes
# plus cosign verify and gh attestation on the same trust root.
if grep -q -F -e 'cosign-bin sign --yes' .github/workflows/ghcr.yml && grep -q -F -e 'cosign-bin verify' .github/workflows/ghcr.yml && grep -q -F -e 'gh attestation' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the gated cosign sign --yes / verify / attestation (echo-only signs nothing)"
fi

# Pinned cosign fetch (no installer action): version-pinned curl plus
# checksum verification against the published release checksums.
if grep -q -F -e 'COSIGN_VERSION=' .github/workflows/ghcr.yml && grep -q -F -e 'cosign_checksums' .github/workflows/ghcr.yml && grep -q -F -e 'sha256sum -c' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the pinned cosign fetch (version + checksums + sha256sum -c)"
fi

# Single-version tag tracking: push and PR tags carry dx ==
# module (0.0.0) plus sha; digest pin (never latest) is the scaffold ref.
if grep -q -F -e 'devcontainer:0.0.0-sha-' .github/workflows/ghcr.yml && grep -q -F -e 'devcontainer:0.0.0-ci-' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost single-version tag tracking (want 0.0.0-sha- plus 0.0.0-ci-)"
fi

# Keyless OIDC scope (Sigstore keyless needs id-token): only the build job
# carries `id-token: write`; top-level stays read-only like packages.
if grep -q -F -e 'id-token: write' .github/workflows/ghcr.yml && ! grep -q -E -e '^  id-token: write' .github/workflows/ghcr.yml && grep -q -E -e '^      id-token: write' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost job-scoped id-token:write (keyless needs it; top-level must stay read-only)"
fi

# Signing trust root shared with releases per.
if grep -q -F -e '#311 trust root' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the #311 trust-root record (shared with //deploy/release:signing_demo)"
fi

# Scaffold digest ref + qualified quota (never claimed build-only without
# evidence; build-only PRs consume no quota, gated push records bytes).
if grep -q -F -e 'scaffold' .github/workflows/ghcr.yml && grep -q -F -i -e 'quota' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the scaffold-digest/quota first-push record"
fi

# Quota qualified in docs (not assumed): container free for public with
# 1-month notice; private-Packages quotas do not apply to containers.
if grep -q -F -e 'currently free for public' docs/contributing/devcontainer.md && grep -q -F -e 'one month notice' docs/contributing/devcontainer.md && grep -q -F -e 'currently free for public' docs/testing/README.md; then
  ok
else
  bad "quota record lost its qualification (want currently-free + 1-month notice in devcontainer.md + testing README)"
fi

# Scaffold still on mcr until the first push lands a digest (switching
# the image to ghcr.io...@sha256: now would invent an unpublished
# digest; the ghcr.io feature reference below is the upstream Bazel
# feature, not our image).
if grep -q -F -e '"image": "mcr.microsoft.com/devcontainers/base' .devcontainer/devcontainer.json && ! grep -q -F -e '"image": "ghcr.io' .devcontainer/devcontainer.json; then
  ok
else
  bad "devcontainer.json image drifted from mcr before the first GHCR push (digest ref follows push)"
fi

# Least-privilege split explicit (parity with): only
# ghcr.yml carries `packages: write` for image push; the dry run stays
# read-only. Losing the push permission breaks gated publication, while
# gaining it in publish-dry-run.yml is rejected there.
if grep -q -F -e 'packages: write' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost packages:write (gated push needs it; dry run must stay read-only)"
fi

# Least-privilege default: top-level permissions stay
# read-only (`contents: read`); only the build job carries
# `packages: write` for the gated push, so a future job without
# explicit permissions never inherits push scope.
if ! grep -q -E -e '^  packages: write' .github/workflows/ghcr.yml && grep -q -E -e '^      packages: write' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml top-level permissions gained push scope (packages:write belongs on the build job only)"
fi

dx_test_summary "ghcr hygiene harness"
