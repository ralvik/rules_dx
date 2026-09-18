#!/usr/bin/env bash
# Registry-singularity harness (issues #6, #84): machine-checks that the
# semantic file-class registry stays single-sourced.
#
# `docs/quality/quality-sources.md` owns the frozen first-release
# registry of canonical lowercase IDs; `quality/adapters.bzl` owns the
# one class-to-family map (`REAL_CLASS_TO_FAMILY`) plus the real
# adapter capability manifests (`REAL_ADAPTERS`). Adapters must consume
# canonical IDs (no invented classes), map keys must be frozen IDs (no
# rogue IDs), each class must map to exactly one family (no second
# table assigns them differently), and the explicitly frozen example
# assignments (JavaScript owns javascript/jsx, TypeScript owns
# typescript/tsx, JSON owns the JSON classes) must agree with the map.
# The synthetic fixture maps stay labeled fixture/provisional so they
# can never be mistaken for the taxonomy, and the sources doc keeps
# the #6-pending disclaimer so the assignment is not published as
# stable API.
#
# This harness machine-checks the static half verifiable on a clean
# tree today (11 checks). The full registry review — class-to-family
# assignment for the still-unassigned frozen IDs, per-ID admissibility
# mappings, per-family independence rules — stays open per #6, and the
# cache-execution plus determinism-permutation proofs stay open per
# #84; all are recorded as gaps, not claimed here. Only the three
# admissibility edge examples owned in quality-sources.md are pinned,
# the 12 unassigned IDs are pinned as still unassigned, and the
# taxonomy grouping rule (one family per class, group only when sharing
# tool selection) is pinned as the pending-review constraint.
#
# Versioned here, run by CI via `bazel run //tools/ci:registry_singularity`,
# following //tools/ci:release_policy.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
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

# The explicitly frozen example assignments agree with the map:
# JavaScript owns javascript/jsx, TypeScript owns typescript/tsx, JSON
# owns the JSON classes (quality-sources.md candidate registry).
agree=1
for pair in "javascript:javascript" "jsx:javascript" "typescript:typescript" "tsx:typescript" "json:json"; do
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

# The sources doc keeps the pending-review disclaimer: the assignment is
# a candidate, not stable API.
if grep -q -F -e 'must not be published as stable' docs/quality/quality-sources.md; then
  ok
else
  bad "docs/quality/quality-sources.md lost the pending-review disclaimer"
fi

# The sources doc explicitly enumerates the still-unassigned frozen IDs
# as open work under issue #6, so the pending assignment cannot rot
# into an undocumented gap (assignment itself stays pending review).
open_doc_ok=1
for class in css gherkin graphql html html_template json5 jsonc less scss sql text xml; do
  if ! grep -q -F -e "\`$class\`" docs/quality/quality-sources.md; then
    open_doc_ok=0
    bad "docs/quality/quality-sources.md lost its open-work record for unassigned class $class (#6)"
  fi
done
if ! grep -q -F -e 'Open work under issue #6' docs/quality/quality-sources.md; then
  open_doc_ok=0
  bad "docs/quality/quality-sources.md lost its issue #6 open-work enumeration"
fi
[[ "$open_doc_ok" == "1" ]] && ok

# The sources doc keeps the per-ID admissibility pending record while
# owning only the three edge examples, so the prose cannot rot into a
# published-table claim (full mappings stay pending review per #6).
if grep -q -F -e 'Per-ID mappings remain pending the' docs/quality/quality-sources.md \
  && grep -q -F -e '.h` alone cannot decide' docs/quality/quality-sources.md \
  && grep -q -F -e 'extensionless files do not become `shell`' docs/quality/quality-sources.md \
  && grep -q -F -e 'BUILD`/`BUILD.bazel` are admissible as `starlark`' docs/quality/quality-sources.md; then
  ok
else
  bad "docs/quality/quality-sources.md lost its admissibility pending-record or edge examples (#6)"
fi

# The 12 open-work IDs stay unassigned: no silent family assignment or
# adapter claim lands without the registry review, so the gap cannot be
# closed without updating the #6 enumeration above.
unassigned_clean=1
for class in css gherkin graphql html html_template json5 jsonc less scss sql text xml; do
  if grep -q -F -e "\"$class\":" "$scratch/pairs.txt"; then
    unassigned_clean=0
    bad "unassigned class $class gained a family assignment without registry review (#6)"
  fi
  if grep -q -x -F -e "$class" "$scratch/adapter_classes.txt"; then
    unassigned_clean=0
    bad "unassigned class $class gained an adapter claim without registry review (#6)"
  fi
done
[[ "$unassigned_clean" == "1" ]] && ok

# The taxonomy grouping rule stays owned: every class has exactly one
# family and future assignment may group classes only when they should
# always share tool selection, so the pending review cannot be closed
# with an arbitrary grouping.
if grep -q -F -e 'Every semantic class has exactly one quality policy family' docs/quality/quality-sources.md \
  && grep -q -F -e 'must assign' docs/quality/quality-sources.md \
  && grep -q -F -e 'only when they should always share tool' docs/quality/quality-sources.md; then
  ok
else
  bad "docs/quality/quality-sources.md lost its taxonomy grouping rule (#6)"
fi

# Record the open remainder as information, not a gate: frozen IDs
# with no family assignment yet stay open work.
open="$(comm -23 "$scratch/frozen.txt" "$scratch/keys.txt" | tr '\n' ' ')"
echo "registry open work (frozen IDs without family assignment): ${open:-none}"
echo "registry singularity audit: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
