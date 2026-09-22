#!/usr/bin/env bash
# Shared workspace-root + runfiles + CI shell helpers.
#
# Single-sources the `BUILD_WORKSPACE_DIRECTORY || git rev-parse` workspace
# probe plus the multi-candidate runfiles probing (`RUNFILES_DIR`,
# `TEST_SRCDIR` layouts, `bazel-bin` fallbacks) repeated across every shell
# driver, plus the CI harness header (`set -euo pipefail` is per-file;
# pass/fail counters, scratch cleanup, portable realpath/hash/timing/sed)
# repeated across `tools/ci` drivers. Drivers load this file via the
# single-sourced bootstrap (`tools/sh/bootstrap.sh` `dx_bootstrap`, issue
# #654) so both direct execution and Bazel `run`/`test` layouts work with
# no per-file depth adjustment (`data = ["//tools/sh:lib"]` carries the
# bootstrap in the runfiles forest):
#
#   source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
#   dx_bootstrap "tools/sh/lib.sh"
#
# Rust binaries share `dx_process::workspace_start` plus standard `runfiles`
# `rlocation` (never `TEST_SRCDIR` in prod); see `cli/process/src/lib.rs`.
#
# Provides:
#   dx_workspace_root            prints the checkout root
#   dx_runfiles_root             prints the Bazel runfiles root
#                                (`RUNFILES_DIR` else `TEST_SRCDIR`)
#   dx_resolve_runfile <rel>     prints the absolute path for a
#                                workspace-relative or `rootpath` rel
#   dx_test_init                 resets the shared pass/fail counters
# ok [<msg>] / bad <msg> shared harness counters
#                                replaces per-file copies; `ok` echoes
#                                `ok: <msg>` when given a message for
#                                depcheck-style verbosity, silent otherwise)
#   dx_test_summary <name>       prints `<name>: $pass passed, $fail failed`
#                                and fails when fail != 0
#   dx_cd_workspace              cds to `dx_workspace_root` and sets
#                                `$workspace`
#   dx_mkscratch <var> [template]   portable `mktemp -d` with auto-cleanup on
# EXIT (replaces per-file
#                                `scratch=...; trap ...` copies)
#   dx_mktemp_file <var> [template] portable `mktemp` file with
#                                auto-cleanup on EXIT (e.g. execution-log
#                                outputs; issue #914)
#   dx_realpath <path>           portable realpath (`realpath` ->
# `readlink -f` -> python3,;
#                                `portable_realpath` stays as an alias)
#   dx_sha256_file <file>        portable sha256 hex (`sha256sum` ->
# `shasum -a 256` -> python3,)
#   dx_sha256_stdin              portable sha256 hex of stdin
#   dx_tree_sha256 <dir>         deterministic tree digest (sorted find +
# sha pipeline,)
#   dx_sha256_check <sums>       portable `sha256sum -c` /
#                                `shasum -a 256 -c` check
#   dx_now_secs                  portable monotonic stamp
#                                (`$EPOCHREALTIME` -> `date +%s.%N` ->
# `date +%s`,; `now_secs` stays
#                                as an alias)
#   dx_replace <expr> <file>     portable in-place sed (tmpfile + mv, no
# `sed -i`,)
# dx_expect_file <file> guard pin: file exists
#                                ok/bad with file context)
#   dx_expect_contains <file> <lit>...
#                                guard pin: fixed-string literals present
# (; snapshot stays for golden bytes)
#   dx_expect_absent <file> <lit>...
#                                guard pin: fixed-string literals absent
#   dx_python3                  prints `python3` else `python`
#                                (Windows `shell: bash` ships only `python`)
#   dx_hermetic_grep <args>     hermetic grep/sed-extract via
#                                `tools/sh/hermetic_grep.py` (pure-stdlib
#                                python3, identical Linux/macOS/Windows;
#                                issue #1006)
#   dx_grep_contains <file> <lit>...
#                                hermetic fixed-string present (all must match)
#   dx_grep_absent <file> <lit>...
#                                hermetic fixed-string absent (none may match)
#   dx_grep_re_contains <file> <re>...
#                                hermetic regex present
#   dx_grep_re_absent <file> <re>...
#                                hermetic regex absent
#   dx_tree_contains [opts] PATTERN... -- ROOTS...
#                                hermetic tree fixed-string present
#                                (opts: --include=G --exclude=SELF
#                                --allow=LIT --allow-path=SUB;
#                                DX_TREE_RE=1 for regex)
#   dx_tree_absent [opts] PATTERN... -- ROOTS...
#                                hermetic tree fixed-string absent
#   dx_context_contains FILE ANCHOR -A N PATTERN...
#                                hermetic `grep -A` window present
#                                (DX_CONTEXT_ANCHOR_RE=1, DX_CONTEXT_RE=1)
#   dx_context_absent FILE ANCHOR -A N PATTERN...
#                                hermetic `grep -A` window absent
#   dx_extract_quoted <file> <lit>
#                                prints first `"..."` value on the first
#                                line containing lit (hermetic
#                                `grep -F | sed 's/.*= "//...'`)
#   dx_extract_re <file> <re>   prints the first regex match
#                                (hermetic `grep -o -E | head -1`)
#   dx_bash_pin                 logs the bash version and fails closed
#                                below the 3.2+ floor (issue #1006)
#
#
# `dx_resolve_runfile` prefers the standard `runfiles.bash` `rlocation`
# when available and falls back to manual `TEST_SRCDIR` / `RUNFILES_DIR` /
# `bazel-bin` probing for `bazel run` invocations plus `git` / cwd for
# direct execution. Drivers must not reimplement workspace, runfiles,
# counter, scratch, realpath, hash, timing, sed, grep, or guard-pin
# probing; extend this file instead.
#
# Bash-only Linux harness: sourced by `sh_binary` /
# `sh_test` drivers carrying `target_compatible_with =
# ["@platforms//os:linux"]`. Bootstrap requires bash by design under issue
# (`BASH_SOURCE`, `[[`, arrays, `printf -v` plus the 5-way runfiles
# fallback never run under POSIX `sh`); portable-shell means OS-portable
# helper implementations (probes below), not a POSIX interpreter. Floor is
# bash 3.2+ with Linux execution (macOS/Windows run the same bash via
# `shell: bash` with no behavior change). Intentional lib-free exceptions:
# POSIX `#!/bin/sh` fixtures (no bootstrap, no constraint) plus deploy
# hermetic python-only runtime (bash + python3 + coreutils, no lib
# bootstrap) plus standalone renderers needing no
# workspace/runfiles.
# Portable forms: no bare `realpath`, `sha256sum`, `sed -i`,
# `cp -a`, or unguarded `$EPOCHREALTIME` here; every helper probes
# portably with no Linux behavior change. Guard maintenance owns shared
# helpers plus snapshot versus grep policy snapshot
# (`tools/sh/snapshot.sh` with UPDATE_EXPECT) is for byte-identical golden
# outputs, `dx_expect_*` fixed-string pins plus `tools/sh/guards.sh`
# `dx_guard_*` table rows are for doc/code contract
# sentences/symbols; `//tools/ci:shell_contract` owns the rule.
# Shellcheck/shfmt clean (`shfmt -i 2 -ci`, `.shellcheckrc` bash + all
# checks).
set -euo pipefail

