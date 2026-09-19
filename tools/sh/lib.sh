#!/usr/bin/env bash
# Shared workspace-root + runfiles + CI shell helpers (issues #319, #323).
#
# Single-sources the `BUILD_WORKSPACE_DIRECTORY || git rev-parse` workspace
# probe plus the multi-candidate runfiles probing (`RUNFILES_DIR`,
# `TEST_SRCDIR` layouts, `bazel-bin` fallbacks) repeated across every shell
# driver, plus the CI harness header (`set -euo pipefail` is per-file;
# pass/fail counters, scratch cleanup, portable realpath/hash/timing/sed)
# repeated across `tools/ci` and `perf` drivers. Drivers source this file
# via a runfiles-first bootstrap so both direct execution and Bazel
# `run`/`test` layouts work (`data = ["//tools/sh:lib"]` carries it in the
# runfiles forest):
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
#   dx_test_init                 resets the shared pass/fail counters
#   ok [<msg>] / bad <msg>       shared harness counters (issue #323:
#                                replaces per-file copies; `ok` echoes
#                                `ok: <msg>` when given a message for
#                                depcheck-style verbosity, silent otherwise)
#   dx_test_summary <name>       prints `<name>: $pass passed, $fail failed`
#                                and fails when fail != 0
#   dx_cd_workspace              cds to `dx_workspace_root` and sets
#                                `$workspace`
#   dx_mkscratch <var> [template]   portable `mktemp -d` with auto-cleanup on
#                                EXIT (issue #323: replaces per-file
#                                `scratch=...; trap ...` copies)
#   dx_realpath <path>           portable realpath (`realpath` ->
#                                `readlink -f` -> python3, issue #323;
#                                `portable_realpath` stays as an alias)
#   dx_sha256_file <file>        portable sha256 hex (`sha256sum` ->
#                                `shasum -a 256` -> python3, issue #323)
#   dx_sha256_stdin              portable sha256 hex of stdin
#   dx_tree_sha256 <dir>         deterministic tree digest (sorted find +
#                                sha pipeline, issue #323)
#   dx_sha256_check <sums>       portable `sha256sum -c` /
#                                `shasum -a 256 -c` check
#   dx_now_secs                  portable monotonic stamp
#                                (`$EPOCHREALTIME` -> `date +%s.%N` ->
#                                `date +%s`, issue #323; `now_secs` stays
#                                as an alias)
#   dx_replace <expr> <file>     portable in-place sed (tmpfile + mv, no
#                                `sed -i`, issue #323)
#
# `dx_resolve_runfile` prefers the standard `runfiles.bash` `rlocation`
# when available and falls back to manual `TEST_SRCDIR` / `RUNFILES_DIR` /
# `bazel-bin` probing for `bazel run` invocations plus `git` / cwd for
# direct execution. Drivers must not reimplement workspace, runfiles,
# counter, scratch, realpath, hash, timing, or sed probing; extend this
# file instead.
#
# Bash-only Linux harness (issue #299): sourced by `sh_binary` / `sh_test`
# drivers carrying `target_compatible_with = ["@platforms//os:linux"]`.
# Portable forms (issue #323): no bare `realpath`, `sha256sum`, `sed -i`,
# `cp -a`, or unguarded `$EPOCHREALTIME` here; every helper probes
# portably with no Linux behavior change. Shellcheck/shfmt clean
# (`shfmt -i 2 -ci`, `.shellcheckrc` bash + all checks).
set -euo pipefail

# Bring `rlocation` into scope when running under Bazel. Non-fatal:
# direct execution has no runfiles tree, so fall back to manual probing.
if ! declare -F rlocation >/dev/null 2>&1; then
  _dx_runfiles_bash="bazel_tools/tools/bash/runfiles/runfiles.bash"
  # shellcheck disable=SC1090
  source "${RUNFILES_DIR:-/dev/null}/$_dx_runfiles_bash" 2>/dev/null ||
    source "$(grep -sm1 "^$_dx_runfiles_bash " "${RUNFILES_MANIFEST_FILE:-/dev/null}" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null ||
    source "$0.runfiles/$_dx_runfiles_bash" 2>/dev/null ||
    source "$(grep -sm1 "^$_dx_runfiles_bash " "$0.runfiles_manifest" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null ||
    source "$(grep -sm1 "^$_dx_runfiles_bash " "$0.exe.runfiles_manifest" 2>/dev/null | cut -f2- -d' ')" 2>/dev/null ||
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
    MINGW* | MSYS* | CYGWIN* | Windows_NT) os="windows" ;;
    *)
      echo "FAIL: dx_perf_host: unsupported OS '$(uname -s)'" >&2
      return 1
      ;;
  esac
  case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    aarch64 | arm64) arch="arm64" ;;
    *)
      echo "FAIL: dx_perf_host: unsupported CPU '$(uname -m)'" >&2
      return 1
      ;;
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

# Shared harness counters (issue #323): replaces the per-file
# `pass=0; fail=0; ok() ...; bad() ...` copies. `ok` echoes `ok: <msg>`
# when given a message (depcheck verbosity) and stays silent otherwise.
dx_test_init() {
  pass=0
  fail=0
}

