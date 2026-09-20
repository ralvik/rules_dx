#!/usr/bin/env bash
# Install-verification policy tests.
#
# Proves the mandatory authenticity-verification policy from
# docs/testing/tools.md: accept a valid artifact from the approved
# publisher; reject tampered bytes, a replaced binary/checksum pair
# without valid authenticity evidence, an unapproved signer, and
# missing/invalid/unavailable verification inputs; fail before
# installing or executing; no checksum-only fallback; verifier
# bootstrap + trust-root provenance reported.
#
# `$1` is the dx_verify.sh rootpath under test. Uses stub
# cosign/gh executables via DX_VERIFY_COSIGN/DX_VERIFY_GH so no
# network, Rekor, or TUF access is needed. Tagged `no-coverage`:
# process-spawning tests stay out of the coverage denominator per the
# repo coverage preset.
set -euo pipefail

# Host-tool contract: bash + python3 + POSIX coreutils
# only in this harness. Realpath and sha256 go through python3 (no
# `realpath`, `readlink -f`, `sha256sum`, or `shasum` probes).
py_realpath() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

verifier="$(py_realpath "$1")"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0
ok() { pass=$((pass + 1)); echo "ok: $1"; }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

sha_of() {
  python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
}

# Stub cosign: binds the bundle to the exact binary bytes plus the
# expected identity/issuer. Fails closed on anything else, proving
# tamper + unapproved-signer rejection without network access.
mkdir -p "$scratch/stubbin"
cat > "$scratch/stubbin/cosign" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == "version" ]]; then echo "cosign stub 1.0"; exit 0; fi
# args: verify-blob --bundle B --certificate-identity ID --certificate-issuer ISS BINARY
bundle=""; ident=""; issuer=""; bin=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    verify-blob) shift ;;
    --bundle) bundle="$2"; shift 2 ;;
    --certificate-identity) ident="$2"; shift 2 ;;
    --certificate-issuer) issuer="$2"; shift 2 ;;
    *) bin="$1"; shift ;;
  esac
done
[[ -n "$bundle" && -n "$ident" && -n "$issuer" && -n "$bin" ]] || { echo "stub cosign: bad args" >&2; exit 1; }
[[ "$ident" == "${STUB_EXPECT_IDENTITY:-}" ]] || { echo "stub cosign: unapproved signer $ident" >&2; exit 1; }
[[ "$issuer" == "${STUB_EXPECT_ISSUER:-}" ]] || { echo "stub cosign: unapproved issuer $issuer" >&2; exit 1; }
[[ -f "$bundle" && -f "$bin" ]] || { echo "stub cosign: missing inputs" >&2; exit 1; }
d="$(python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$bin")"
want="bundle-for-$d"
got="$(cat "$bundle")"
[[ "$got" == "$want" ]] || { echo "stub cosign: bundle does not bind binary bytes (tampered or replaced)" >&2; exit 1; }
exit 0
EOF
chmod +x "$scratch/stubbin/cosign"

# Stub gh for the attestation path.
cat > "$scratch/stubbin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
# args: attestation verify BINARY --owner OWNER
[[ "${1:-}" == "attestation" && "${2:-}" == "verify" ]] || { echo "stub gh: bad args" >&2; exit 1; }
bin="$3"
own=""
for a in "$@"; do case "$a" in --owner) : ;; esac; done
# crude owner extraction: last arg after --owner
prev=""
for a in "$@"; do
  if [[ "$prev" == "--owner" ]]; then own="$a"; fi
  prev="$a"
done
[[ "$own" == "${STUB_EXPECT_OWNER:-}" ]] || { echo "stub gh: unapproved owner $own" >&2; exit 1; }
[[ -f "$bin" ]] || { echo "stub gh: missing binary" >&2; exit 1; }
exit 0
EOF
chmod +x "$scratch/stubbin/gh"

