#!/usr/bin/env bash
# Shared workspace-root + runfiles helpers (issue #319).
#
# Single-sources the `BUILD_WORKSPACE_DIRECTORY || git rev-parse` workspace
# probe plus the multi-candidate runfiles probing (`RUNFILES_DIR`,
# `TEST_SRCDIR` layouts, `bazel-bin` fallbacks) repeated across every shell
# driver. Drivers source this file via a runfiles-first bootstrap so both
# direct execution and Bazel `run`/`test` layouts work (`data =
# ["//tools/sh:lib"]` carries it in the runfiles forest):
#
#   source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"
#
# (adjust the trailing `../` depth for the source-tree fallback:
# `tools/ci/*.sh` and `tools/depcheck/*.sh` use `../sh/lib.sh`,
# `perf/*.sh` use `../tools/sh/lib.sh`, `cli/env/*.sh` use
# `../../tools/sh/lib.sh`.)
#
# Rust binaries share `dx_process::workspace_start` plus standard `runfiles`
# `rlocation` (never `TEST_SRCDIR` in prod); see `cli/process/src/lib.rs`.
#
# Provides:
#   dx_workspace_root            prints the checkout root
#   dx_e2e_workspace_root        prints the E2E parent workspace
#                                (`E2E_WORKSPACE` override, else shared root)
#   dx_runfiles_root             prints the Bazel runfiles root
#                                (`RUNFILES_DIR` else `TEST_SRCDIR`)
#   dx_resolve_runfile <rel>     prints the absolute path for a
#                                workspace-relative or `rootpath` rel
#   dx_perf_host                 prints the perf host label
#                                (`linux_x86_64`, `linux_arm64`,
#                                `macos_arm64`, ...); fails fast on unknown
#                                OS/CPU instead of silently recording the
#                                seed host (issue #320 portable route).
#
# `dx_resolve_runfile` prefers the standard `runfiles.bash` `rlocation`
# when available and falls back to manual `TEST_SRCDIR` / `RUNFILES_DIR` /
# `bazel-bin` probing for `bazel run` invocations plus `git` / cwd for
# direct execution. Drivers must not reimplement workspace or runfiles
# probing; extend this file instead.
#
# Bash-only Linux harness (issue #299): sourced by `sh_binary` / `sh_test`
# drivers carrying `target_compatible_with = ["@platforms//os:linux"]`.
# No `realpath`, `sha256sum`, `sed -i`, `cp -a`, or `$EPOCHREALTIME`
# probing here (issues #318, #323 own those).
set -euo pipefail

# Bring `rlocation` into scope when running under Bazel. Non-fatal:
# direct execution has no runfiles tree, so fall back to manual probing.
if ! declare -F rlocation >/dev/null 2>&1; then
  _dx_runfiles_bash="bazel_tools/tools/bash/runfiles/runfiles.bash"
  # shellcheck disable=SC1090
  source "${RUNFILES_DIR:-/dev/null}/$_dx_runfiles_bash" 2>/dev/null || \
    source "$(grep -sm1 "^$_dx_runfiles_bash " "${RUNFILES_MANIFEST_FILE:-/dev/null}" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null || \
    source "$0.runfiles/$_dx_runfiles_bash" 2>/dev/null || \
    source "$(grep -sm1 "^$_dx_runfiles_bash " "$0.runfiles_manifest" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null || \
    source "$(grep -sm1 "^$_dx_runfiles_bash " "$0.exe.runfiles_manifest" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null || \
    true
  unset _dx_runfiles_bash
fi

# Prints the checkout root: `BUILD_WORKSPACE_DIRECTORY` under `bazel run`,
# else the enclosing git top-level. Fails actionably outside a checkout.
dx_workspace_root() {
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY"
    return 0
  fi
  git rev-parse --show-toplevel
}

# Prints the E2E parent workspace (issue #55): explicit `E2E_WORKSPACE`
# wins, else `BUILD_WORKSPACE_DIRECTORY` when it holds `integration/`,
# else the enclosing git top-level. Fails with the actionable
# `E2E_WORKSPACE` hint instead of bare git noise.
dx_e2e_workspace_root() {
  if [[ -n "${E2E_WORKSPACE:-}" ]]; then
    printf '%s\n' "$E2E_WORKSPACE"
    return 0
  fi
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -d "${BUILD_WORKSPACE_DIRECTORY}/integration" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY"
    return 0
  fi
  local root
  if root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    printf '%s\n' "$root"
    return 0
  fi
  echo "FAIL: cannot locate parent workspace (no \$E2E_WORKSPACE, no usable \$BUILD_WORKSPACE_DIRECTORY, git rev-parse failed)" >&2
  echo "Run explicitly as: E2E_WORKSPACE=\$PWD bazel test //tools/ci:e2e --test_env=E2E_WORKSPACE" >&2
  echo "CI passes E2E_WORKSPACE=\$GITHUB_WORKSPACE with --test_env=E2E_WORKSPACE (issue #55)." >&2
  return 1
}

# Prints the Bazel runfiles root when running under `bazel test` / `bazel
# run` (`RUNFILES_DIR` else `TEST_SRCDIR`). Fails outside Bazel. Replaces
# the bare `RUNFILES_DIR:-$TEST_SRCDIR` copy (unsafe under `set -u` when
# both are unset) repeated in drivers.
dx_runfiles_root() {
  if [[ -n "${RUNFILES_DIR:-}" ]]; then
    printf '%s\n' "$RUNFILES_DIR"
    return 0
  fi
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    printf '%s\n' "$TEST_SRCDIR"
    return 0
  fi
  echo "FAIL: no Bazel runfiles root (RUNFILES_DIR and TEST_SRCDIR are unset)" >&2
  return 1
}

# Prints the perf host label for benchmark fairness pins (issue #320
# portable route): maps `uname -s`/`uname -m` to the repo platform
# strings (`linux_x86_64`, `linux_arm64`, `macos_arm64`,
# `macos_x86_64`, `windows_x86_64`, `windows_arm64`). Fails fast on
# unknown OS/CPU instead of silently recording the seed host. Linux
# behavior unchanged: the seed host still reports `linux_x86_64`.
dx_perf_host() {
  local os arch
  case "$(uname -s)" in
    Linux) os="linux" ;;
    Darwin) os="macos" ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT) os="windows" ;;
    *) echo "FAIL: dx_perf_host: unsupported OS '$(uname -s)'" >&2; return 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="arm64" ;;
    *) echo "FAIL: dx_perf_host: unsupported CPU '$(uname -m)'" >&2; return 1 ;;
  esac
  printf '%s_%s\n' "$os" "$arch"
}

