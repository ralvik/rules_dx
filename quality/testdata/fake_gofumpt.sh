#!/usr/bin/env bash
# Fake gofumpt for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics `gofumpt -d` (unified diff headers on stdout when dirty, empty
# stdout when clean, exit 0 either way like `gofmt -d`) plus `gofumpt -w`
# (in-place rewrite, exit 0) over the BADFMT marker. Real-tool behavior
# stays proven by `go/tests/fixtures/gofumpt/` (`check_dirty.diff`
# modeled on the pinned `-d` output); this fake proves the runner
# dispatch plus parser plus fix flow without a Go toolchain.
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
      sed -i 's/BADFMT/fixed/g' "$f"
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
