#!/usr/bin/env bash
# Publication-group guards.
#
# Prebuilt devcontainer images (live successor to closed) ship from a workflow kept separate
# from releases per owner decision, build on PR, and push only on
# workflow_dispatch + approve:true; signing follows /
# (cosign <digest> on the trust root). The publishing dry run
# is workflow_dispatch-only and publishes nothing either way. Standalone
# dx distribution + install verification and release hygiene
# no tags/releases/submissions without explicit owner approval) stay
# owner-gated with SECURITY reporting as the release precondition.
#
# This harness machine-checks the group half verifiable on a clean tree
# today: workflow separation + triggers + gates, pinned base
# + scaffold state, security precondition, gitignored
# outputs, unpublishable module version, and the no-approval-no-publish
# policy. Matrix/SBOM/BCR-submission/install verification stay open.
#
# Versioned here, run by CI via `bazel run //tools/ci:ghcr_publish_guards`,
# following //tools/ci:ghcr_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Workflows stay separate per owner decision (different lifecycles).
if [[ -f ".github/workflows/ghcr.yml" &&
  -f ".github/workflows/publish-dry-run.yml" ]]; then
  ok
else
  bad "ghcr.yml or publish-dry-run.yml missing (must stay separate)"
fi

# Dry run never auto-publishes: workflow_dispatch only, no push/PR/tag/schedule.
if grep -q -F -e 'workflow_dispatch:' .github/workflows/publish-dry-run.yml &&
  ! grep -q -E -e '^  push:|^  pull_request:|^  schedule:' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml gained an auto trigger or lost dispatch-only"
fi

# Both gated pushes default closed (explicit owner approval required).
if grep -q -F -e 'default: false' .github/workflows/ghcr.yml &&
  grep -q -F -e 'default: false' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "ghcr/dry-run lost the default-closed approval gate"
fi

# Admissibility: digest-pinned base, never latest.
if grep -q -E -e '^FROM [^ ]+@sha256:[0-9a-f]{64}' .devcontainer/Dockerfile.prebuilt &&
  ! grep -qi -E -e 'FROM [^ ]+:latest([ @]|$)' .devcontainer/Dockerfile.prebuilt; then
  ok
else
  bad "Dockerfile.prebuilt lost its digest-pinned base"
fi

# Scaffold still on mcr until the first push lands a digest.
if grep -q -F -e '"image": "mcr.microsoft.com/devcontainers/base' .devcontainer/devcontainer.json &&
  ! grep -q -F -e '"image": "ghcr.io' .devcontainer/devcontainer.json; then
  ok
else
  bad "devcontainer.json image drifted before the first GHCR push"
fi

# Release precondition: private vulnerability reporting enabled.
if grep -q -F -e 'private vulnerability' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost its private-reporting release precondition"
fi

# Publication outputs stay git-ignored build outputs, never committed.
if grep -q -F -e '/dist/' .gitignore &&
  grep -q -F -e '/release/' .gitignore &&
  [[ -z "$(git ls-files dist/ release/ || true)" ]]; then
  ok
else
  bad "dist/release outputs committed or lost gitignore (must stay build outputs)"
fi

# Module stays at the unpublishable version (nothing published).
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted off the unpublishable 0.0.0 version"
fi

# Policy: no tags/releases/submissions/outputs without owner approval.
if grep -q -F -e 'without explicit owner approval' CONTRIBUTING.md; then
  ok
else
  bad "CONTRIBUTING lost its no-publication-without-approval policy"
fi

dx_test_summary "ghcr publish guards harness"
