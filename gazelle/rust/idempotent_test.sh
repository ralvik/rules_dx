#!/usr/bin/env bash
# Gazelle idempotence check (snapshot workflow issue #322): compares two
# fresh generation runs, not a checked-in golden, so UPDATE_EXPECT does not
# apply. The `diff -u` below is an idempotence assertion (run-to-run
# equality), not a snapshot refresh path.
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../../tools/sh/lib.sh"

# Portable helpers via tools/sh/lib.sh dx_realpath/dx_sha256 (issues #299, #323).

gazelle="$(dx_realpath "$1")"
root="${TEST_TMPDIR}/workspace"
mkdir -p "${root}/crate/src" "${root}/crate/tests"
touch "${root}/WORKSPACE"
printf 'pub fn current() {}\n\n#[test]\nfn works() {}\n' >"${root}/crate/src/lib.rs"
printf 'fn main() {}\n' >"${root}/crate/src/main.rs"
printf '#[test]\nfn smoke() {}\n' >"${root}/crate/tests/smoke.rs"

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' | LC_ALL=C sort | while IFS= read -r f; do printf '%s  %s\n' "$(dx_sha256_file "$f")" "$f"; done | LC_ALL=C sort >"${TEST_TMPDIR}/first.sums")

(cd "${root}" && "${gazelle}" -repo_root="${root}")
(cd "${root}" && find . -name 'BUILD.bazel' | LC_ALL=C sort | while IFS= read -r f; do printf '%s  %s\n' "$(dx_sha256_file "$f")" "$f"; done | LC_ALL=C sort >"${TEST_TMPDIR}/second.sums")

if ! diff -u "${TEST_TMPDIR}/first.sums" "${TEST_TMPDIR}/second.sums"; then
  echo "repository generation is not idempotent" >&2
  exit 1
fi
