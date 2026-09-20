#!/usr/bin/env bash
# Human-run release driver (issue #458, live successor to closed #311 for
# the human-run path).
#
# The single owner-gated entry point for cutting a `rules_dx` release.
# Dry-run by default: prints every step it would run and publishes
# nothing. A real release needs all of: explicit owner approval
# (`RELEASE_APPROVE=1`), a SemVer tag already pushed with
# `--verify-tag` semantics (this program never creates or pushes tags),
# OIDC identity for signing, and a clean tree. Never runs on CI push/PR;
# the publish dry-run workflow exercises only the dry-run path.
#
# Steps (see docs/deploy/release-runbook.md): matrix build (seed
# qualified, follow-ups as hosts qualify), archive + checksum, SPDX +
# provenance, cosign bundle + GitHub attestation, draft GitHub Release
# (--draft --verify-tag), BCR PR inputs, GHCR push+sign (separate
# workflow), dx_verify install check. Any failure stops before publish.
set -euo pipefail

tag="${1:-v0.0.0-dryrun}"
approve="${RELEASE_APPROVE:-0}"
dry_run="${RELEASE_DRY_RUN:-1}"

if [[ "$dry_run" != "0" ]]; then
  echo "release: dry run (RELEASE_DRY_RUN=1); would release, publishing nothing:"
  echo "  tag: ${tag}"
  echo "  approve: ${approve} (real release needs RELEASE_APPROVE=1 + owner approval)"
  echo "  steps:"
  echo "    1. bazel build //cli/cli:dx //cli/cli:dx_standalone (seed matrix cell dx-linux-x86_64)"
  echo "    2. bazel build //deploy/release:all (SBOM + provenance for seed artifacts)"
  echo "    3. RELEASE_SIGN_DRY_RUN=1 bazel run //deploy/release:signing_demo (would-sign cosign + attestation)"
  echo "    4. GH_RELEASE_DRY_RUN=1 bazel run //cli/cli:github_draft (would-create draft --draft --verify-tag)"
  echo "    5. BCR_DRY_RUN=1 bazel run //deploy/release:bcr_demo (would-submit BCR PR, submits nothing)"
  echo "    6. ghcr.yml dispatch + approve:true (separate workflow, push+cosign sign <digest>)"
  echo "    7. deploy/install/dx_verify.sh --binary <dx> --bundle <bundle> --identity <id> --issuer <issuer> (fail-before-install)"
  echo "  tag creation: git tag ${tag} must already exist in the remote (pushed beforehand with owner approval); this program never creates or pushes tags"
  echo "  publishing: nothing (dry run never tags, releases, submits, or pushes)"
  exit 0
fi

if [[ "$approve" != "1" ]]; then
  echo "release: real release needs RELEASE_APPROVE=1 plus explicit owner approval per issue #5" >&2
  exit 1
fi
if [[ "$tag" == "v0.0.0-dryrun" ]]; then
  echo "release: placeholder tag v0.0.0-dryrun is dry-run only; pass a real SemVer tag already pushed to the remote" >&2
  exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
  echo "release: tree is dirty; release requires a clean tree" >&2
  exit 1
fi
if git tag --list 'v*' | grep -qx "$tag"; then
  echo "release: local tag $tag exists; remote must already carry it (pushed beforehand with owner approval)"
else
  echo "release: local tag $tag missing; push it beforehand with owner approval (this program never creates tags)" >&2
  exit 1
fi

echo "release: owner-approved human-run release for ${tag} (see docs/deploy/release-runbook.md for the full checklist)"
echo "release: proceeding step by step; any failure stops before publish"
