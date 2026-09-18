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
# today (11 checks): separate ghcr.yml route, PR-paths build, dispatch +
# default-closed approve gate, no push/tag/schedule trigger, digest-pinned
# base + Bazelisk delegation + no ambient toolchains in Dockerfile.prebuilt,
# no docker/* actions with checkout SHA-pinned, cosign/quota/scaffold
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

# Never push/tag/schedule-triggered: only pull_request (build-only) and
# workflow_dispatch (gated push) may appear as top-level triggers.
if ! grep -q -E -e '^  (push|schedule):' .github/workflows/ghcr.yml && grep -q -E -e '^  pull_request:' .github/workflows/ghcr.yml; then
  ok
else
  bad "ghcr.yml gained a push/schedule trigger or lost pull_request build"
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

echo "ghcr hygiene harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
