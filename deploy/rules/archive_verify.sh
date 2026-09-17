#!/usr/bin/env bash
# Verifies one `archive_release` output pair (issue #181).
#
# `$1` is the tarball rootpath, `$2` the checksum rootpath, `$3` the
# expected top-level member basename. Asserts the tarball lists the
# member and the checksum file matches a fresh digest of the tarball.
# Tagged `no-coverage`: process-spawning tests stay out of the coverage
# denominator per the repo coverage preset.
set -euo pipefail

tarball="$(realpath "$1")"
checksum="$(realpath "$2")"
member="$3"

members="$(tar -tzf "${tarball}")"
echo "${members}" | grep -qx "${member}" || {
  echo "archive member '${member}' not found in ${tarball}" >&2
  echo "tarball contents:" >&2
  echo "${members}" >&2
  exit 1
}

if command -v sha256sum >/dev/null 2>&1; then
  expected="$(cut -d' ' -f1 "${checksum}")"
  actual="$(sha256sum "${tarball}" | cut -d' ' -f1)"
else
  expected="$(cut -d' ' -f1 "${checksum}")"
  actual="$(shasum -a 256 "${tarball}" | cut -d' ' -f1)"
fi
[[ "${expected}" == "${actual}" ]] || {
  echo "checksum mismatch for ${tarball}" >&2
  echo "  expected: ${expected}" >&2
  echo "  actual:   ${actual}" >&2
  exit 1
}
echo "archive OK: ${tarball} holds ${member}, sha256 ${actual}"