# Bring `rlocation` into scope when running under Bazel. Non-fatal:
# direct execution has no runfiles tree, so fall back to manual probing.
if ! declare -F rlocation >/dev/null 2>&1; then
  _dx_runfiles_bash="bazel_tools/tools/bash/runfiles/runfiles.bash"
  # SC1090 single-sourced in `.shellcheckrc` (runfiles layouts exist
  # only under `bazel run` / `bazel test`, issue #319).
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

# Nested E2E removed, so `dx_e2e_workspace_root` plus the
# `E2E_WORKSPACE` override are deleted. Drivers use `dx_workspace_root`
# (BUILD_WORKSPACE_DIRECTORY else git top-level) with runfiles plus
# TEST_TMPDIR scratch under `bazel test //...`; no second Bazel download,
# no manual/local/exclusive/no-sandbox.

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

# Shared harness counters: replaces the per-file
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

# Cds to the checkout root and sets `$workspace`.
dx_cd_workspace() {
  workspace="$(dx_workspace_root)"
  cd "$workspace"
}

# Portable scratch dirs with EXIT auto-cleanup: replaces the
# per-file `scratch="$(mktemp -d)"; trap 'rm -rf "$scratch"' EXIT` copies.
# Single scratch policy (issues #323, #750): bare mktemp honors TMPDIR
# (Bazel TEST_TMPDIR plus runner RUNNER_TEMP flow through it); pass an
# explicit "${TEST_TMPDIR:-/tmp}/..." or "${TMPDIR:-/tmp}/..." template
# when the prefix must be pinned.
# Usage (no command substitution so the EXIT trap lands in the caller):
#   dx_mkscratch scratch
#   dx_mkscratch scratch "${TEST_TMPDIR:-/tmp}/depcheck.XXXXXX"
_DX_SCRATCHES=()
_DX_SCRATCH_TRAP_INSTALLED=0

