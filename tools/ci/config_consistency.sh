#!/usr/bin/env bash
# Config single-source-of-truth consistency (issue #912).
#
# Canonical sources:
#   Biome version: `quality/artifacts/update.py` TOOLS[biome] upstream_version;
#     both `biome.json` files carry `$schema: .../<version>/schema.json`.
#   Biome editor excludes: workspace `biome.json` owns `files.includes`
#     with `**` plus Bazel-mirroring ignores (regular `!**/node_modules`
#     so type info still indexes, force-ignore `!!` for build outputs,
#     caches, and agent-local state); the `quality/testdata` fixture
#     stays byte-identical. Biome v2 removed `files.ignore`, so the
#     v2 `includes` negations are the ignore mechanism (issue #1072).
#   Ruff lint selection + Python floor: workspace `ruff.toml` owns the
#     selection; the `quality/testdata` fixture mirrors it.
#   pnpm version: both `package.json` files pin the same `packageManager`;
#     `MODULE.bazel` resolves the pnpm toolchain from the root pin.
#   Node floor: both `package.json` files carry `engines: {node: 22}` to
#     match the managed Node 22 toolchain (see MODULE.bazel plus
#     docs/tools/tool-acquisition.md).
#   Python floor + bounds: `quality/tools/python/pyproject.toml` owns
#     `requires-python` plus lower bounds; exact pins live in `uv.lock`.
#   Vale style: `quality/corpus_styles/Dx/Markers.yml` owns the Dx.Markers
#     rule; the `quality/testdata` copy stays byte-identical (StylesPath
#     differs per package, so label sharing cannot apply). The root
#     `.vale.ini` editor-discovery shim mirrors the corpus binding
#     (`StylesPath` points at `quality/corpus_styles`); Bazel keeps
#     binding `quality/corpus_vale.ini` explicitly.
#
# Every other copy below must equal its canonical source or this fails, so
# a version bump means: bump the canonical file once, then update the
# tracked copies in the same reviewed change.
#
# Usage: config_consistency.sh <biome_json> <biome_fixture> <update_py>
#   <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace>
#   <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers>
#   <corpus_ini> <fixture_ini> <root_vale_ini>
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_test_init

biome_json="${1:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
biome_fixture="${2:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
update_py="${3:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
ruff_toml="${4:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
ruff_fixture="${5:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
root_pkg="${6:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
js_pkg="${7:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
root_workspace="${8:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
js_workspace="${9:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
pyproject="${10:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
uv_lock="${11:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
corpus_markers="${12:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
fixture_markers="${13:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
corpus_ini="${14:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
fixture_ini="${15:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"
root_vale_ini="${16:?usage: config_consistency.sh <biome_json> <biome_fixture> <update_py> <ruff_toml> <ruff_fixture> <root_pkg> <js_pkg> <root_workspace> <js_workspace> <pyproject> <uv_lock> <corpus_markers> <fixture_markers> <corpus_ini> <fixture_ini> <root_vale_ini>}"

