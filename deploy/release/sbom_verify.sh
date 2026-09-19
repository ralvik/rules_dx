#!/usr/bin/env bash
# Verifies one `sbom_release` output pair (issue #311).
#
# `$1` is the artifact rootpath, `$2` the SPDX rootpath, `$3` the
# provenance rootpath. Asserts the SPDX document is SPDX-2.3 with the
# artifact sha256, and the provenance statement is in-toto v1 + SLSA v1
# with the same digest as subject. Tagged `no-coverage`.
#
# Host-tool contract (issue #318): bash + python3 + POSIX coreutils
# only. Realpath and sha256 go through python3 (no `realpath`,
# `readlink -f`, `sha256sum`, or `shasum` probes).
set -euo pipefail

py_realpath() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

py_sha256() {
  python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
}

artifact="$(py_realpath "$1")"
spdx="$(py_realpath "$2")"
prov="$(py_realpath "$3")"

digest="$(py_sha256 "$artifact")"

grep -q -F -e '"spdxVersion": "SPDX-2.3"' "$spdx" || { echo "sbom spdxVersion not SPDX-2.3" >&2; exit 1; }
grep -q -F -e "$digest" "$spdx" || { echo "sbom SPDX missing artifact digest $digest" >&2; exit 1; }
grep -q -F -e '"_type": "https://in-toto.io/Statement/v1"' "$prov" || { echo "provenance missing in-toto Statement v1" >&2; exit 1; }
grep -q -F -e '"predicateType": "https://slsa.dev/provenance/v1"' "$prov" || { echo "provenance missing SLSA v1 predicate" >&2; exit 1; }
grep -q -F -e "$digest" "$prov" || { echo "provenance missing subject digest $digest" >&2; exit 1; }
echo "sbom OK: SPDX-2.3 + SLSA v1 bind $digest"
