#!/usr/bin/env bash
set -euo pipefail

gazelle="$(realpath "$1")"
root="${TEST_TMPDIR}/workspace"
mkdir -p "${root}/crate/src"
touch "${root}/WORKSPACE"
printf 'use missing_crate::Thing;\n' > "${root}/crate/src/lib.rs"

set +e
output="$(cd "${root}" && "${gazelle}" -repo_root="${root}" 2>&1)"
status=$?
set -e

if [[ ${status} -eq 0 ]]; then
  echo "expected unresolved import to fail" >&2
  exit 1
fi
if [[ "${output}" != *'unresolved import "missing_crate"'* ]]; then
  echo "missing actionable unresolved-import diagnostic" >&2
  echo "${output}" >&2
  exit 1
fi
if [[ -e "${root}/crate/BUILD.bazel" || -e "${root}/crate/BUILD" ]]; then
  echo "failed generation wrote a BUILD file" >&2
  exit 1
fi

printf '# gazelle:dx_ignore_import rust missing_crate\n' > "${root}/BUILD.bazel"
(cd "${root}" && "${gazelle}" -repo_root="${root}")
if [[ ! -f "${root}/crate/BUILD.bazel" ]]; then
  echo "exact inherited ignore did not permit generation" >&2
  exit 1
fi

rm "${root}/crate/src/lib.rs" "${root}/crate/BUILD.bazel"
set +e
output="$(cd "${root}" && "${gazelle}" -repo_root="${root}" 2>&1)"
status=$?
set -e
if [[ ${status} -eq 0 || "${output}" != *'stale # gazelle:dx_ignore_import'* ]]; then
  echo "stale ignore did not fail" >&2
  echo "${output}" >&2
  exit 1
fi