ok() {
  pass=$((pass + 1))
  if [[ $# -gt 0 ]]; then
    echo "ok: $1"
  fi
}

bad() {
  echo "FAIL: $1" >&2
  fail=$((fail + 1))
}

# Prints `<name>: $pass passed, $fail failed`; fails when fail != 0.
dx_test_summary() {
  local name="$1"
  echo "$name: $pass passed, $fail failed"
  [[ "$fail" == "0" ]]
}

# Cds to the checkout root and sets `$workspace` (issue #323).
dx_cd_workspace() {
  workspace="$(dx_workspace_root)"
  cd "$workspace"
}

# Portable scratch dirs with EXIT auto-cleanup (issue #323): replaces the
# per-file `scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT` copies.
# Usage (no command substitution so the EXIT trap lands in the caller):
#   dx_mkscratch scratch
#   dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/depcheck.XXXXXX"
_DX_SCRATCHES=""
_DX_SCRATCH_TRAP_INSTALLED=0

_dx_cleanup_scratches() {
  local d
  # shellcheck disable=SC2086
  for d in $_DX_SCRATCHES; do
    if [[ -n "$d" && -d "$d" ]]; then
      rm -rf "$d"
    fi
  done
}

dx_mkscratch() {
  local var="$1" template="${2:-}" dir
  if [[ -n "$template" ]]; then
    dir="$(mktemp -d "$template")"
  else
    dir="$(mktemp -d)"
  fi
  _DX_SCRATCHES="$_DX_SCRATCHES $dir"
  if [[ "$_DX_SCRATCH_TRAP_INSTALLED" == "0" ]]; then
    trap '_dx_cleanup_scratches' EXIT
    _DX_SCRATCH_TRAP_INSTALLED=1
  fi
  printf -v "$var" '%s' "$dir"
}

# Portable realpath (issue #323): GNU `realpath` is absent on macOS;
# `readlink -f` covers some platforms, python3 covers the rest.
dx_realpath() {
  if command -v realpath >/dev/null 2>&1; then
    realpath "$1"
  elif command -v readlink >/dev/null 2>&1 && readlink -f "$1" >/dev/null 2>&1; then
    readlink -f "$1"
  else
    python3 -c 'import os,sys; print(os.path.realpath(sys.argv[1]))' "$1"
  fi
}

portable_realpath() {
  dx_realpath "$@"
}

# Portable sha256 hex (issue #323): GNU `sha256sum` is absent on macOS;
# `shasum -a 256` is the portable fallback, python3 covers the rest.
dx_sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | cut -d' ' -f1
  else
    python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$file"
  fi
}

dx_sha256_stdin() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum | cut -d' ' -f1
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | cut -d' ' -f1
  else
    python3 -c 'import hashlib,sys; print(hashlib.sha256(sys.stdin.buffer.read()).hexdigest())'
  fi
}

# Deterministic tree digest (issue #323): sorted find plus sha pipeline,
# matching the historical `find | sort | xargs sha256sum | sha256sum`
# bytes on Linux with a macOS `shasum` fallback and a python3 fallback
# that reproduces the same line format.
dx_tree_sha256() {
  local dir="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dir" && find . -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$dir" && find . -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256 | cut -d' ' -f1)
  else
    python3 -c '
import hashlib, os, sys
root = sys.argv[1]
names = []
for base, _dirs, files in os.walk(root):
  for f in files:
    rel = os.path.relpath(os.path.join(base, f), root)
    names.append(rel)
names.sort()
lines = []
for rel in names:
  with open(os.path.join(root, rel), "rb") as fh:
    lines.append(hashlib.sha256(fh.read()).hexdigest() + "  " + rel)
print(hashlib.sha256(("\n".join(lines) + "\n").encode()).hexdigest())
' "$dir"
  fi
}

# Portable `sha256sum -c` check (issue #323).
dx_sha256_check() {
  local sums="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c "$sums"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c "$sums"
  else
    python3 -c '
import hashlib, sys
ok = True
for line in open(sys.argv[1]):
  line = line.rstrip("\n")
  if not line:
    continue
  want, _, path = line.partition("  ")
  if not want or not path:
    want, _, path = line.partition(" *")
  try:
    got = hashlib.sha256(open(path, "rb").read()).hexdigest()
  except OSError:
    print(path + ": FAILED open") 
    ok = False
    continue
  if got != want:
    print(path + ": FAILED") 
    ok = False
sys.exit(0 if ok else 1)
' "$sums"
  fi
}

# Portable monotonic stamp (issue #323): `$EPOCHREALTIME` needs bash 5
# (macOS ships bash 3); fall back to `date +%s.%N`, then whole seconds.
dx_now_secs() {
  if [[ -n "${EPOCHREALTIME:-}" ]]; then
    printf '%s' "${EPOCHREALTIME}"
  elif date +%s.%N >/dev/null 2>&1; then
    date +%s.%N
  else
    date +%s
  fi
}

now_secs() {
  dx_now_secs
}

# Portable in-place sed (issue #323): GNU `sed -i -e` breaks on macOS BSD
# sed; the tmpfile form works on both.
dx_replace() {
  local expr="$1" file="$2" tmp
  tmp="$file.tmp"
  sed -e "$expr" "$file" >"$tmp" && mv "$tmp" "$file"
}
