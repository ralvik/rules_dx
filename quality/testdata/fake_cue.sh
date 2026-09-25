#!/usr/bin/env bash
# Fake cue for Layer-2 matrix cells (seed-only wiring proof).
#
# Mimics check (diff headers on stdout when dirty, exit 1; empty stdout
# when clean, exit 0) plus fix (in-place rewrite, exit 0) over the
# BADFMT marker. Real-tool behavior stays proven by
# `quality/tests/fixtures/file_family_adapters/cue/`.
# See: `docs/quality/tool-integrations.md#initial-adapter-qualification`
set -euo pipefail
is_fix=0
files=()
for arg in "$@"; do
  case "$arg" in
    -w|--write|--fix|-i|--reformat) is_fix=1 ;;
    --check|--diff|-d|-lint|--test|--lint|-lint) ;;
    -*) ;;
    *) if [[ -f "$arg" ]]; then files+=("$arg"); fi ;;
  esac
done
# djlint_format carries --reformat plus --check for check; fix is bare --reformat.
if [[ "cue" == "djlint_format" ]]; then
  is_fix=0
  has_reformat=0
  has_check=0
  for arg in "$@"; do
    case "$arg" in
      --reformat) has_reformat=1 ;;
      --check) has_check=1 ;;
    esac
  done
  if [[ "$has_reformat" == "1" && "$has_check" == "0" ]]; then is_fix=1; fi
fi
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
    printf -- "--- a/%s\n+++ b/%s\n@@ -1 +1 @@\n-BADFMT\n+fixed\n" "$f" "$f"
    dirty=1
  fi
done
if [[ "$dirty" == "1" ]]; then exit 1; fi
exit 0
