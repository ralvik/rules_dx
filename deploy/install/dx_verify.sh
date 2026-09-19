#!/usr/bin/env bash
# Standalone `dx` install-time publisher-identity verifier (issue #26).
#
# Verifies a downloaded standalone `dx` binary against its Sigstore
# keyless bundle (Fulcio OIDC + Rekor public-good, `cosign sign-blob
# --bundle`) or GitHub Artifact Attestation before installing or
# executing it. There is no checksum-only fallback: a sha256 delivered
# alongside a binary is not by itself proof of publisher identity.
#
# Trust root (documented, not self-hosted): Sigstore TUF
# (`https://tuf-repo-cdn.sigstore.dev`). Verifiers: `cosign verify-blob`
# / `gh attestation verify` / `slsa-verifier`. BCR itself needs no
# signing (archive `source.json` + integrity hash + `presubmit.yml` +
# PR review); signing covers GitHub Release binaries. SBOM (Syft/CycloneDX)
# bundles verify through the same cosign path when passed as --sbom.
# Order per #26: human-run signing workflow (#78) first, then GHCR
# images (#184, separate workflow, `cosign sign <digest>`).
#
# Usage:
#   dx_verify.sh --binary PATH --bundle PATH --identity ID --issuer ISSUER \
#     [--attestation PATH] [--owner OWNER] [--sbom PATH --sbom-bundle PATH] \
#     [--install-dir DIR]
#
# All of --binary/--bundle/--identity/--issuer are mandatory. Any
# --sha256/--checksum argument is rejected: checksum-only verification
# is not publisher-identity proof. Verification runs before any install
# or exec; on failure nothing is installed and the binary is never
# executed. On success the binary is copied (no exec) to --install-dir
# when given, else reported OK in place.
#
# Test hooks (CI only, no network): DX_VERIFY_COSIGN and DX_VERIFY_GH
# override the verifier executables so sh_tests can inject stubs that
# prove dispatch + refusal policy without Rekor/TUF access.
set -euo pipefail

# Host-tool contract (issue #318): bash + python3 + POSIX coreutils plus
# the publisher-identity verifiers (`cosign`/`gh`) only. Realpath and
# sha256 go through python3 (no `realpath`, `readlink -f`, `sha256sum`,
# or `shasum` probes); `cp`/`mkdir`/`basename`/`chmod` are POSIX coreutils.
py_realpath() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

py_sha256() {
  python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
}

TRUST_ROOT="https://tuf-repo-cdn.sigstore.dev"
HELP="usage: dx_verify.sh --binary PATH --bundle PATH --identity ID --issuer ISSUER [--attestation PATH] [--owner OWNER] [--sbom PATH --sbom-bundle PATH] [--install-dir DIR]"

binary=""
bundle=""
identity=""
issuer=""
attestation=""
owner=""
sbom=""
sbom_bundle=""
install_dir=""

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --binary) binary="${2:-}"; shift 2 ;;
    --bundle) bundle="${2:-}"; shift 2 ;;
    --identity) identity="${2:-}"; shift 2 ;;
    --issuer) issuer="${2:-}"; shift 2 ;;
    --attestation) attestation="${2:-}"; shift 2 ;;
    --owner) owner="${2:-}"; shift 2 ;;
    --sbom) sbom="${2:-}"; shift 2 ;;
    --sbom-bundle) sbom_bundle="${2:-}"; shift 2 ;;
    --install-dir) install_dir="${2:-}"; shift 2 ;;
    --sha256|--checksum|--sha256-file|--checksum-file)
      echo "dx_verify: checksum-only verification is not publisher-identity proof; pass --bundle plus --identity/--issuer (issue #26)" >&2
      exit 1
      ;;
    -h|--help) echo "$HELP"; exit 0 ;;
    *) echo "dx_verify: unknown argument '$1'" >&2; echo "$HELP" >&2; exit 1 ;;
  esac
done

if [[ -z "$binary" ]]; then
  echo "dx_verify: missing --binary PATH (standalone dx binary to verify)" >&2
  exit 1
fi
if [[ -z "$bundle" ]]; then
  echo "dx_verify: missing --bundle PATH; checksum-only verification is not publisher-identity proof (issue #26)" >&2
  exit 1
fi
if [[ -z "$identity" ]]; then
  echo "dx_verify: missing --identity ID (expected certificate identity, e.g. GitHub Actions workflow identity)" >&2
  exit 1
fi
if [[ -z "$issuer" ]]; then
  echo "dx_verify: missing --issuer ISSUER (expected OIDC issuer, e.g. https://token.actions.githubusercontent.com)" >&2
  exit 1
fi
if [[ -n "$sbom" && -z "$sbom_bundle" ]]; then
  echo "dx_verify: --sbom needs --sbom-bundle (SBOM verifies through the same cosign path)" >&2
  exit 1
fi
if [[ -z "$sbom" && -n "$sbom_bundle" ]]; then
  echo "dx_verify: --sbom-bundle needs --sbom" >&2
  exit 1
