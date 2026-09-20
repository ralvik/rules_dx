#!/usr/bin/env bash
# Verifies one `signed_release` deploy program (issue #459, live successor
# to closed #311/#26 for the signing stack).
#
# `$1` is the program rootpath. Runs with RELEASE_SIGN_DRY_RUN=1 (no
# network, no mutation) and asserts the trust root, identity, issuer,
# pinned cosign version plus bundle media type, and would-run cosign +
# attestation commands appear. Tagged `no-coverage`.
set -euo pipefail

prog="$1"

out="$(RELEASE_SIGN_DRY_RUN=1 "${prog}")"
echo "${out}"

echo "${out}" | grep -q -F -e 'tuf-repo-cdn.sigstore.dev' || { echo "signing missing trust root" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'cosign sign-blob' || { echo "signing missing cosign would-run" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'gh attestation create' || { echo "signing missing attestation would-run" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'cosign verify-blob' || { echo "signing missing verify hint" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'v2.4.1' || { echo "signing missing pinned cosign version v2.4.1 (issue #459)" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'application/vnd.dev.sigstore.bundle.v0.3+json' || { echo "signing missing bundle media type v0.3 (issue #459)" >&2; exit 1; }
echo "signing OK: dry-run would-sign with Sigstore keyless + attestation (cosign v2.4.1, bundle v0.3)"
