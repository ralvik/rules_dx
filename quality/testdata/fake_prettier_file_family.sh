#!/usr/bin/env bash
set -euo pipefail
is_fix=0
files=()
for arg in "$@"; do
  case "$arg" in
    --write) is_fix=1 ;;
    --check|--no-config|--no-editorconfig) ;;
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
dirty=0
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    rel="${f#"$PWD"/}"
    if [[ "$rel" == "$f" ]]; then
      rel="${f##*/}"
    fi
    printf -- "[warn] %s\n" "$rel" >&2
    dirty=1
  fi
done
if [[ "$dirty" == "1" ]]; then exit 1; fi
exit 0