fi

if [[ ! -f "$binary" ]]; then
  echo "dx_verify: binary not found: $binary" >&2
  exit 1
fi
if [[ ! -f "$bundle" ]]; then
  echo "dx_verify: bundle not found: $bundle (missing, invalid, or unavailable verification inputs fail closed)" >&2
  exit 1
fi
if [[ -n "$attestation" && ! -f "$attestation" ]]; then
  echo "dx_verify: attestation not found: $attestation" >&2
  exit 1
fi
if [[ -n "$sbom" && ! -f "$sbom" ]]; then
  echo "dx_verify: sbom not found: $sbom" >&2
  exit 1
fi
if [[ -n "$sbom_bundle" && ! -f "$sbom_bundle" ]]; then
  echo "dx_verify: sbom bundle not found: $sbom_bundle" >&2
  exit 1
fi
if [[ ! -s "$bundle" ]]; then
  echo "dx_verify: bundle is empty: $bundle" >&2
  exit 1
fi

binary_abs="$(py_realpath "$binary")"
bundle_abs="$(py_realpath "$bundle")"

echo "dx_verify: trust root $TRUST_ROOT (Sigstore TUF public-good; verifiers bootstrapped from the trust root, never alongside the binary)"
if command -v cosign >/dev/null 2>&1; then
  echo "dx_verify: cosign $(cosign version 2>&1 | head -n 1 || true)"
fi
if command -v gh >/dev/null 2>&1; then
  echo "dx_verify: gh $(gh --version 2>&1 | head -n 1 || true)"
fi

cosign_bin="${DX_VERIFY_COSIGN:-cosign}"
gh_bin="${DX_VERIFY_GH:-gh}"

verified=""
# Prefer the Sigstore bundle path (signing-first per #26). The bundle
# must bind the exact binary bytes: the verifier checks the bundle
# before anything is installed or executed.
if command -v "$cosign_bin" >/dev/null 2>&1; then
  echo "dx_verify: verifying $binary_abs against bundle via $cosign_bin verify-blob"
  if "$cosign_bin" verify-blob --bundle "$bundle_abs" --certificate-identity "$identity" --certificate-issuer "$issuer" "$binary_abs"; then
    verified="cosign"
    echo "dx_verify: cosign publisher-identity OK (identity=$identity issuer=$issuer)"
  else
    echo "dx_verify: cosign verification failed (tampered bytes, replaced binary/bundle, or unapproved signer fail closed)" >&2
    exit 1
  fi
elif [[ -n "$attestation" ]] && command -v "$gh_bin" >/dev/null 2>&1; then
  if [[ -z "$owner" ]]; then
    echo "dx_verify: --owner OWNER is required with --attestation (GitHub attestation path)" >&2
    exit 1
  fi
  echo "dx_verify: verifying $binary_abs against attestation via $gh_bin attestation verify"
  if "$gh_bin" attestation verify "$binary_abs" --owner "$owner"; then
    verified="gh-attestation"
    echo "dx_verify: attestation publisher-identity OK (owner=$owner)"
  else
    echo "dx_verify: attestation verification failed (fail closed before install)" >&2
    exit 1
  fi
else
  echo "dx_verify: no verifier available (want $cosign_bin verify-blob or $gh_bin attestation verify); missing, invalid, or unavailable verification inputs fail closed" >&2
  exit 1
fi

# Optional SBOM bundle binds the same release bytes through the same
# cosign path; a substituted SBOM fails here before install.
if [[ -n "$sbom" ]]; then
  sbom_abs="$(py_realpath "$sbom")"
  sbom_bundle_abs="$(py_realpath "$sbom_bundle")"
  if ! command -v "$cosign_bin" >/dev/null 2>&1; then
    echo "dx_verify: SBOM verification needs $cosign_bin (unavailable)" >&2
    exit 1
  fi
  echo "dx_verify: verifying SBOM $sbom_abs via $cosign_bin verify-blob"
  if "$cosign_bin" verify-blob --bundle "$sbom_bundle_abs" --certificate-identity "$identity" --certificate-issuer "$issuer" "$sbom_abs"; then
    echo "dx_verify: SBOM publisher-identity OK"
  else
    echo "dx_verify: SBOM verification failed (fail closed before install)" >&2
    exit 1
  fi
fi

# Verification succeeded before any install or exec. Install is a copy
# only; the binary is never executed here.
if [[ -n "$install_dir" ]]; then
  mkdir -p "$install_dir"
  base="$(basename "$binary_abs")"
  digest="$(py_sha256 "$binary_abs")"
  cp -RPp "$binary_abs" "$install_dir/$base"
  chmod 0755 "$install_dir/$base"
  echo "dx_verify: installed $base to $install_dir (sha256 $digest, verified via $verified)"
else
  echo "dx_verify: OK (verified via $verified; nothing installed, binary never executed)"
fi
