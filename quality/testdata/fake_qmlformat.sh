#!/usr/bin/env bash
# Fake qmlformat for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics `qmlformat --check` (exit 0 clean, exit 1 with unformatted
# workspace-relative paths when dirty) plus `qmlformat -i` (in-place
# rewrite, exit 0) over the BADFMT marker. Paths are reported relative
# to the working directory (the scratch root), like the real tool, so
# the backend re-anchors them. Real-tool behavior stays proven by
# `quality/tests/fixtures/qmlformat/`; this fake proves dispatch plus
# parser plus fix flow without Qt.
set -euo pipefail
mode=""
files=()
for arg in "$@"; do
  case "$arg" in
    --check) mode="check" ;;
    -i) mode="fix" ;;
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
    sed -i 's/BADFMT/fixed/g' "$f"
  fi
done
exit 0
