#!/usr/bin/env bash
# Rules-lint comparison synthetic-tree harness (issue #86).
#
# Generates a deterministic synthetic large tree (many files, mixed
# clean/dirty) for the quality-only rules_lint comparison, then emits a
# JSON report skeleton with fairness pins. Report-not-gate: exit 0 on
# success, no parity claim, no gate — numbers publish first, gates follow
# only if/when noise is understood (per docs/tools/rules_lint-comparison.md).
#
# Usage:
#   bazel run //perf:rules_lint_comparison -- [--files 200] [--dirty-pct 10] [--seed 86] [--out DIR]
#
# Determinism: same (files, dirty-pct, seed) always yields the same
# tree_sha256 (sorted find + sha256). Dirty files carry known lint
# triggers (markdown trailing whitespace, Starlark `x=1` unformatted)
# so clean vs dirty runs are comparable across harnesses.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../tools/sh/lib.sh"

files=200
dirty_pct=10
seed=86
out=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --files=*) files="${1#--files=}" ; shift ;;
    --files) files="$2"; shift 2 ;;
    --dirty-pct=*) dirty_pct="${1#--dirty-pct=}"; shift ;;
    --dirty-pct) dirty_pct="$2"; shift 2 ;;
    --seed=*) seed="${1#--seed=}"; shift ;;
    --seed) seed="$2"; shift 2 ;;
    --out=*) out="${1#--out=}"; shift ;;
    --out) out="$2"; shift 2 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) echo "rules_lint_comparison: unknown arg '$1'" >&2; exit 2 ;;
  esac
done

workspace="$(dx_workspace_root 2>/dev/null || echo unknown)"
if [[ -f "$workspace/.bazelversion" ]]; then
  bazel_version="$(cat "$workspace/.bazelversion")"
else
  bazel_version="unknown"
fi
# Issue #320 portable route: record the actual host instead of the
# hard-coded seed label; unknown OS/CPU fails fast in dx_perf_host.
host="$(dx_perf_host)"
rules_lint_pin="v2.8.0"

scratch=""
if [[ -z "$out" ]]; then
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  out="$scratch/synth"
fi
mkdir -p "$out"

dirty_count=$(( files * dirty_pct / 100 ))

# Portable monotonic stamp (issue #299): `$EPOCHREALTIME` needs bash 5
# (macOS ships bash 3); fall back to `date +%s.%N`, then whole seconds.
now_secs() {
  if [[ -n "${EPOCHREALTIME:-}" ]]; then
    printf '%s' "${EPOCHREALTIME}"
  elif date +%s.%N >/dev/null 2>&1; then
    date +%s.%N
  else
    date +%s
  fi
}
start="$(now_secs)"
i=1
while [[ "$i" -le "$files" ]]; do
  dirty=0
  if [[ "$i" -le "$dirty_count" ]]; then dirty=1; fi
  if (( i % 2 == 1 )); then
    f="$out/doc_$(printf '%04d' "$i").md"
    if [[ "$dirty" == "1" ]]; then
      printf '# Synthetic doc %d (seed %s)\n\nParagraph %d with trailing whitespace.   \n' "$i" "$seed" "$i" > "$f"
    else
      printf '# Synthetic doc %d (seed %s)\n\nParagraph %d with clean text.\n' "$i" "$seed" "$i" > "$f"
    fi
  else
    f="$out/target_$(printf '%04d' "$i").bzl"
    if [[ "$dirty" == "1" ]]; then
      printf 'CONST_%d=1\n' "$i" > "$f"
    else
      printf 'CONST_%d = 1\n' "$i" > "$f"
    fi
  fi
  i=$((i + 1))
done
end="$(now_secs)"
gen_ms="$(awk "BEGIN {print ($end - $start) * 1000.0}")"

# Portable tree digest (issue #299): GNU `sha256sum` is absent on macOS;
# `shasum -a 256` is the portable fallback. Linux behavior unchanged.
if command -v sha256sum >/dev/null 2>&1; then
  tree_sha256="$(cd "$out" && find . -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)"
else
  tree_sha256="$(cd "$out" && find . -type f | LC_ALL=C sort | xargs shasum -a 256 | shasum -a 256 | cut -d' ' -f1)"
fi

python3 -c '
import json, sys
print(json.dumps({
  "benchmark": "rules_lint_comparison_synth",
  "files": int(sys.argv[1]),
  "dirty_files": int(sys.argv[2]),
  "dirty_pct": int(sys.argv[3]),
  "seed": int(sys.argv[4]),
  "tree_sha256": sys.argv[5],
  "generation_ms": float(sys.argv[6]),
  "host": sys.argv[7],
  "bazel_version": sys.argv[8],
  "rules_lint_pin": sys.argv[9],
  "gate": False,
  "claim": "none",
  "note": "synthetic-tree harness, report-not-gate; no parity claim until measured runs land",
}, indent=2))
' "$files" "$dirty_count" "$dirty_pct" "$seed" "$tree_sha256" "$gen_ms" "$host" "$bazel_version" "$rules_lint_pin"
