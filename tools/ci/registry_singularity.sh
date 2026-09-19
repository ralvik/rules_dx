#!/usr/bin/env bash
# Registry-singularity harness (issue #6 closed): machine-checks that the
# semantic file-class registry stays single-sourced.
#
# `quality/adapters.bzl` owns the
# one class-to-family map (`REAL_CLASS_TO_FAMILY`) plus the real
# adapter capability manifests (`REAL_ADAPTERS`). Adapters must consume
# canonical IDs (no invented classes), map keys must be frozen IDs (no
# rogue IDs), each class must map to exactly one family (no second
# table assigns them differently), and the frozen assignments (JavaScript
# owns javascript/jsx, TypeScript owns typescript/tsx, JSON owns
# json/json5/jsonc, CSS owns css/less/scss, plus the single-family
# graphql/html/html_template/xml/gherkin/sql/text classes) must agree
# with the map. The synthetic fixture maps stay labeled
# fixture/provisional so they can never be mistaken for the taxonomy.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (9 checks): adapters consume canonical IDs, map keys are
# frozen IDs, one-class-one-family, canonical spelling, frozen assignment
# agreement, fixture labeling, registry completeness (every frozen ID
# has exactly one family), curated-defaults stay within the taxonomy
# (no parallel family or tool), and the admissibility table covers every
# frozen ID. Cache-execution plus determinism-permutation proofs stay
# open per #84.
#
# Versioned here, run by CI via `bazel run //tools/ci:registry_singularity`,
# following //tools/ci:release_policy.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
cd "$workspace"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# One `key": "family` pair per line from the single map block.
sed -n '/^REAL_CLASS_TO_FAMILY = {/,/^}/p' quality/adapters.bzl \
  | grep -o -E '"[a-z0-9_]+": "[a-z0-9_]+"' > "$scratch/pairs.txt"
sed 's/":.*//; s/"//g' "$scratch/pairs.txt" | LC_ALL=C sort -u > "$scratch/keys.txt"
sed 's/.*": "//; s/"//' "$scratch/pairs.txt" | LC_ALL=C sort -u > "$scratch/families.txt"

# Every class named in any real adapter capability list.
sed -n '/^REAL_ADAPTERS = {/,/^}/p' quality/adapters.bzl \
  | grep -o -E '"(lint|format|typecheck)": \[[^]]*\]' \
  | sed 's/^"[^"]*": \[//; s/\]$//' \
  | tr ',' '\n' | tr -d ' "' | grep -E '.+' \
  | LC_ALL=C sort -u > "$scratch/adapter_classes.txt"

# Frozen registry IDs: backtick-quoted IDs in the frozen table rows
# (header through the last row; exactly one such table exists).
sed -n '/^| Ecosystem or family |/,/^| Ecosystem manifests |/p' docs/quality/quality-sources.md \
  | grep -E '^\| ' | grep -v -E -e '^\| ---' | grep -o -E '`[a-z0-9_]+`' | tr -d '`' \
  | LC_ALL=C sort -u > "$scratch/frozen.txt"

# Adapters consume canonical IDs: no manifest invents a class outside
# the single map.
if missing="$(comm -23 "$scratch/adapter_classes.txt" "$scratch/keys.txt")"; [[ -z "$missing" ]]; then
  ok
else
  bad "adapter manifests name classes outside REAL_CLASS_TO_FAMILY: $(echo "$missing" | tr '\n' ' ')"
fi

# Map keys are frozen IDs: no rogue IDs outside the owned registry.
if rogue="$(comm -23 "$scratch/keys.txt" "$scratch/frozen.txt")"; [[ -z "$rogue" ]]; then
  ok
else
  bad "REAL_CLASS_TO_FAMILY keys outside the frozen registry: $(echo "$rogue" | tr '\n' ' ')"
fi

# One class, one family: every key occurs exactly once in the map
# block, so no second assignment can disagree.
if dup="$(sed 's/":.*//; s/"//g' "$scratch/pairs.txt" | LC_ALL=C sort | uniq -d)"; [[ -z "$dup" ]]; then
  ok
else
  bad "REAL_CLASS_TO_FAMILY assigns a class twice: $(echo "$dup" | tr '\n' ' ')"
fi

# Canonical spelling only: lowercase IDs with no aliases or case
# drift on either side of the map.
if grep -q -E -v -e '^[a-z0-9_]+$' "$scratch/keys.txt" "$scratch/families.txt"; then
  bad "REAL_CLASS_TO_FAMILY carries a non-canonical class or family spelling"
