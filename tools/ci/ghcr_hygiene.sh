#!/usr/bin/env bash
# GHCR prebuilt-image hygiene (issue #184, seed slice).
#
# The prebuilt devcontainer image removes per-create feature-install cost,
# but publication is gated: separate workflow from releases per owner
# decision, build on PR, push only on workflow_dispatch + approve:true,
# signing-second after #26/#78 (cosign <digest> on the #26 trust root).
# Scaffold `image:` digest reference + quota/retention record follow the
# first push; this build-only slice pushes nothing.
#
# This harness machine-checks the gate half verifiable on a clean tree
# today (16 checks): separate ghcr.yml route, PR-paths build, dispatch +
# default-closed approve gate, typed approve, push run-gate explicit, no push/tag/schedule trigger, digest-pinned
# base + Bazelisk delegation + no ambient toolchains in Dockerfile.prebuilt,
# no docker/* actions with checkout SHA-pinned, no-secrets checkout plus
# non-cancelling concurrency, cosign/quota/scaffold
# deferrals named, scaffold still on mcr (switch follows first push). First-push
# signing + quota record stay open per #184 and are recorded as gaps, not
# claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:ghcr_hygiene`,
# following //tools/ci:examples_laziness_aquery.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

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

# The approve gate stays typed (issue #184 parity with #78): boolean
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

# No docker/* actions: plain `docker build`/`push` keeps the push gate
# explicit in `run:` steps; the sole third-party action (checkout) stays
# pinned to a commit SHA per #80.
if ! grep -q -F -e 'uses: docker/' .github/workflows/ghcr.yml && grep -q -E -e 'uses: actions/checkout@[0-9a-f]{40}' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml gained a docker/* action or lost the checkout SHA pin (plain build/push only)"
fi

# No-secrets checkout plus non-cancelling concurrency: persist-credentials
# false keeps the token out of the build, cancel-in-progress false queues
# overlapping dispatches instead of cancelling the gated push.
if grep -q -F -e 'persist-credentials: false' .github/workflows/ghcr.yml && grep -q -F -e 'cancel-in-progress: false' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost no-secrets checkout or non-cancelling concurrency"
fi

# Signing-second deferred explicitly: cosign <digest> on the #26 trust
# root after #26/#78, never claimed here.
if grep -q -F -e 'cosign sign <digest>' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the cosign-sign deferral to the #26 trust root"
fi

# Scaffold digest ref + quota record deferred to first push (never
# claimed build-only; no quota consumed here).
if grep -q -F -e 'scaffold' .github/workflows/ghcr.yml && grep -q -F -i -e 'quota' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost the scaffold-digest/quota first-push deferral"
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

# Least-privilege split explicit (issue #184 parity with #78): only
# ghcr.yml carries `packages: write` for image push; the dry run stays
# read-only. Losing the push permission breaks gated publication, while
# gaining it in publish-dry-run.yml is rejected there.
if grep -q -F -e 'packages: write' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml lost packages:write (gated push needs it; dry run must stay read-only)"
fi

# Least-privilege default (issue #184): top-level permissions stay
# read-only (`contents: read`); only the build job carries
# `packages: write` for the gated push, so a future job without
# explicit permissions never inherits push scope.
if ! grep -q -E -e '^  packages: write' .github/workflows/ghcr.yml && grep -q -E -e '^      packages: write' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml top-level permissions gained push scope (packages:write belongs on the build job only)"
fi

echo "ghcr hygiene harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
