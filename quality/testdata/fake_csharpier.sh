#!/usr/bin/env bash
# Fake CSharpier for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics `csharpier check` (exit 0 clean, exit 1 with unformatted
# workspace-relative paths when dirty) plus `csharpier format` (in-place
# rewrite, exit 0) over the BADFMT marker. Paths are reported relative to
# the working directory (the scratch root), like the real tool, so the
# backend re-anchors them. Real-tool behavior stays proven by
# `csharp/tests/fixtures/csharpier/`; this fake proves dispatch plus
# parser plus fix flow without .NET.
set -euo pipefail
mode=""
files=()
skip_next=0
for arg in "$@"; do
  if [[ "$skip_next" == "1" ]]; then
    skip_next=0
    continue
  fi
  case "$arg" in
    check) mode="check" ;;
    format) mode="format" ;;
    --config-path) skip_next=1 ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
relpath() {
  python3 -c 'import os,sys; print(os.path.relpath(sys.argv[1], sys.argv[2]))' "$1" "$PWD"
}
if [[ "$mode" == "check" ]]; then
  dirty=0
  for f in "${files[@]}"; do
    if grep -q -F "BADFMT" "$f"; then
      relpath "$f"
      dirty=1
    fi
  done
  if [[ "$dirty" == "1" ]]; then exit 1; fi
  exit 0
fi
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    sed 's/BADFMT/fixed/g' "$f" >"$f.dxtmp" && mv "$f.dxtmp" "$f"
  fi
done
exit 0
