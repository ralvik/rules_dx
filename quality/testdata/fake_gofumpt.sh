#!/usr/bin/env bash
set -euo pipefail
is_fix=0
files=()
for arg in "$@"; do
  case "$arg" in
    -w) is_fix=1 ;;
    -d) ;;
    -*) ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
if [[ "$is_fix" == "1" ]]; then
  for f in "${files[@]}"; do
    if grep -q -F "BADFMT" "$f"; then
      sed 's/BADFMT/fixed/g' "$f" >"$f.dxtmp" && mv "$f.dxtmp" "$f"
    fi
  done
  exit 0
fi
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    printf -- "--- a/%s\n+++ b/%s\n@@ -1 +1 @@\n-BADFMT\n+fixed\n" "$f" "$f"
  fi
done
exit 0