# Resolves a workspace-relative or `$(rootpath)` rel to an absolute path.
# Prefers standard `rlocation` (`_main/` Bzlmod plus `rules_dx/` legacy
# plus external-repo stripped layouts); falls back to manual `TEST_SRCDIR`
# / `RUNFILES_DIR` probing, `BUILD_WORKSPACE_DIRECTORY`, `bazel-bin`, git
# root, and cwd so `bazel test`, `bazel run`, and direct execution all work.
dx_resolve_runfile() {
  local rel="$1" stripped base cand
  stripped="$(printf '%s' "$rel" | sed -e 's|^\(\.\./\)*||')"
  if [[ "$rel" = /* ]] && [[ -e "$rel" ]]; then
    printf '%s\n' "$rel"
    return 0
  fi
  if declare -F rlocation >/dev/null 2>&1; then
    for cand in "_main/$rel" "_main/$stripped" "rules_dx/$rel" "rules_dx/$stripped" "$rel" "$stripped"; do
      if base="$(rlocation "$cand" 2>/dev/null)" && [[ -n "$base" && -e "$base" ]]; then
        printf '%s\n' "$base"
        return 0
      fi
    done
  fi
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    for base in "$TEST_SRCDIR/_main/$rel" "$TEST_SRCDIR/$rel" "$TEST_SRCDIR/$stripped" "$TEST_SRCDIR/rules_dx/$rel" "$TEST_SRCDIR/rules_dx/$stripped"; do
      if [[ -e "$base" ]]; then
        printf '%s\n' "$base"
        return 0
      fi
    done
  fi
  if [[ -n "${RUNFILES_DIR:-}" && "${RUNFILES_DIR:-}" != "${TEST_SRCDIR:-}" ]]; then
    for base in "$RUNFILES_DIR/_main/$rel" "$RUNFILES_DIR/$rel" "$RUNFILES_DIR/$stripped" "$RUNFILES_DIR/rules_dx/$rel" "$RUNFILES_DIR/rules_dx/$stripped"; do
      if [[ -e "$base" ]]; then
        printf '%s\n' "$base"
        return 0
      fi
    done
  fi
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    local found
    found="$(find "${TEST_SRCDIR:-/nonexistent}" -path "*$rel" -print -quit 2>/dev/null || true)"
    if [[ -n "$found" ]]; then
      printf '%s\n' "$found"
      return 0
    fi
    if [[ "$stripped" != "$rel" ]]; then
      found="$(find "${TEST_SRCDIR:-/nonexistent}" -path "*$stripped" -print -quit 2>/dev/null || true)"
      if [[ -n "$found" ]]; then
        printf '%s\n' "$found"
        return 0
      fi
    fi
  fi
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -e "$BUILD_WORKSPACE_DIRECTORY/$rel" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY/$rel"
    return 0
  fi
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -e "$BUILD_WORKSPACE_DIRECTORY/$stripped" ]]; then
    printf '%s\n' "$BUILD_WORKSPACE_DIRECTORY/$stripped"
    return 0
  fi
  for base in "$(pwd)/bazel-bin/$rel" "$(pwd)/bazel-bin/$stripped" "$(pwd)/$rel"; do
    if [[ -e "$base" ]]; then
      printf '%s\n' "$base"
      return 0
    fi
  done
  if [[ -e "$rel" ]]; then
    printf '%s\n' "$rel"
    return 0
  fi
  local ws
  if ws="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    if [[ -e "$ws/$rel" ]]; then
      printf '%s\n' "$ws/$rel"
      return 0
    fi
    if [[ -e "$ws/$stripped" ]]; then
      printf '%s\n' "$ws/$stripped"
      return 0
    fi
  fi
  return 1
}