IDENT="https://github.com/ralvik/rules_dx/.github/workflows/release.yml@refs/tags/v9.9.9"
ISSUER="https://token.actions.githubusercontent.com"
export STUB_EXPECT_IDENTITY="$IDENT"
export STUB_EXPECT_ISSUER="$ISSUER"
export STUB_EXPECT_OWNER="ralvik"

# A binary that marks execution: the verifier must never exec it.
marker="$scratch/should-never-exist"
printf '#!/usr/bin/env bash\ntouch "%s"\n' "$marker" > "$scratch/dx-fake"
chmod +x "$scratch/dx-fake"
printf 'standalone-dx-bytes-v1' >> "$scratch/dx-fake"
digest="$(sha_of "$scratch/dx-fake")"
printf 'bundle-for-%s' "$digest" > "$scratch/dx-fake.bundle"
printf 'bundle-for-%s' "$digest" > "$scratch/dx-fake.sbom"
printf 'sbom-bytes' > "$scratch/dx-fake.sbom-file"
sbom_digest="$(sha_of "$scratch/dx-fake.sbom-file")"
printf 'bundle-for-%s' "$sbom_digest" > "$scratch/dx-fake.sbom.bundle"

export DX_VERIFY_COSIGN="$scratch/stubbin/cosign"
export DX_VERIFY_GH="$scratch/stubbin/gh"
export PATH="$scratch/stubbin:$PATH"

# 1. Valid artifact accepted, installed, never executed, trust root reported.
out=""
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --install-dir "$scratch/install1" 2>&1)"; then
  if [[ -f "$scratch/install1/dx-fake" ]] && [[ ! -e "$marker" ]] && echo "$out" | grep -q -F -e 'tuf-repo-cdn.sigstore.dev'; then
    if [[ "$(sha_of "$scratch/install1/dx-fake")" == "$digest" ]]; then
      ok "valid artifact accepted, installed with digest, never executed, trust root reported"
    else
      bad "valid install digest mismatch"
    fi
  else
    bad "valid artifact missing install, executed binary, or missing trust root: $out"
  fi
else
  bad "valid artifact unexpectedly failed: $out"
fi

# 2. Checksum-only refused (no --bundle): must fail closed with the
# publisher-identity message and install nothing.
rm -rf "$scratch/install2"
if out="$("$verifier" --binary "$scratch/dx-fake" --identity "$IDENT" --issuer "$ISSUER" --install-dir "$scratch/install2" 2>&1)"; then
  bad "checksum-only (missing bundle) unexpectedly passed"
else
  if echo "$out" | grep -q -F -e 'checksum-only' || echo "$out" | grep -q -F -e 'missing --bundle'; then
    if [[ ! -e "$scratch/install2/dx-fake" ]]; then
      ok "checksum-only (missing bundle) refused before install"
    else
      bad "checksum-only failure still installed bytes"
    fi
  else
    bad "checksum-only refusal missing publisher-identity diagnostic: $out"
  fi
fi

# 3. Explicit --sha256 flag refused even with a bundle present.
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --sha256 "$scratch/dx-fake.bundle" 2>&1)"; then
  bad "explicit --sha256 unexpectedly passed"
else
  if echo "$out" | grep -q -F -e 'checksum-only'; then
    ok "explicit --sha256 refused as checksum-only fallback"
  else
    bad "explicit --sha256 refusal missing checksum-only diagnostic: $out"
  fi
fi

# 4. Missing identity fails closed.
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --issuer "$ISSUER" 2>&1)"; then
  bad "missing identity unexpectedly passed"
else
  if echo "$out" | grep -q -F -e 'missing --identity'; then
    ok "missing identity fails closed"
  else
    bad "missing identity diagnostic wrong: $out"
  fi
fi

# 5. Missing issuer fails closed.
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" 2>&1)"; then
  bad "missing issuer unexpectedly passed"
else
  if echo "$out" | grep -q -F -e 'missing --issuer'; then
    ok "missing issuer fails closed"
  else
    bad "missing issuer diagnostic wrong: $out"
  fi
