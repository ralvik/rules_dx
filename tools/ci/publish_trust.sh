#!/usr/bin/env bash
# Publication-trust harness: machine-checks
# the draft-only + nothing-published invariants that the release-hygiene
# harness does not own.
#
# The publish dry-run stays `workflow_dispatch`-only with a
# default-closed approve gate (proven by //tools/ci:release_hygiene),
# and every `github_release` site stays draft-only by construction:
# `draft` defaults to True and any `draft = False` fails analysis in
# `deploy/rules/github.bzl`, tags validate against the launcher-safe
# charset with the `v0.0.0-dryrun` placeholder default, and the deploy
# program always passes `--draft --verify-tag` so it never creates or
# pushes tags. BCR runs owner-gated dry-run-first per 
# (`BCR_DRY_RUN=1` prints would-submit, submits nothing; no
# auto-submit in workflows). The module stays `rules_dx` at `0.0.0`, an
# unpublishable shape, and SECURITY.md still records that no release
# exists.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (22 checks): module name + unpublishable version, no BCR
# auto-submit in workflows, no `draft = False` site, both
# validators wired to `fail()` in the macro, default placeholder tag
# at every site, draft-only flags on the real `gh release create`
# path, no unflagged executable release-create lines, the tag charset
# gate, the SECURITY.md no-release record, plus install-time
# publisher-identity verification
# //deploy/install:dx_verify: bundle-required, no checksum-only
# fallback, TUF trust root, fail-before-install) plus seed exercised
# path (standalone wired, draft dry-run with GH_RELEASE_DRY_RUN=1 still
# publishing nothing, BCR shape checked-not-submitted, verifier refusal
# proved in the workflow) plus the full release path (matrix
# frozen with seed qualified, SBOM SPDX-2.3 + SLSA v1 wired, signing
# Sigstore keyless + attestation selected with dry-run gate, BCR
# owner-gated with dry-run gate, human-run driver with tag ceiling,
# workflow exercises release tests + signing/bcr/human-run dry-runs
# with published/submitted False everywhere).
#
# Versioned here, run by CI via `bazel run //tools/ci:publish_trust`,
# following //tools/ci:release_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# The module keeps its public name at the unpublishable 0.0.0 version:
# consumers pin reviewed commits, never tags or releases.
if grep -q -F -e 'name = "rules_dx"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost the rules_dx module name"
fi
if grep -q -F -e 'version = "0.0.0"' MODULE.bazel && ! grep -q -F -e 'version = "0.1.0"' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel drifted from the unpublishable version 0.0.0"
fi

# BCR runs owner-gated dry-run-first per: no auto-submit tooling
# runs in any workflow (no publish-to-bcr, no BCR_APPROVE=1); the
# dry-run prints would-submit via //deploy/release:bcr_demo.
if grep -rn -E -e 'publish-to-bcr|bcr publish|bazel-central-registry' .github/ | grep -v -E -e '^\s*#' | head -n 5 | grep -q .; then
  bad "a workflow executes BCR auto-submit tooling (owner-gated dry-run only per #311)"
else
  ok
fi
if grep -rn -F -e 'BCR_APPROVE=1' .github/ | head -n 5 | grep -q .; then
  bad "a workflow carries BCR_APPROVE=1 (submission is human-run only per #311)"
else
  ok
fi

# No github_release site opts out of the draft gate: draft defaults to
# True and any explicit False fails analysis, so the tree must carry
# no `draft = False` at all.
if grep -rn -F -e 'draft = False' --include='BUILD.bazel' . | head -n 5 | grep -q .; then
  bad "a github_release site sets draft = False (draft-only per #5)"
else
  ok
fi

# The macro wires both validators to analysis failure: a bad tag or a
# non-draft request fails the build instead of publishing.
if grep -q -F -e 'fail(tag_error' deploy/rules/github.bzl && grep -q -F -e 'fail(draft_error' deploy/rules/github.bzl; then
  ok
else
  bad "deploy/rules/github.bzl no longer fails analysis on tag/draft violations"
fi

# Every github_release site uses the default placeholder tag: no
# explicit `tag =` override may point a checked-in target at a real
# release tag.
if grep -rn -A8 -F -e 'github_release(' --include='BUILD.bazel' . | grep -F -e 'tag =' | head -n 5 | grep -q .; then
  bad "a github_release site overrides the v0.0.0-dryrun placeholder tag"
else
  ok
fi

# The real publisher path always passes --draft --verify-tag, so the
# program never creates or pushes tags itself.
if grep -q -F -e 'exec gh release create "${tag}" "$@" --draft --verify-tag' deploy/rules/github_deploy.sh; then
  ok
else
  bad "deploy/rules/github_deploy.sh real path lost the --draft --verify-tag flags"
fi

# No other executable `gh release create` line exists without the
# draft-only flags (printf dry-run text plus echo/grep verify contexts
# are the only other mentions).
if grep -rn -F -e 'gh release create' --include='*.sh' deploy/ cli/ | grep -v -F -e 'printf' | grep -v -F -e 'echo' | grep -v -F -e 'grep -q' | grep -v -F -e '--draft --verify-tag' | head -n 5 | grep -q .; then
  bad "an executable gh release create line lacks the draft-only flags"
else
  ok
fi

# Release tags embed in the generated launcher, so the charset gate
# keeps them shell-safe.
if grep -q -F -e '_VALID_TAG_CHARS' deploy/rules/github.bzl; then
  ok
