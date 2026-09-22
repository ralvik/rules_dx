#!/usr/bin/env bash
# Single-sourced shell bootstrap (issue #654).
#
# Single-sources the runfiles-first probe repeated across every shell
# driver, so depth-adjusted `../` source-tree variants disappear. Drivers
# carry one identical loader plus one `dx_bootstrap` line per library
# (`data = ["//tools/sh:lib"]` carries this file in the runfiles forest
# via `//tools/sh:bootstrap`):
#
#   source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
#   dx_bootstrap "tools/sh/lib.sh"
#
# Provides:
#   dx_bootstrap <workspace-rel>   sources a workspace-relative shell
#                                library (e.g. `tools/sh/lib.sh`)
#                                runfiles-first, then source tree with no
#                                per-file depth adjustment.
#
# Bash-only Linux harness: sourced by `sh_binary` /
# `sh_test` drivers carrying `target_compatible_with =
# ["@platforms//os:linux"]`. Bootstrap requires bash by design under issue
# #450 (`BASH_SOURCE`, `[[` plus the runfiles fallback never run under
# POSIX `sh`); floor is bash 3.2+. Intentional lib-free exceptions stay in
# `tools/sh/lib.sh` (POSIX `#!/bin/sh` fixtures plus deploy hermetic
# python-only runtime, no bootstrap). Guard maintenance owns shared helpers
# plus snapshot versus grep policy (`tools/sh/lib.sh` plus
# `tools/sh/guards.sh` plus `tools/sh/snapshot.sh` with UPDATE_EXPECT;
# `//tools/ci:shell_contract` owns the rule).
# Shellcheck/shfmt clean (`shfmt -i 2 -ci`, `.shellcheckrc` bash + all
# checks). SC1090/SC1091 are single-sourced in `.shellcheckrc` (runfiles
# layouts exist only under `bazel run` / `bazel test`, issue #319): no
# per-line pragmas here (issue #914).
set -euo pipefail

# dx_bootstrap <workspace-rel>
# Sources a workspace-relative shell library runfiles-first
# (`RUNFILES_DIR` / `TEST_SRCDIR` / `$0.runfiles` /
# `BASH_SOURCE.runfiles` `_main` layouts), then `BUILD_WORKSPACE_DIRECTORY`,
# then the enclosing git top-level, then an upward walk from the caller so
# no per-file `../` depth adjustment is needed. Fails actionably when the
# library cannot be located.
dx_bootstrap() {
  local rel="${1:-}" top="" caller="" caller_dir="" dir="" parent=""
  if [[ -z "$rel" ]]; then
    echo "dx_bootstrap: want workspace-relative path (e.g. tools/sh/lib.sh)" >&2
    return 1
  fi
  source "${RUNFILES_DIR:-/dev/null}/_main/${rel}" 2>/dev/null && return 0 || true
  source "${TEST_SRCDIR:-/dev/null}/_main/${rel}" 2>/dev/null && return 0 || true
  source "$0.runfiles/_main/${rel}" 2>/dev/null && return 0 || true
  source "${BASH_SOURCE[0]}.runfiles/_main/${rel}" 2>/dev/null && return 0 || true
  caller="${BASH_SOURCE[1]:-${BASH_SOURCE[0]:-$0}}"
  source "${caller}.runfiles/_main/${rel}" 2>/dev/null && return 0 || true
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -f "${BUILD_WORKSPACE_DIRECTORY}/${rel}" ]]; then
    source "${BUILD_WORKSPACE_DIRECTORY}/${rel}" && return 0 || return 1
  fi
  top="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  if [[ -n "$top" && -f "$top/${rel}" ]]; then
    source "$top/${rel}" && return 0 || return 1
  fi
  caller_dir="$(dirname "$caller")"
  dir="$caller_dir"
  while true; do
    if [[ -f "$dir/${rel}" ]]; then
      source "$dir/${rel}" && return 0 || return 1
    fi
    if [[ "$dir" == "/" || "$dir" == "." ]]; then
      break
    fi
    parent="$(dirname "$dir")"
    if [[ "$parent" == "$dir" ]]; then
      break
    fi
    dir="$parent"
  done
  echo "dx_bootstrap: cannot locate ${rel} (runfiles plus BUILD_WORKSPACE_DIRECTORY plus git top-level plus source-tree walk failed)" >&2
  return 1
}
