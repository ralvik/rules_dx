#!/usr/bin/env bash
# Hello smoke as a test target: asserts the seed binary prints the expected
# greeting so `bazel test //...` covers the binary launch path (#87, closes
# the TS hole from #98). Shell sources have no corpus class.
set -euo pipefail

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

hello_bin="$(portable_realpath "$1")"

output="$("${hello_bin}")"
[[ "${output}" == "hello world" ]] || {
  echo "unexpected hello output: ${output}" >&2
  exit 1
}
