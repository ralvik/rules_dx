#!/usr/bin/env bash
set -euo pipefail

# Portable hasher selection (issue #299): GNU `sha256sum` is absent on
# macOS; `shasum -a 256` is the portable fallback. Linux behavior unchanged.
if command -v sha256sum >/dev/null 2>&1; then
  _sha256=(sha256sum)
else
  _sha256=(shasum -a 256)
fi

# Portable realpath (issue #299): GNU `realpath` is absent on macOS;
# `readlink -f` covers some platforms, python3 covers the rest.
portable_realpath() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  elif command -v readlink >/dev/null 2>&1 && readlink -f "$1" >/dev/null 2>&1; then
    readlink -f "$1"
  else
    python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
  fi
}

gazelle="$(portable_realpath "$1")"
root="${TEST_TMPDIR}/workspace"
mkdir -p "${root}/crate/src" "${root}/crate/tests"
touch "${root}/WORKSPACE"
printf 'pub fn current() {}\n\n#[test]\nfn works() {}\n' > "${root}/crate/src/lib.rs"
printf 'fn main() {}\n' > "${root}/crate/src/main.rs"
printf '#[test]\nfn smoke() {}\n' > "${root}/crate/tests/smoke.rs"

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' -exec "${_sha256[@]}" {} + | sort > "${TEST_TMPDIR}/first.sums")

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' -exec "${_sha256[@]}" {} + | sort > "${TEST_TMPDIR}/second.sums")

if ! diff -u "${TEST_TMPDIR}/first.sums" "${TEST_TMPDIR}/second.sums"; then
  echo "repository generation is not idempotent" >&2
  exit 1
fi