_dx_cleanup_scratches() {
  local d
  if ((${#_DX_SCRATCHES[@]} == 0)); then
    return 0
  fi
  for d in "${_DX_SCRATCHES[@]}"; do
    if [[ -n "$d" && -e "$d" ]]; then
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
  _DX_SCRATCHES+=("$dir")
  if [[ "$_DX_SCRATCH_TRAP_INSTALLED" == "0" ]]; then
    trap '_dx_cleanup_scratches' EXIT
    _DX_SCRATCH_TRAP_INSTALLED=1
  fi
  printf -v "$var" '%s' "$dir"
}

# Portable scratch files with EXIT auto-cleanup (issue #914): the file
# counterpart of `dx_mkscratch` for harnesses needing a named temp file
# (e.g. `--execution_log_json_file` outputs). Bare `mktemp` honors
# TMPDIR (Bazel TEST_TMPDIR plus runner RUNNER_TEMP flow through it);
# pass an explicit "${TMPDIR:-${RUNNER_TEMP:-/tmp}}/..." template when
# the prefix must be pinned.
# Usage (no command substitution so the EXIT trap lands in the caller):
#   dx_mktemp_file exec_first "${TMPDIR:-${RUNNER_TEMP:-/tmp}}/quality_cache_exec_first.XXXXXX.json"
dx_mktemp_file() {
  local var="$1" template="${2:-}" file
  if [[ -n "$template" ]]; then
    file="$(mktemp "$template")"
  else
    file="$(mktemp)"
  fi
  _DX_SCRATCHES+=("$file")
  if [[ "$_DX_SCRATCH_TRAP_INSTALLED" == "0" ]]; then
    trap '_dx_cleanup_scratches' EXIT
    _DX_SCRATCH_TRAP_INSTALLED=1
  fi
  printf -v "$var" '%s' "$file"
}

# Portable realpath: GNU `realpath` is absent on macOS;
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

# Portable sha256 hex: GNU `sha256sum` is absent on macOS;
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

# Deterministic tree digest: NUL-delimited sorted find plus sha pipeline,
# matching the historical `find | sort | xargs sha256sum | sha256sum`
# bytes on Linux with a macOS `shasum` fallback and a python3 fallback
# that reproduces the same line format.
dx_tree_sha256() {
  local dir="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$dir" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum | sha256sum | cut -d' ' -f1)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$dir" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 shasum -a 256 | shasum -a 256 | cut -d' ' -f1)
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

# Portable `sha256sum -c` check.
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

# Portable monotonic stamp: `$EPOCHREALTIME` needs bash 5
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

# Portable in-place sed: GNU `sed -i -e` breaks on macOS BSD
# sed; the tmpfile form works on both.
dx_replace() {
  local expr="$1" file="$2" tmp
  tmp="$file.tmp"
  sed -e "$expr" "$file" >"$tmp" && mv "$tmp" "$file"
}

# Hermetic grep plus field extraction (issue #1006): BSD `grep` lacks
# GNU `--include`/`--exclude-dir`, `-A` separators plus `-o` quirks
# diverge, and `sed` BRE drifts, so the divergent single-file/tree/
# context/extract operations go through `tools/sh/hermetic_grep.py`
# (pure-stdlib python3, identical on Linux/macOS/Windows) instead of
# host `grep`/`sed`. Plain POSIX `grep -q -F/-E` single-file pins stay
# allowed (variance-free), but drivers should prefer these wrappers so
# qualification never branches on host grep. `dx_python3` probes
# `python3` then `python` (Windows `shell: bash` ships only `python`).
dx_python3() {
  if command -v python3 >/dev/null 2>&1; then
    printf '%s\n' "python3"
  else
    printf '%s\n' "python"
  fi
}

dx_hermetic_grep_py() {
  local cand
  if [[ -n "${RUNFILES_DIR:-}" ]]; then
    for cand in "$RUNFILES_DIR/_main/tools/sh/hermetic_grep.py" "$RUNFILES_DIR/tools/sh/hermetic_grep.py"; do
      if [[ -f "$cand" ]]; then
        printf '%s\n' "$cand"
        return 0
      fi
    done
  fi
  if [[ -n "${TEST_SRCDIR:-}" ]]; then
    for cand in "$TEST_SRCDIR/_main/tools/sh/hermetic_grep.py" "$TEST_SRCDIR/tools/sh/hermetic_grep.py"; do
      if [[ -f "$cand" ]]; then
        printf '%s\n' "$cand"
        return 0
      fi
    done
  fi
  if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" && -f "${BUILD_WORKSPACE_DIRECTORY}/tools/sh/hermetic_grep.py" ]]; then
    printf '%s\n' "${BUILD_WORKSPACE_DIRECTORY}/tools/sh/hermetic_grep.py"
    return 0
  fi
  local top
  if top="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    if [[ -f "$top/tools/sh/hermetic_grep.py" ]]; then
      printf '%s\n' "$top/tools/sh/hermetic_grep.py"
      return 0
    fi
  fi
  if [[ -f "tools/sh/hermetic_grep.py" ]]; then
    printf '%s\n' "tools/sh/hermetic_grep.py"
    return 0
  fi
  echo "dx_hermetic_grep: cannot locate tools/sh/hermetic_grep.py" >&2
  return 1
}

dx_hermetic_grep() {
  local py
  py="$(dx_hermetic_grep_py)" || return 1
  "$(dx_python3)" "$py" "$@"
}

# Single-file fixed-string pins (hermetic `grep -q -F`).
dx_grep_contains() {
  local file="$1"
  shift
  dx_hermetic_grep contains "$file" --fixed -- "$@"
}

dx_grep_absent() {
  local file="$1"
  shift
  dx_hermetic_grep absent "$file" --fixed -- "$@"
}

# Single-file regex pins (hermetic `grep -q -E`).
dx_grep_re_contains() {
  local file="$1"
  shift
  dx_hermetic_grep contains "$file" --re -- "$@"
}

dx_grep_re_absent() {
  local file="$1"
  shift
  dx_hermetic_grep absent "$file" --re -- "$@"
}

# Repo-tree pins (hermetic `grep -rn --include/--exclude` plus the
# `| grep -v` allow chains; skips `bazel-*` plus `.git` like the guards).
# Usage: dx_tree_contains [--include=G]... [--exclude=SELF]...
#   [--allow=LIT]... [--allow-path=SUB]... PATTERN... -- ROOTS...
# Patterns are fixed strings unless DX_TREE_RE=1.
dx_tree_contains() {
  local includes=() excludes=() allows=() allow_paths=() patterns=() roots=()
  local in_roots=0 arg
  for arg in "$@"; do
    if [[ "$arg" == "--" && "$in_roots" == "0" ]]; then
      in_roots=1
      continue
    fi
    if [[ "$in_roots" == "1" ]]; then
      roots+=("$arg")
      continue
    fi
    case "$arg" in
    --include=*) includes+=(--include "${arg#--include=}") ;;
    --exclude=*) excludes+=(--exclude "${arg#--exclude=}") ;;
    --allow=*) allows+=(--allow "${arg#--allow=}") ;;
    --allow-path=*) allow_paths+=(--allow-path "${arg#--allow-path=}") ;;
    *) patterns+=("$arg") ;;
    esac
  done
  if [[ "${#roots[@]}" == "0" ]]; then
    roots=(.)
  fi
  if [[ "${DX_TREE_RE:-0}" == "1" ]]; then
    dx_hermetic_grep tree-contains --re "${includes[@]}" "${excludes[@]}" "${allows[@]}" "${allow_paths[@]}" --roots "${roots[@]}" -- "${patterns[@]}"
  else
    dx_hermetic_grep tree-contains --fixed "${includes[@]}" "${excludes[@]}" "${allows[@]}" "${allow_paths[@]}" --roots "${roots[@]}" -- "${patterns[@]}"
  fi
}