# --- Biome canonical: TOOLS[biome] owns the $schema version ---
biome_version="$(grep -A2 -F -e '"biome": {' "$update_py" | grep -o -E -e '"upstream_version": "[^"]+"' | head -1 | cut -d'"' -f4 || true)"
if [[ -z "$biome_version" ]]; then
  bad "quality/artifacts/update.py missing TOOLS[biome] upstream_version (issue #912)"
else
  ok
fi
for f in "$biome_json" "$biome_fixture"; do
  if grep -q -F -e "https://biomejs.dev/schemas/${biome_version}/schema.json" "$f"; then
    ok
  else
    bad "$f \$schema must track TOOLS[biome] $biome_version (want https://biomejs.dev/schemas/${biome_version}/schema.json, issue #912)"
  fi
done
if cmp -s "$biome_json" "$biome_fixture"; then
  ok
else
  bad "biome.json fixture drifts from the canonical root (want byte-identical $biome_json and $biome_fixture, issue #912)"
fi
for f in "$biome_json" "$biome_fixture"; do
  dx_guards_contains "$f" "$f lost editor excludes (want files.includes with ** plus Bazel-mirroring node_modules/bazel-*/dist/release/caches/opencode, issue #1072)" \
    '"files"' \
    '"includes"' \
    '"**"' \
    '!**/node_modules' \
    '!!**/bazel-*' \
    '!!**/dist' \
    '!!**/release' \
    '!!**/.cache' \
    '!!**/.direnv' \
    '!!**/.tmp' \
    '!!**/__pycache__' \
    '!!**/.ruff_cache' \
    '!!**/.opencode'
done

# --- Ruff canonical: shared lint selection plus Python floor ---
for f in "$ruff_toml" "$ruff_fixture"; do
  dx_guards_contains "$f" "$f lost the shared Ruff lint selection (want E4/E7/E9/F/W/I/B, issue #912)" \
    'select = ["E4", "E7", "E9", "F", "W", "I", "B"]'
  dx_guards_contains "$f" "$f lost the Python floor (want requires-python plus target-version py312, issue #912)" \
    'requires-python = ">=3.12"' \
    'target-version = "py312"'
done

# --- pnpm canonical: both package.json files pin the same packageManager ---
root_pm="$(grep -o -E -e '"packageManager": "[^"]+"' "$root_pkg" | head -1 | cut -d'"' -f4 || true)"
js_pm="$(grep -o -E -e '"packageManager": "[^"]+"' "$js_pkg" | head -1 | cut -d'"' -f4 || true)"
if [[ -z "$root_pm" || -z "$js_pm" ]]; then
  bad "packageManager missing in package.json inputs (want pnpm pin in both, issue #912)"
elif [[ "$root_pm" == "$js_pm" ]]; then
  ok
else
  bad "packageManager drifts ($root_pkg is $root_pm, $js_pkg is $js_pm; want identical, issue #912)"
fi
for f in "$root_pkg" "$js_pkg"; do
  dx_guards_contains "$f" "$f lost the managed Node floor (want engines node 22, issue #912)" \
    '"engines"' \
    '"node": "22"'
done
for f in "$root_workspace" "$js_workspace"; do
  dx_guards_contains "$f" "$f lost the single-importer workspace shape (want packages plus allowBuilds fail-closed, issue #912)" \
    '- .' \
    'allowBuilds: {}'
done

# --- Python canonical: pyproject owns bounds, uv.lock owns exact pins ---
dx_guards_contains "$pyproject" "pyproject.toml lost the interpreter floor (want requires-python >=3.12, issue #912)" \
  'requires-python = ">=3.12"'
dx_guards_contains "$pyproject" "pyproject.toml lost dependency lower bounds (want pydoclint/flake8/pylint with >=, issue #912)" \
  'pydoclint>=' \
  'flake8>=' \
  'pylint>='
dx_guards_contains "$pyproject" "pyproject.toml lost the update-flow contract (want uv lock regeneration note, issue #912)" \
  'uv lock'
for dep in "pydoclint" "flake8" "pylint"; do
  bound="$(grep -o -E -e "${dep}>=([0-9][0-9.]*)" "$pyproject" | head -1 | sed -E "s/^${dep}>=//" || true)"
  pinned="$(grep -A1 -F -e "name = \"${dep}\"" "$uv_lock" | grep -o -E -e 'version = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
  if [[ -z "$bound" || -z "$pinned" ]]; then
    bad "python $dep missing bound ($bound) or lock pin ($pinned); want lower bound in pyproject plus exact pin in uv.lock (issue #912)"
  elif [[ "$bound" == "$pinned" ]]; then
    ok
  else
    bad "python $dep lower bound $bound drifts from uv.lock pin $pinned (want bound == locked version, issue #912)"
  fi
done

# --- Vale canonical: corpus style owns Dx.Markers, fixture stays identical ---
if cmp -s "$corpus_markers" "$fixture_markers"; then
  ok
else
  bad "Vale Markers.yml fixture drifts from the corpus (want byte-identical $corpus_markers and $fixture_markers, issue #912)"
fi
for f in "$corpus_ini" "$fixture_ini"; do
  dx_guards_contains "$f" "$f lost the Vale BasedOnStyles binding (want BasedOnStyles = Dx, issue #912)" \
    'BasedOnStyles = Dx'
done
dx_guards_contains "$root_vale_ini" "$root_vale_ini drifted from the corpus Vale policy (want the Dx binding plus suggestion floor plus quality/corpus_styles, issue #912)" \
  'BasedOnStyles = Dx' \
  'MinAlertLevel = suggestion' \
  'StylesPath = quality/corpus_styles'

dx_test_summary "config consistency"
