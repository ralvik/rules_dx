#!/usr/bin/env bash
# Hello smoke as a test target: asserts the seed binary prints the expected
# greeting so `bazel test //...` covers the binary launch path (#87).
# Shell sources have no corpus class.
set -euo pipefail

hello_bin="$(realpath "$1")"

output="$("${hello_bin}")"
[[ "${output}" == "hello world" ]] || {
  echo "unexpected hello output: ${output}" >&2
  exit 1
}
