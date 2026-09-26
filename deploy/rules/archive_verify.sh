#!/usr/bin/env bash
set -euo pipefail

py_realpath() {
  python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

py_sha256() {
  python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"
}

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
