#!/usr/bin/env bash
# Verifies one `sbom_release` output pair (issue #311).
#
# `$1` is the artifact rootpath, `$2` the SPDX rootpath, `$3` the
# provenance rootpath. Asserts the SPDX document is SPDX-2.3 with the
# artifact sha256, and the provenance statement is in-toto v1 + SLSA v1
# with the same digest as subject. Tagged `no-coverage`.
set -euo pipefail

portable_realpath() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  elif command -v readlink >/dev/null 2>&1 && readlink -f "$1" >/dev/null 2>&1; then
    readlink -f "$1"
  else
    python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
  fi
}

artifact="$(portable_realpath "$1")"
spdx="$(portable_realpath "$2")"
prov="$(portable_realpath "$3")"

if command -v sha256sum >/dev/null 2>&1; then
  digest="$(sha256sum "$artifact" | cut -d' ' -f1)"
else
  digest="$(shasum -a 256 "$artifact" | cut -d' ' -f1)"
fi

grep -q -F -e '"spdxVersion": "SPDX-2.3"' "$spdx" || { echo "sbom spdxVersion not SPDX-2.3" >&2; exit 1; }
grep -q -F -e "$digest" "$spdx" || { echo "sbom SPDX missing artifact digest $digest" >&2; exit 1; }
grep -q -F -e '"_type": "https://in-toto.io/Statement/v1"' "$prov" || { echo "provenance missing in-toto Statement v1" >&2; exit 1; }
grep -q -F -e '"predicateType": "https://slsa.dev/provenance/v1"' "$prov" || { echo "provenance missing SLSA v1 predicate" >&2; exit 1; }
grep -q -F -e "$digest" "$prov" || { echo "provenance missing subject digest $digest" >&2; exit 1; }
echo "sbom OK: SPDX-2.3 + SLSA v1 bind $digest"
