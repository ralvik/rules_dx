#!/usr/bin/env bash
# Verifies one `archive_deploy` output pair.
#
# `$1` is the tarball rootpath, `$2` the checksum rootpath, `$3` the
# expected top-level member basename. Asserts the tarball lists the
# member and the checksum file matches a fresh digest of the tarball.
# Tagged `no-coverage`: process-spawning tests stay out of the coverage
# denominator per the repo coverage preset.
#
# Host-tool contract: bash + python3 + POSIX coreutils
# only. Realpath, tar listing, and sha256 go through python3 (no
# `realpath`, `readlink -f`, `tar`, `sha256sum`, or `shasum` probes).
set -euo pipefail

# Single-tool realpath via python3 (portable across Linux/macOS).
py_realpath() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

# sha256 of one file via python3 hashlib.
py_sha256() {
  python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
}

# List top-level tar.gz members via python3 tarfile (no host `tar`).
py_tar_list() {
  python3 -c 'import sys,tarfile; print("\n".join(tarfile.open(sys.argv[1],"r:gz").getnames()))' "$1"
}

tarball="$(py_realpath "$1")"
checksum="$(py_realpath "$2")"
member="$3"

members="$(py_tar_list "${tarball}")"
echo "${members}" | grep -qx "${member}" || {
  echo "archive member '${member}' not found in ${tarball}" >&2
  echo "tarball contents:" >&2
  echo "${members}" >&2
  exit 1
}

expected="$(cut -d' ' -f1 "${checksum}")"
actual="$(py_sha256 "${tarball}")"
[[ "${expected}" == "${actual}" ]] || {
  echo "checksum mismatch for ${tarball}" >&2
  echo "  expected: ${expected}" >&2
  echo "  actual:   ${actual}" >&2
  exit 1
}
echo "archive OK: ${tarball} holds ${member}, sha256 ${actual}"
