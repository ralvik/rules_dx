#!/usr/bin/env bash
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
    sed 's/BADFMT/fixed/g' "$f" >"$f.dxtmp" && mv "$f.dxtmp" "$f"
  fi
done
exit 0