dx_tree_absent() {
  local includes=() excludes=() allows=() allow_paths=() patterns=() roots=()
  local in_roots=0 arg
  for arg in "$@"; do
    if [[ "$arg" == "--" && "$in_roots" == "0" ]]; then
      in_roots=1
      continue
    fi
    if [[ "$in_roots" == "1" ]]; then
      roots+=("$arg")
      continue
    fi
    case "$arg" in
    --include=*) includes+=(--include "${arg#--include=}") ;;
    --exclude=*) excludes+=(--exclude "${arg#--exclude=}") ;;
    --allow=*) allows+=(--allow "${arg#--allow=}") ;;
    --allow-path=*) allow_paths+=(--allow-path "${arg#--allow-path=}") ;;
    *) patterns+=("$arg") ;;
    esac
  done
  if [[ "${#roots[@]}" == "0" ]]; then
    roots=(.)
  fi
  if [[ "${DX_TREE_RE:-0}" == "1" ]]; then
    dx_hermetic_grep tree-absent --re "${includes[@]}" "${excludes[@]}" "${allows[@]}" "${allow_paths[@]}" --roots "${roots[@]}" -- "${patterns[@]}"
  else
    dx_hermetic_grep tree-absent --fixed "${includes[@]}" "${excludes[@]}" "${allows[@]}" "${allow_paths[@]}" --roots "${roots[@]}" -- "${patterns[@]}"
  fi
}

