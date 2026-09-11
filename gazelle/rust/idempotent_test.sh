#!/usr/bin/env bash
set -euo pipefail

gazelle="$(realpath "$1")"
root="${TEST_TMPDIR}/workspace"
mkdir -p "${root}/crate/src" "${root}/crate/tests"
touch "${root}/WORKSPACE"
printf 'pub fn current() {}\n\n#[test]\nfn works() {}\n' > "${root}/crate/src/lib.rs"
printf 'fn main() {}\n' > "${root}/crate/src/main.rs"
printf '#[test]\nfn smoke() {}\n' > "${root}/crate/tests/smoke.rs"

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' -exec sha256sum {} + | sort > "${TEST_TMPDIR}/first.sums")

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' -exec sha256sum {} + | sort > "${TEST_TMPDIR}/second.sums")

if ! diff -u "${TEST_TMPDIR}/first.sums" "${TEST_TMPDIR}/second.sums"; then
  echo "repository generation is not idempotent" >&2
  exit 1
fi
