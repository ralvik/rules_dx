#!/usr/bin/env bash
# Upstream C/C++ IDE tool acquisition proof.
#
# Proves the pinned C++ toolchain supplies the compiler the environment
# contract reuses for action-derived compile commands plus managed clangd.
# The version binary prints `__VERSION__`; managed host-native clangd
# resolution stays open under the native plan.
set -euo pipefail
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
bin="$(dx_realpath "$1")"
out="$("$bin" 2>&1)"
echo "$out"
case "$out" in
  *.*) ;;
  *) echo "cc version missing version number" >&2; echo "$out" >&2; exit 1;;
esac
echo "ide acquisition: cc compiler answers version"
