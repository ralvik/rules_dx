#!/usr/bin/env bash
# Fake Fantomas for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics `fantomas check --json` (exit 0 all unchanged, exit 99 with
# `needs-formatting` files) plus bare in-place format (rewrite, exit 0)
# over the BADFMT marker. Paths are workspace-relative (relative to the
# scratch root working directory), like the real tool. Real-tool behavior
# stays proven by `fsharp/tests/fixtures/fantomas/`; this fake proves
# dispatch plus parser plus fix flow without .NET.
set -euo pipefail
is_check=0
files=()
for arg in "$@"; do
  case "$arg" in
    check) is_check=1 ;;
    --json) ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
relpath() {
  python3 -c 'import os,sys; print(os.path.relpath(sys.argv[1], sys.argv[2]))' "$1" "$PWD"
}
if [[ "$is_check" == "1" ]]; then
  entries=()
  dirty=0
  for f in "${files[@]}"; do
    rel="$(relpath "$f")"
    if grep -q -F "BADFMT" "$f"; then
      entries+=("{\"path\": \"$rel\", \"status\": \"needs-formatting\"}")
      dirty=1
    else
      entries+=("{\"path\": \"$rel\", \"status\": \"unchanged\"}")
    fi
  done
  joined=""
  for e in "${entries[@]}"; do
    if [[ -z "$joined" ]]; then joined="$e"; else joined="$joined, $e"; fi
  done
  printf '{"files": [%s]}\n' "$joined"
  if [[ "$dirty" == "1" ]]; then exit 99; fi
  exit 0
fi
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    sed 's/BADFMT/fixed/g' "$f" >"$f.dxtmp" && mv "$f.dxtmp" "$f"
  fi
done
exit 0
