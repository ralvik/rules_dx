#!/usr/bin/env bash
# Hello smoke as a test target: asserts the seed binary prints the expected
# greeting so `bazel test //...` covers the binary launch path.
# Shell sources have no corpus class.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../../tools/sh/lib.sh"

# Portable realpath via tools/sh/lib.sh dx_realpath.

hello_bin="$(dx_realpath "$1")"

output="$("${hello_bin}")"
[[ "${output}" == "hello 42" ]] || {
  echo "unexpected hello output: ${output}" >&2
  exit 1
}
