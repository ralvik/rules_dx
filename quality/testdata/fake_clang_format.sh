#!/usr/bin/env bash
# Fake clang-format for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics the pinned clang-format check shape (exit 0 clean, exit 1 with
# unified diff headers when dirty) plus `-i` in-place fix (rewrite,
# exit 0) over the BADFMT marker. Real-tool behavior stays proven by
# `cc/tests/fixtures/clang_format/` (`check_dirty.diff` modeled on the
# pinned check output); this fake proves the runner dispatch plus parser
# plus fix flow without an LLVM toolchain.
set -euo pipefail
is_fix=0
files=()
for arg in "$@"; do
  case "$arg" in
    -i) is_fix=1 ;;
    --dry-run|--Werror) ;;
    --style=file:*) ;;
    -*) ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
if [[ "$is_fix" == "1" ]]; then
  for f in "${files[@]}"; do
    if grep -q -F "BADFMT" "$f"; then
      sed -i 's/BADFMT/fixed/g' "$f"
    fi
  done
  exit 0
fi
dirty=0
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    printf -- "--- a/%s\n+++ b/%s\n@@ -1 +1 @@\n-BADFMT\n+fixed\n" "$f" "$f"
    dirty=1
  fi
done
if [[ "$dirty" == "1" ]]; then exit 1; fi
exit 0
