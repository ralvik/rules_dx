#!/usr/bin/env bash
# Fake prettier for file-family matrix cells (seed-only wiring proof).
#
# Mimics `prettier --no-config --no-editorconfig --check` ([warn] lines
# on stderr when dirty, exit 1; clean exit 0) plus `--write` (in-place
# rewrite, exit 0) over the BADFMT marker. Real-tool behavior stays
# proven by `quality/tests/fixtures/file_family_adapters/` plus the
# pinned prettier parser; this fake proves runner dispatch plus parser
# plus fix flow without plugin closures.
# See: `docs/quality/tool-integrations.md#initial-adapter-qualification`
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
      sed -i 's/BADFMT/fixed/g' "$f"
    fi
  done
  exit 0
fi
dirty=0
for f in "${files[@]}"; do
  if grep -q -F "BADFMT" "$f"; then
    # Prettier reports working-directory-relative paths even for absolute
    # arguments, so emit the scratch-relative mirror path like the real
    # tool (the runner passes workspace-relative mirrors for attribution).
    rel="$f"
    if command -v realpath >/dev/null 2>&1; then
      rel="$(realpath --relative-to="$PWD" "$f" 2>/dev/null || echo "$f")"
    else
      rel="${f##*/}"
      # Fallback keeps the basename; the runner still re-anchors, but
      # prefer realpath above for exact workspace mirrors.
      :
    fi
    printf -- "[warn] %s\n" "$rel" >&2
    dirty=1
  fi
done
if [[ "$dirty" == "1" ]]; then exit 1; fi
exit 0