# Context pins (hermetic `grep -A N -e ANCHOR FILE | grep -q PATTERN`).
# Usage: dx_context_contains FILE ANCHOR -A N PATTERN...
# Anchor is fixed unless DX_CONTEXT_ANCHOR_RE=1; patterns are fixed
# unless DX_CONTEXT_RE=1.
dx_context_contains() {
  local file="$1" anchor="$2"
  shift 2
  local after="" patterns=() arg
  for arg in "$@"; do
    case "$arg" in
    -A) continue ;;
    [0-9]*) after="$arg" ;;
    *) patterns+=("$arg") ;;
    esac
  done
  # Allow `-A N` as two words: re-parse when $3 was `-A`.
  if [[ -z "$after" ]]; then
    echo "dx_context_contains: want FILE ANCHOR -A N PATTERN..." >&2
    return 2
  fi
  local anchor_flag="--anchor-fixed" mode_flag="--fixed"
  [[ "${DX_CONTEXT_ANCHOR_RE:-0}" == "1" ]] && anchor_flag="--anchor-re"
  [[ "${DX_CONTEXT_RE:-0}" == "1" ]] && mode_flag="--re"
  # shellcheck disable=SC2086
  dx_hermetic_grep context-contains "$file" "$anchor" -A "$after" $anchor_flag $mode_flag -- "${patterns[@]}"
}