else
  bad "deploy/rules/github.bzl lost the launcher-safe tag charset gate"
fi

# SECURITY.md still records that no release exists, so the supported-
# versions section cannot drift into implying support.
if grep -q -F -e 'No release exists yet' SECURITY.md; then
  ok
else
  bad "SECURITY.md lost the no-release-exists record"
fi

# Install-time publisher-identity verification is implemented per:
# the verifier requires a bundle plus identity/issuer, refuses
# checksum-only, and fails before install/exec on the trust root.
if [[ -f "deploy/install/dx_verify.sh" ]] &&
  grep -q -F -e 'checksum-only verification is not publisher-identity proof' deploy/install/dx_verify.sh &&
  grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/install/dx_verify.sh; then
  ok
else
  bad "install verifier missing bundle-required / trust-root record (#26)"
fi

# Seed-host standalone packaging stays wired; the wider matrix is frozen
# in deploy/release/matrix.bzl with seed qualified (no platform claimed
# qualified beyond the seed without host evidence).
if grep -q -F -e 'dx_standalone' cli/cli/BUILD.bazel; then
  ok
else
  bad "seed-host standalone archive missing (//cli/cli:dx_standalone, #26)"
fi

# Seed exercised path stays draft-only: the workflow runs
# //cli/cli:github_draft with GH_RELEASE_DRY_RUN=1 on the placeholder
# tag with draft-only flags, and the report records published False.
if grep -q -F -e 'GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft' .github/workflows/publish-dry-run.yml && grep -q -F -e '"published": False' .github/workflows/publish-dry-run.yml && grep -q -F -e '--draft --verify-tag' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the draft-only exercised record (GH_RELEASE_DRY_RUN=1 + published False, issue #78)"
fi

# BCR shape stays checked-not-submitted: the workflow runs
# //deploy/release:bcr_demo in BCR_DRY_RUN=1 mode and records submitted
# False without auto-submitting.
if grep -q -F -e '"submitted": False' .github/workflows/publish-dry-run.yml && grep -q -F -e 'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the BCR owner-gated record (BCR_DRY_RUN=1 + submitted False, issue #311)"
fi

# Verifier refusal stays exercised in the workflow
# signing-first): checksum-only refused on the TUF trust root with
# nothing installed, without network.
if grep -q -F -e 'verify-refusal.log' .github/workflows/publish-dry-run.yml && grep -q -F -e 'checksum-only verification is not publisher-identity proof' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the verifier-refusal exercised record (issue #78)"
fi

# Full matrix frozen per: five cells, seed qualified, four
# follow-ups unqualified with owner-approval qualification.
if [[ -f "deploy/release/matrix.bzl" ]] &&
  grep -q -F -e 'dx-linux-x86_64' deploy/release/matrix.bzl &&
  grep -q -F -e 'unqualified-per-issue-311' deploy/release/matrix.bzl &&
  grep -q -F -e 'unqualified-per-issue-311' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "release matrix missing frozen five-cell shape (deploy/release/matrix.bzl + workflow, #311)"
fi

# SBOM/provenance selected per: SPDX-2.3 + SLSA v1 wired in the
# macro and exercised in the workflow, publishing nothing.
if grep -q -F -e 'SPDX-2.3' deploy/release/sbom.bzl &&
  grep -q -F -e 'https://slsa.dev/provenance/v1' deploy/release/sbom.bzl &&
  grep -q -F -e '//deploy/release:sbom_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "SBOM/provenance selection missing (SPDX-2.3 + SLSA v1 via //deploy/release:sbom_demo, #311)"
fi

# Signing/attestation selected per: Sigstore keyless + GitHub
# attestations on the TUF trust root, dry-run gate in workflow.
if grep -q -F -e 'tuf-repo-cdn.sigstore.dev' deploy/release/signing.bzl &&
  grep -q -F -e 'RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "signing selection missing (Sigstore keyless + attestation dry-run via //deploy/release:signing_demo, #311)"
fi

# BCR owner-gated tooling per: macro plus dry-run gate in workflow.
if [[ -f "deploy/release/bcr.bzl" ]] &&
  grep -q -F -e 'BCR_DRY_RUN=1' deploy/release/bcr_deploy.sh &&
  grep -q -F -e 'BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "BCR owner-gated tooling missing (deploy/release/bcr.bzl + BCR_DRY_RUN=1, #311)"
fi

# Human-run driver per (live successor to closed for the
# human-run path): dry-run by default, tag ceiling, owner
# approval gate, exercised in the workflow.
if [[ -f "deploy/release/release.sh" ]] &&
  grep -q -F -e 'never creates or pushes tags' deploy/release/release.sh &&
  grep -q -F -e 'deploy/release/release.sh' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "human-run release driver missing (deploy/release/release.sh + workflow exercise, #458)"
fi

# Release tests stay exercised in the workflow: //deploy/release:all
# green with published/submitted False everywhere.
if grep -q -F -e 'bazel test //deploy/release:all' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"submitted": False' .github/workflows/publish-dry-run.yml &&
  grep -q -F -e '"published": False' .github/workflows/publish-dry-run.yml; then
  ok
else
  bad "publish dry run lost the release-tests exercised record (//deploy/release:all + published/submitted False, #311)"
fi

dx_test_summary "publish trust audit"
