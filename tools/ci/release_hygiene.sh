#!/usr/bin/env bash
# Release-hygiene harness (issue #5).
#
# No release has been cut. Tags, GitHub releases, registry submissions, and
# publication outputs require explicit owner approval (see CONTRIBUTING.md):
# `dist/` and `release/` are git-ignored build outputs and must never be
# committed, `MODULE.bazel` keeps `version = "0.0.0"` as a module version
# string only, and the publish dry-run stays `workflow_dispatch`-only so CI
# stays green by construction and nothing runs uninvited.
#
# This harness machine-checks the no-publication-inputs half that is
# verifiable on a clean tree today (17 checks): dist/release
# git-ignored and uncommitted, module at 0.0.0, no version tags,
# SECURITY.md reporting link, publish dry-run dispatch-only with a
# default-closed approve gate, no-secrets minimal permissions,
# RUNNER_TEMP staging plus a clean-checkout proof, explicit release
# matrix (seed qualified, rest unqualified per #5), SBOM/BCR
# deferrals to #26 tooling, signing-first + GHCR-separate notes,
# checkout SHA pin, typed approve plus non-cancelling concurrency, and
# least-privilege no-packages-write. Platform,
# packaging, provenance (SPDX/SLSA), registry submission, and
# public-install smoke runs stay unqualified per #5 and are recorded
# as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_hygiene`,
# following //tools/ci:corpus_audit.
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

# dist/ and release/ are git-ignored build outputs (never commit).
if grep -q -F -e '/dist/' .gitignore && grep -q -F -e '/release/' .gitignore; then
  ok
else
  bad ".gitignore lost the /dist/ + /release/ entries"
fi

# No dist/ or release/ bytes are committed (rebuilt locally on demand).
if [[ -z "$(git ls-files | grep -E '^(dist|release)/' || true)" ]]; then
  ok
else
  bad "dist/ or release/ paths are committed: $(git ls-files | grep -E '^(dist|release)/' | head -n 5)"
fi

# The module stays at 0.0.0; consumers pin reviewed commits, never tags.
if grep -q -E -e '^module\(|version = "0\.0\.0"' MODULE.bazel && ! grep -q -E -e 'version = "0\.1\.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted from version 0.0.0"
fi

# No version tags without explicit owner approval (the unapproved v0.1.0
# candidate was removed; remote stays tag-free, verified via ls-remote).
if [[ -z "$(git tag --list 'v*' || true)" ]]; then
  ok
else
  bad "unapproved version tag present: $(git tag --list 'v*' | head -n 5)"
fi

# SECURITY.md private-vulnerability-report link points at this repo, not
# the upstream-copy leftover (small verification slice, issue #5).
if grep -q -F -e 'github.com/ralvik/rules_dx/security/advisories/new' SECURITY.md && ! grep -q -F -e 'TrapsterDK/rules_dx/security/advisories' SECURITY.md; then
  ok
else
  bad "SECURITY.md reporting link drifted from ralvik/rules_dx"
fi

# The publish dry-run never publishes uninvited: workflow_dispatch only,
# no push/pull_request/tag/schedule trigger.
if grep -q -F -e 'workflow_dispatch:' .github/workflows/publish-dry-run.yml && ! grep -q -E -e '^  (push|pull_request|schedule):' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml gained a non-dispatch trigger"
fi

# The dry-run approval gate stays explicit and default-closed (issue
# #78): an `approve` input defaulting to false, with nothing publishing
# either way.
if grep -q -F -e 'approve:' .github/workflows/publish-dry-run.yml && grep -q -F -e 'default: false' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the default-closed approve gate"
fi

# The dry-run stores no secrets and keeps minimal permissions (issue
# #78): checkout with persist-credentials false, contents read-only.
if grep -q -F -e 'persist-credentials: false' .github/workflows/publish-dry-run.yml && grep -q -F -e 'contents: read' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml drifted from no-secrets minimal permissions"
fi

# The dry-run never dirties the checkout (issue #78): staging under
# RUNNER_TEMP plus a clean-checkout proof step.
if grep -q -F -e 'RUNNER_TEMP/publish-dry-run' .github/workflows/publish-dry-run.yml && grep -q -F -e 'test -z "$(git status --porcelain)"' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost RUNNER_TEMP staging or the clean-checkout proof"
fi

# The release-hygiene policy itself stays documented (prevents silent
# deletion of the gate this harness enforces).
if grep -q -F -e 'No tags, GitHub releases' CONTRIBUTING.md; then
  ok
else
  bad "CONTRIBUTING.md lost the no-tags/no-releases policy"
fi

# The dry-run report names the full release matrix explicitly (issue
# #78): seed linux-x86_64 qualified-built-here, the other four
# (linux-arm64, macos-x86_64/arm64, windows-x86_64) unqualified per #5.
if grep -q -F -e 'dx-linux-arm64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-macos-arm64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-windows-x86_64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'unqualified-per-issue-5' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the explicit release-matrix qualification table"
fi

# SBOM/provenance and BCR submission stay explicitly deferred to #26
# tooling (issue #78 signing-first): the dry run must name the gap,
# never claim the tooling.
if grep -q -F -e 'SBOM/provenance generation (tooling unselected; issue #26)' .github/workflows/publish-dry-run.yml && grep -q -F -e 'BCR dry-run submission (registry tooling unselected; issue #26)' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the SBOM/BCR deferral to issue #26"
fi

# Signing-first order stays explicit (issue #78/#26): Sigstore keyless +
# GitHub attestations on the #26 trust root, GHCR signs separately via
# cosign <digest> under #184 — never claimed here, only deferred.
if grep -q -F -e 'Signing/attestation (Sigstore keyless + GitHub attestations on the issue #26 trust root' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the signing-first deferral to issue #26"
fi

# GHCR stays a separate workflow (owner decision, issue #184): the dry
# run must name the separate ghcr.yml route, never fold images here.
if grep -q -F -e 'GHCR prebuilt images (separate workflow .github/workflows/ghcr.yml' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the GHCR-separate note (issue #184)"
fi

# Third-party actions stay SHA-pinned (issue #80): the sole third-party
# action (checkout) must pin to a commit SHA, never float a tag.
if grep -q -E -e 'uses: actions/checkout@[0-9a-f]{40}' .github/workflows/publish-dry-run.yml && ! grep -q -F -e 'uses: docker/' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the checkout SHA pin or gained a docker/* action"
fi

# The approve gate stays typed and non-cancelling (issue #78): boolean
# input type plus concurrency cancel-in-progress false so overlapping
# dispatches queue instead of cancelling the qualification run.
if grep -q -F -e 'type: boolean' .github/workflows/publish-dry-run.yml && grep -q -F -e 'cancel-in-progress: false' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the typed approve gate or non-cancelling concurrency"
fi

# The dry-run stays least-privilege (issue #78): no packages:write
# (only ghcr.yml needs packages:write for image push; the dry run
# never publishes, so contents:read is sufficient).
if ! grep -q -F -e 'packages: write' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml gained packages:write (dry run must stay read-only; push lives in ghcr.yml)"
fi

echo "release hygiene harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
