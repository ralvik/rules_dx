#!/usr/bin/env bash
# Signing program for `signed_release` (issue #459, live successor to
# closed #311/#26 for the signing stack).
#
# Invoked via `bazel run :<name>` with pinned artifact paths as `$@`.
# Selected stack (issue #459 decision, no stack change): Sigstore keyless
# (`cosign sign-blob --bundle` v2.4.1 pinned per
# `deploy/release/signing.bzl` SIGNING_COSIGN_VERSION, Fulcio OIDC + Rekor
# public-good on the TUF trust root) plus GitHub Artifact Attestations
# (`gh attestation create`). Bundle media type
# `application/vnd.dev.sigstore.bundle.v0.3+json` (0.1/0.2 only if
# declared). Host tools resolved at run time; no new module dependencies.
#
# Safety (issue #5): never runs on CI push/PR. With
# `RELEASE_SIGN_DRY_RUN=1` prints the would-run commands and publishes
# nothing (what CI exercises). Real signing needs the tag pushed
# beforehand, explicit owner approval, and OIDC identity per
# docs/deploy/release-runbook.md; without `cosign`/`gh` it fails closed.
set -euo pipefail

trust_root="https://tuf-repo-cdn.sigstore.dev"
identity="${SIGNING_IDENTITY:-}"
issuer="${SIGNING_ISSUER:-https://token.actions.githubusercontent.com}"

if [[ "$#" -eq 0 ]]; then
  echo "signing: need at least one artifact" >&2
  exit 1
fi
if [[ -z "$identity" ]]; then
  echo "signing: missing SIGNING_IDENTITY (owner-approved release workflow identity)" >&2
  exit 1
fi

if [[ "${RELEASE_SIGN_DRY_RUN:-}" == "1" ]]; then
  echo "signing: dry run (RELEASE_SIGN_DRY_RUN=1); would sign, publishing nothing:"
  echo "  trust root: ${trust_root}"
  echo "  identity: ${identity}"
  echo "  issuer: ${issuer}"
  echo "  cosign: v2.4.1 (pinned per deploy/release/signing.bzl SIGNING_COSIGN_VERSION; checksum-verified fetch per .github/workflows/ghcr.yml)"
  echo "  bundle media type: application/vnd.dev.sigstore.bundle.v0.3+json (Sigstore bundle v0.3; 0.1/0.2 only if declared)"
  for asset in "$@"; do
    echo "  asset: $(basename "${asset}") (${asset})"
    echo "  command: cosign sign-blob --bundle $(basename "${asset}").bundle --certificate-identity ${identity} --certificate-issuer ${issuer} ${asset}"
    echo "  command: gh attestation create ${asset} --bundle $(basename "${asset}").bundle"
  done
  printf '  verify: cosign verify-blob --bundle <bundle> --certificate-identity %s --certificate-issuer %s <binary>\n' "${identity}" "${issuer}"
  exit 0
fi

if ! command -v cosign >/dev/null 2>&1; then
  echo "signing: 'cosign' CLI not found on PATH; install it to sign releases (owner-approved human-run path only)" >&2
  exit 1
fi

for asset in "$@"; do
  bundle="${asset}.bundle"
  echo "signing: cosign sign-blob ${asset} -> ${bundle} (identity=${identity} issuer=${issuer} trust=${trust_root})"
  cosign sign-blob --bundle "${bundle}" --certificate-identity "${identity}" --certificate-issuer "${issuer}" "${asset}"
  if command -v gh >/dev/null 2>&1; then
    echo "signing: gh attestation create ${asset}"
    gh attestation create "${asset}" --bundle "${bundle}" || {
      echo "signing: gh attestation failed for ${asset} (bundle still produced)" >&2
      exit 1
    }
  else
    echo "signing: 'gh' not found; bundle produced, attestation skipped (verify bundle via dx_verify)" >&2
  fi
done
