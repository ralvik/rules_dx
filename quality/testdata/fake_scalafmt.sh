#!/usr/bin/env bash
# Fake Scalafmt for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics `scalafmt --check` (exit 0 clean, exit 1 with unified diff headers
# when dirty) plus in-place fix (rewrite, exit 0) over the BADFMT marker.
# Real-tool behavior stays proven by `scala/tests/fixtures/scalafmt/`
# (`check_dirty.diff` modeled on real `--check` output); this fake proves
# the runner dispatch plus parser plus fix flow without a JVM.
set -euo pipefail
is_check=0
files=()
skip_next=0
for arg in "$@"; do
  if [[ "$skip_next" == "1" ]]; then
    skip_next=0
    continue
  fi
  case "$arg" in
    --check) is_check=1 ;;
    --config) skip_next=1 ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
if [[ "$is_check" == "1" ]]; then
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