else
  ok
fi

# The frozen assignments agree with the map: JavaScript owns
# javascript/jsx, TypeScript owns typescript/tsx, JSON owns
# json/json5/jsonc, CSS owns css/less/scss, and graphql, html,
# html_template, xml, gherkin, sql, and text each own their own family.
agree=1
for pair in "javascript:javascript" "jsx:javascript" "typescript:typescript" "tsx:typescript" "json:json" "json5:json" "jsonc:json" "css:css" "less:css" "scss:css" "graphql:graphql" "html:html" "html_template:html_template" "xml:xml" "gherkin:gherkin" "sql:sql" "text:text"; do
  class="${pair%%:*}"
  want="${pair##*:}"
  got="$(grep -E -e "\"$class\": " "$scratch/pairs.txt" | sed 's/.*": "//; s/"//' || true)"
  if [[ "$got" != "$want" ]]; then
    agree=0
    bad "frozen assignment drifted: $class maps to '$got', want '$want'"
  fi
done
[[ "$agree" == "1" ]] && ok

# The synthetic maps stay labeled fixture/provisional, never the
# taxonomy.
if grep -q -F -e 'Provisional M03-only registry' quality/adapters.bzl && grep -q -F -e 'WP2 fixture class-to-family assignment' quality/adapters.bzl; then
  ok
else
  bad "synthetic adapter maps lost their fixture/provisional labeling"
fi

# Registry completeness (issue #6 closed): every frozen ID has exactly
# one family, so no silent gap can rot into an undocumented assignment.
if missing="$(comm -23 "$scratch/frozen.txt" "$scratch/keys.txt")"; [[ -z "$missing" ]]; then
  ok
else
  bad "frozen IDs without family assignment: $(echo "$missing" | tr '\n' ' ')"
fi

# Curated defaults stay within the single-sourced taxonomy: every
# curated family is a family value in REAL_CLASS_TO_FAMILY and every
# curated tool is a known REAL_ADAPTERS tool, so curated defaults cannot
# drift into a parallel taxonomy without review.
sed -n '/^CURATED_DEFAULTS = {/,/^}/p' quality/curated_defaults.bzl \
  | grep -E -e '^    "[a-z0-9_]+": \{' | sed 's/^    "//; s/":.*//' \
  | LC_ALL=C sort -u > "$scratch/curated_families.txt"
sed -n '/^CURATED_DEFAULTS = {/,/^}/p' quality/curated_defaults.bzl \
  | grep -o -E -e '"(audit|format|lint|typecheck)": \[[^]]*\]' \
  | sed 's/^"[^"]*": \[//; s/\]$//' \
  | tr ',' '\n' | tr -d ' "' | grep -E -e '.+' \
  | LC_ALL=C sort -u > "$scratch/curated_tools.txt"
sed -n '/^REAL_ADAPTERS = {/,/^}/p' quality/adapters.bzl \
  | grep -E -e '^    "[a-z0-9_]+": \{' | sed 's/^    "//; s/":.*//' \
  | LC_ALL=C sort -u > "$scratch/real_tools.txt"
curated_clean=1
if missing="$(comm -23 "$scratch/curated_families.txt" "$scratch/families.txt")"; [[ -z "$missing" ]]; then
  :
else
  curated_clean=0
  bad "curated defaults name families outside REAL_CLASS_TO_FAMILY: $(echo "$missing" | tr '\n' ' ')"
fi
if missing_tools="$(comm -23 "$scratch/curated_tools.txt" "$scratch/real_tools.txt")"; [[ -z "$missing_tools" ]]; then
  :
else
  curated_clean=0
  bad "curated defaults name tools outside REAL_ADAPTERS: $(echo "$missing_tools" | tr '\n' ' ')"
fi
[[ "$curated_clean" == "1" ]] && ok

# Admissibility covers every frozen ID: the owned table in
# quality-sources.md names each class once in its first column, so
# per-ID shapes cannot rot without failing here.
sed -n '/^| Class |/,/^$/p' docs/quality/quality-sources.md \
  | grep -E '^\| `' | sed -E 's/^\| `([a-z0-9_]+)`.*/\1/' \
  | LC_ALL=C sort -u > "$scratch/admissible.txt"
if missing_adm="$(comm -23 "$scratch/frozen.txt" "$scratch/admissible.txt")"; [[ -z "$missing_adm" ]]; then
  ok
else
  bad "admissibility table misses frozen IDs: $(echo "$missing_adm" | tr '\n' ' ')"
fi

echo "registry singularity audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
