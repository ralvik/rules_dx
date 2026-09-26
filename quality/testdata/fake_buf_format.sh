#!/usr/bin/env bash
set -euo pipefail
is_diff=0
is_write=0
files=()
for arg in "$@"; do
  case "$arg" in
    --diff) is_diff=1 ;;
    --write) is_write=1 ;;
    format|--exit-code|lint|--error-format=json) ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
if [[ "$is_diff" == "1" ]]; then
  dirty=0
  for f in "${files[@]}"; do
    if grep -q -F "BADFMT" "$f"; then
      printf -- "--- a/%s\n+++ b/%s\n@@ -1 +1 @@\n-BADFMT\n+fixed\n" "$f" "$f"
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
