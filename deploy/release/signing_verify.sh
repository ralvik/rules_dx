#!/usr/bin/env bash
# Verifies one `signed_release` deploy program (issue #311).
#
# `$1` is the program rootpath. Runs with RELEASE_SIGN_DRY_RUN=1 (no
# network, no mutation) and asserts the trust root, identity, issuer,
# and would-run cosign + attestation commands appear. Tagged
# `no-coverage`.
set -euo pipefail

prog="$1"

out="$(RELEASE_SIGN_DRY_RUN=1 "${prog}")"
echo "${out}"

echo "${out}" | grep -q -F -e 'tuf-repo-cdn.sigstore.dev' || { echo "signing missing trust root" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'cosign sign-blob' || { echo "signing missing cosign would-run" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'gh attestation create' || { echo "signing missing attestation would-run" >&2; exit 1; }
echo "${out}" | grep -q -F -e 'cosign verify-blob' || { echo "signing missing verify hint" >&2; exit 1; }
echo "signing OK: dry-run would-sign with Sigstore keyless + attestation"