dx_context_absent() {
  local file="$1" anchor="$2"
  shift 2
  local after="" patterns=() arg
  for arg in "$@"; do
    case "$arg" in
    -A) continue ;;
    [0-9]*) after="$arg" ;;
    *) patterns+=("$arg") ;;
    esac
  done
  if [[ -z "$after" ]]; then
    echo "dx_context_absent: want FILE ANCHOR -A N PATTERN..." >&2
    return 2
  fi
  local anchor_flag="--anchor-fixed" mode_flag="--fixed"
  [[ "${DX_CONTEXT_ANCHOR_RE:-0}" == "1" ]] && anchor_flag="--anchor-re"
  [[ "${DX_CONTEXT_RE:-0}" == "1" ]] && mode_flag="--re"
  # shellcheck disable=SC2086
  dx_hermetic_grep context-absent "$file" "$anchor" -A "$after" $anchor_flag $mode_flag -- "${patterns[@]}"
}

# Field extraction (hermetic `grep -F ... | sed 's/.*= "//; s/";.*//'`).
dx_extract_quoted() {
  dx_hermetic_grep extract-quoted "$@"
}

# First-regex-match extraction (hermetic `grep -o -E -e RE | head -1`).
dx_extract_re() {
  dx_hermetic_grep extract-re "$@"
}

# Bash floor pin (issue #1006): harness floor stays bash 3.2+ with Linux
# execution; macOS/Windows run the same bash with no behavior change.
# Fails closed below the floor; drivers log the version for provenance.
dx_bash_pin() {
  local major="${BASH_VERSINFO[0]:-0}" minor="${BASH_VERSINFO[1]:-0}"
  echo "bash ${BASH_VERSION:-unknown} (floor 3.2+, issue #1006)"
  if [[ "$major" -gt 3 ]] || [[ "$major" == "3" && "$minor" -ge 2 ]]; then
    return 0
  fi
  echo "FAIL: bash floor 3.2+ required, found ${BASH_VERSION:-unknown}" >&2
  return 1
}

# Guard-maintenance pins: fixed-string contract checks so
# `tools/ci` guards share one grep shape instead of brittle per-file
# `grep -q -F` copies. Snapshot (`tools/sh/snapshot.sh` with UPDATE_EXPECT)
# stays for byte-identical golden outputs; these helpers are for doc/code
# sentence/symbol pins (fail-closed, no UPDATE_EXPECT). Each reports one
# `ok`/`bad` with file:pattern context and always returns 0 so the harness
# collects every failure before `dx_test_summary`.
dx_expect_file() {
  local file="$1"
  if [[ -f "$file" ]]; then
    ok
  else
    bad "missing file $file"
  fi
  return 0
}

dx_expect_contains() {
  local file="$1"
  shift
  local missing="" lit
  if [[ ! -f "$file" ]]; then
    bad "missing file $file (want literals: $*)"
    return 0
  fi
  for lit in "$@"; do
    if ! grep -q -F -e "$lit" -- "$file"; then
      missing="$missing [$lit]"
    fi
  done
  if [[ -z "$missing" ]]; then
    ok
  else
    bad "$file missing literals:$missing"
  fi
  return 0
}

dx_expect_absent() {
  local file="$1"
  shift
  local present="" lit
  if [[ ! -f "$file" ]]; then
    bad "missing file $file (want absence of: $*)"
    return 0
  fi
  for lit in "$@"; do
    if grep -q -F -e "$lit" -- "$file"; then
      present="$present [$lit]"
    fi
  done
  if [[ -z "$present" ]]; then
    ok
  else
    bad "$file must not contain:$present"
  fi
  return 0
}

# Default LLVM_PROFILE_FILE containment (issue #953): instrumented Rust
# binaries (e.g. `bazel coverage`-built helpers executed directly from the
# workspace root) spill `default_%m_%p.profraw` into CWD when the variable
# is unset. Default it here to an auto-cleaned scratch dir so every driver
# sourcing this library is contained even before its explicit per-harness
# export; an explicit `export LLVM_PROFILE_FILE=...` later still wins.
# Non-instrumented binaries ignore it; `bazel test` coverage collection is
# unaffected (Bazel sandboxes test env, direct runs are what spill).
if [[ -z "${LLVM_PROFILE_FILE:-}" ]]; then
  dx_mkscratch _DX_PROFRAW_DIR
  export LLVM_PROFILE_FILE="$_DX_PROFRAW_DIR/profraw_%m_%p.profraw"
fi