fi

# 6. Tampered bytes rejected: append one byte, bundle no longer binds.
cp "$scratch/dx-fake" "$scratch/dx-tampered"
printf 'x' >> "$scratch/dx-tampered"
rm -rf "$scratch/install6"
if out="$("$verifier" --binary "$scratch/dx-tampered" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --install-dir "$scratch/install6" 2>&1)"; then
  bad "tampered bytes unexpectedly passed"
else
  if [[ ! -e "$scratch/install6/dx-tampered" ]] && [[ ! -e "$marker" ]]; then
    ok "tampered bytes rejected before install/exec"
  else
    bad "tampered failure still installed or executed"
  fi
fi

# 7. Replaced binary/checksum pair without authenticity rejected: fresh
# bytes with a fresh checksum but the old bundle must still fail.
cp "$scratch/dx-fake" "$scratch/dx-replaced"
printf 'replaced' >> "$scratch/dx-replaced"
python3 -c 'import hashlib,os,sys; p=sys.argv[1]; print(hashlib.sha256(open(p,"rb").read()).hexdigest()+"  "+os.path.basename(p))' "$scratch/dx-replaced" > "$scratch/dx-replaced.sha256"
rm -rf "$scratch/install7"
if out="$("$verifier" --binary "$scratch/dx-replaced" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --install-dir "$scratch/install7" 2>&1)"; then
  bad "replaced binary/bundle pair unexpectedly passed"
else
  if [[ ! -e "$scratch/install7/dx-replaced" ]]; then
    ok "replaced binary without valid authenticity evidence rejected"
  else
    bad "replaced pair failure still installed bytes"
  fi
fi

# 8. Unapproved signer rejected.
rm -rf "$scratch/install8"
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "https://evil.example/workflow" --issuer "$ISSUER" --install-dir "$scratch/install8" 2>&1)"; then
  bad "unapproved signer unexpectedly passed"
else
  if [[ ! -e "$scratch/install8/dx-fake" ]]; then
    ok "unapproved signer rejected before install"
  else
    bad "unapproved signer failure still installed bytes"
  fi
fi

# 9. Unavailable verifier fails closed: empty PATH stubs, no overrides.
rm -rf "$scratch/install9"
if out="$(PATH=/usr/bin:/bin DX_VERIFY_COSIGN=/nonexistent-cosign DX_VERIFY_GH=/nonexistent-gh "$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" 2>&1)"; then
  bad "unavailable verifier unexpectedly passed"
else
  if echo "$out" | grep -q -F -e 'no verifier available'; then
    ok "missing/unavailable verification inputs fail closed"
  else
    bad "unavailable verifier diagnostic wrong: $out"
  fi
fi

# 10. SBOM bundle binds the same way: substituted SBOM fails.
rm -rf "$scratch/install10"
printf 'tampered-sbom' > "$scratch/dx-fake.sbom-file-tampered"
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --sbom "$scratch/dx-fake.sbom-file-tampered" --sbom-bundle "$scratch/dx-fake.sbom.bundle" 2>&1)"; then
  bad "substituted SBOM unexpectedly passed"
else
  ok "substituted SBOM rejected before install"
fi

# 11. Valid SBOM passes alongside the binary.
rm -rf "$scratch/install11"
if out="$("$verifier" --binary "$scratch/dx-fake" --bundle "$scratch/dx-fake.bundle" --identity "$IDENT" --issuer "$ISSUER" --sbom "$scratch/dx-fake.sbom-file" --sbom-bundle "$scratch/dx-fake.sbom.bundle" --install-dir "$scratch/install11" 2>&1)"; then
  if [[ -f "$scratch/install11/dx-fake" ]]; then
    ok "valid SBOM accepted alongside binary"
  else
    bad "valid SBOM path missing install"
  fi
else
  bad "valid SBOM path unexpectedly failed: $out"
fi

echo "dx install-verification policy: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
