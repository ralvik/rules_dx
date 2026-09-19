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
# verifiable on a clean tree today (34 checks): dist/release
# git-ignored and uncommitted, module at 0.0.0, no version tags,
# SECURITY.md reporting link + enabled record, publish dry-run dispatch-only with a
# default-closed approve gate, no-secrets minimal permissions plus no
# secrets usage, RUNNER_TEMP staging plus a clean-checkout proof, explicit release
# matrix (seed qualified, rest unqualified per #5), SBOM/BCR
# deferrals to #26 tooling, signing-first + GHCR-separate notes,
# checkout SHA pin, typed approve plus non-cancelling concurrency,
# least-privilege no-packages-write, no-secrets usage, and sole-tracker deletion plus
# reporting-enabled record plus consumer/docs-caller SHA pins plus
# both-callers policy plus no-tag/no-release/no-submission record plus
# never-rebuild policy plus byte-identity fail-closed record plus seed
# exercised path (standalone archive, draft dry-run, BCR shape check,
# verifier refusal). Platform, provenance (SPDX/SLSA), registry submission, and
# public-install smoke runs stay unqualified per #5 and are recorded
# as gaps, not claimed here.
#
# Versioned here, run by CI via `bazel run //tools/ci:release_hygiene`,
# following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
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

# The dry-run report names the full release matrix explicitly (issues
# #78/#311): seed linux-x86_64 qualified-built-here, the other four
# (linux-arm64, macos-x86_64/arm64, windows-x86_64) unqualified per #311
# (frozen in deploy/release/matrix.bzl).
if grep -q -F -e 'dx-linux-arm64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-macos-arm64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-windows-x86_64' .github/workflows/publish-dry-run.yml && grep -q -F -e 'unqualified-per-issue-311' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the explicit release-matrix qualification table"
fi

# SBOM/provenance and BCR submission run owner-gated dry-run-first per
# #311: the dry run exercises //deploy/release:sbom_demo and
# //deploy/release:bcr_demo in dry-run mode, publishing nothing.
if grep -q -F -e '//deploy/release:sbom_demo' .github/workflows/publish-dry-run.yml && grep -q -F -e 'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the SBOM/BCR owner-gated exercise (issue #311)"
fi

