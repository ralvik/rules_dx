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

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel 2>/dev/null || echo unknown)"
fi
if [[ -f "$workspace/.bazelversion" ]]; then
  bazel_version="$(cat "$workspace/.bazelversion")"
else
  bazel_version="unknown"
fi
host="linux_x86_64"
rules_lint_pin="v2.8.0"

scratch=""
if [[ -z "$out" ]]; then
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  out="$scratch/synth"
fi
mkdir -p "$out"

dirty_count=$(( files * dirty_pct / 100 ))

start="$EPOCHREALTIME"
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
end="$EPOCHREALTIME"
gen_ms="$(awk "BEGIN {print ($end - $start) * 1000.0}")"

tree_sha256="$(cd "$out" && find . -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)"

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
