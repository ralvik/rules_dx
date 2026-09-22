#!/usr/bin/env bash
# Upstream .NET IDE tool acquisition proof (C#).
set -euo pipefail
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
bin="$(dx_realpath "$1")"
out="$("$bin" --info 2>&1)"
echo "$out"
case "$out" in
  *"10.0.201"*) ;;
  *) echo "dotnet --info missing 10.0.201" >&2; echo "$out" >&2; exit 1;;
esac
echo "ide acquisition: dotnet answers --info"