# Signing-first order stays explicit (issues #78/#311): Sigstore keyless +
# GitHub attestations on the #311 trust root, GHCR signs separately via
# cosign <digest> under #184 — dry-run here, never publishing.
if grep -q -F -e 'Signing/attestation publishing (Sigstore keyless + GitHub attestations on the issue #311 trust root' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the signing-first owner-gated record (issue #311)"
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

# The dry-run uses no secrets at all (issue #78): no `secrets.`
# reference (the gated GHCR push alone uses GITHUB_TOKEN under #184;
# the dry run only builds locally and reports, so any secret reference
# would be an unreviewed publication input).
if ! grep -q -F -e 'secrets.' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml gained a secrets reference (dry run must use no secrets)"
fi

# Sole tracker stays sole (issue #5): the local `issues/` mirror and
# `archive/delivery-v0.1/` were deleted; delivery history lives in git
# history. Neither may reappear on disk nor committed.
if [[ ! -e "issues" ]] && [[ ! -e "archive/delivery-v0.1" ]] && [[ -z "$(git ls-files | grep -E '^(issues|archive/delivery-v0\.1)/' || true)" ]]; then
  ok
else
  bad "issues/ mirror or archive/delivery-v0.1/ reappeared (sole tracker is issue #5)"
fi

# SECURITY.md reporting stays enabled (issue #5 precondition for any
# public release): the static half records that private vulnerability
# reporting is enabled; live API verification remains a manual gap
# recorded in the issue, never claimed here.
if grep -q -F -e 'Private vulnerability reporting is enabled' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost the private-reporting-enabled record (precondition per issue #5)"
fi

# Consumer-CI caller template pins the reusable workflow at a reviewed
# commit SHA (issue #5 context): never a tag or floating ref, so
# consumers track qualified commits while MODULE stays at 0.0.0.
if grep -q -E -e 'reusable-consumer\.yml@[0-9a-f]{40}' examples/consumer-ci/caller.yml; then
  ok
else
  bad "examples/consumer-ci/caller.yml lost its reviewed-commit SHA pin (issue #5)"
fi

# Docs-CI caller template pins the reusable docs workflow at a reviewed
# commit SHA (issue #5 context, CONTRIBUTING.md both-callers policy):
# never a tag or floating ref, so docs consumers track the same
# qualified commit as consumer-ci (sync enforced by
# //tools/ci:examples_pins_test; this guard keeps the release-hygiene
# track self-contained).
if grep -q -E -e 'reusable-docs\.yml@[0-9a-f]{40}' examples/docs-ci/caller.yml; then
  ok
else
  bad "examples/docs-ci/caller.yml lost its reviewed-commit SHA pin (issue #5)"
fi

# CONTRIBUTING both-callers pin policy stays documented (issue #5
# context): caller pins stay frozen at the last qualified commit, stay
# in sync across both example callers, and move only together — so the
# SHA-pin guards above cannot be silently redefined as floating tags.
if grep -q -F -e 'pin reviewed commits, never release tags' CONTRIBUTING.md && grep -q -F -e 'stay in sync across both example callers' CONTRIBUTING.md; then
  ok
else
  bad "CONTRIBUTING.md lost the both-callers pin policy (issue #5)"
fi

# The dry-run report stays explicit that it creates nothing (issue #5
# policy: no tags, GitHub releases, or registry submissions without
# explicit owner approval): the not_attempted list must name that any
# tag, registry submission, or release creation is out of scope, so the
# nothing-publishes ceiling cannot be silently narrowed to only the
# tooling deferrals above.
if grep -q -F -e 'Any tag, registry submission, or release creation' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the no-tag/no-release/no-submission record (issue #5)"
fi

# Published bytes are never rebuilt or substituted silently (issue #5
# next-steps): the policy stays recorded in CONTRIBUTING.md plus
# CHANGELOG.md, so the approval gate cannot be read as allowing a quiet
# byte swap after approval.
if grep -q -F -e 'never' CONTRIBUTING.md && grep -q -F -e 'rebuilt or substituted silently' CONTRIBUTING.md && grep -q -F -e 'rebuilt or substituted silently' CHANGELOG.md; then
  ok
else
  bad "CONTRIBUTING.md/CHANGELOG.md lost the never-rebuild-or-substitute record (issue #5)"
fi

# Byte identity stays fail-closed (issue #5 never-rebuild mechanism):
# the artifact generator rejects changed upstream bytes instead of
# silently recording new content, and the dry run records the seed
# binary sha256 digest, so a substituted byte cannot pass as the same
# release.
if grep -q -F -e 'instead of silently recording new content' quality/artifacts/update.py && grep -q -F -e 'does not match published' quality/artifacts/update.py && grep -q -F -e 'sha256' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "byte-identity fail-closed record lost (update.py + dry-run sha256, issue #5)"
fi

# Seed standalone packaging stays exercised (issue #78 seed-qualified):
# the dry run builds //cli/cli:dx_standalone and stages the tarball plus
# checksum under RUNNER_TEMP, never committed; the wider matrix is frozen
# in deploy/release/matrix.bzl, unqualified per #311.
if grep -q -F -e '//cli/cli:dx_standalone' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-standalone.tar.gz' .github/workflows/publish-dry-run.yml && grep -q -F -e 'dx-standalone.tar.gz.sha256' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the seed standalone exercise (dx_standalone + tarball + checksum, issue #78)"
fi

# Draft creation stays exercised without publishing (issue #78 draft-only):
# the dry run runs //cli/cli:github_draft with GH_RELEASE_DRY_RUN=1 and
# proves the placeholder plus draft-only flags, publishing nothing.
if grep -q -F -e 'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' .github/workflows/publish-dry-run.yml && grep -q -F -e 'v0.0.0-dryrun' .github/workflows/publish-dry-run.yml && grep -q -F -e '--draft --verify-tag' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the draft dry-run exercise (github_draft + GH_RELEASE_DRY_RUN=1 + draft-only flags, issue #78)"
fi

# BCR shape stays checked-not-submitted (issue #311 owner-gated):
# the dry run runs //deploy/release:bcr_demo in BCR_DRY_RUN=1 mode and
# records checked-not-submitted without submitting.
if grep -q -F -e 'bcr-shape.txt' .github/workflows/publish-dry-run.yml && grep -q -F -e 'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo' .github/workflows/publish-dry-run.yml && grep -q -F -e '"submitted": False' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the BCR owner-gated record (bcr_demo + BCR_DRY_RUN=1 + submitted False, issue #311)"
fi

# Install-verifier refusal stays proved (issue #78 signing-first):
# the dry run proves //deploy/install:dx_verify refuses checksum-only
# inputs on the TUF trust root and installs nothing, without network.
if grep -q -F -e 'dx_verify.sh --help' .github/workflows/publish-dry-run.yml && grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml && grep -q -F -e 'checksum-only verification is not publisher-identity proof' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the verifier-refusal exercise (dx_verify checksum-only refused, issue #78)"
fi

# Release tests stay exercised (issue #311): the dry run runs
# //deploy/release:all green, proving matrix + SBOM + signing + BCR +
# human-run gates without publishing.
if grep -q -F -e 'bazel test //deploy/release:all' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the release-tests exercise (//deploy/release:all, issue #311)"
fi

# Signing dry-run stays exercised (issue #311 signing-first): the dry run
# runs //deploy/release:signing_demo with RELEASE_SIGN_DRY_RUN=1 and
# proves the trust root plus would-sign, publishing nothing.
if grep -q -F -e 'RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo' .github/workflows/publish-dry-run.yml && grep -q -F -e 'tuf-repo-cdn.sigstore.dev' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the signing dry-run exercise (signing_demo + RELEASE_SIGN_DRY_RUN=1, issue #311)"
fi

# Human-run driver stays exercised (issue #311): the dry run runs
# deploy/release/release.sh in dry-run mode, proving the tag ceiling
# plus owner-approval gate with nothing published.
if grep -q -F -e 'deploy/release/release.sh' .github/workflows/publish-dry-run.yml && grep -q -F -e 'human-run-dry-run.log' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish-dry-run.yml lost the human-run driver exercise (release.sh dry run, issue #311)"
fi

# Release runbook stays owned (issue #311): the human-run path is
# documented, not just workflow steps.
if [[ -f "docs/deploy/release-runbook.md" ]] \
  && grep -q -F -e 'never creates or pushes tags' docs/deploy/release-runbook.md; then
  ok
else
  bad "release runbook missing (docs/deploy/release-runbook.md + tag ceiling, issue #311)"
fi

echo "release hygiene harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
