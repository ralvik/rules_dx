#!/usr/bin/env bash
# Code ownership audit (issue #12, lane A slice 1): every checked-in code
# source must ride its normal Bazel target (not a parallel corpus list),
# so no production file is silently skipped by `bazel build //...` and
# `bazel test //...`. Corpus stays for target-less files only (docs,
# BUILD files, configs); code rides normal targets.
#
# This is the code-extension counterpart to the corpus audit
# (`corpus_audit.sh`, issue #80): corpus_audit proves every
# BUILD/MODULE/*.bzl/*.toml/*.md has a `real_source_target` owner;
# this harness proves every code source (rs/py/js/ts/jsx/tsx/go/java/
# kotlin/scala/csharp/fsharp/c/cxx/vue/svelte/astro/mdx) is in
# `deps(//...)` under the parent universe. A file with no target fails
# here instead of being silently skipped.
#
# Declared generated-file exclusion list (each entry names its
# generator, per #12): foreign-tree arrival files under
# examples/adopt-js-ts/ that the Gazelle JS/TS extensions correctly
# classify as inert (no targets, see examples/adopt-js-ts/README.md).
# Framework SFC and sourcemap/typings arrivals stay inert until the
# framework quality regions land (#8) and TS declaration handling is
# qualified (#7). If an excluded file gains a target, this harness
# fails asking to drop the exclusion (stale exclusions never linger).
#
# Versioned here, run by CI via `bazel run //tools/ci:code_ownership`,
# following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_mkscratch scratch

# Stage 4 E2E carve-out (issue #55): `integration/` scenario workspaces
# are .bazelignore'd out of the parent universe and staged to scratch
# by shell copy, so they can never be owned by //... targets. The
# marker check keeps this exclusion honest: dropping the ignore
# re-lists every scenario file below as uncovered.
grep -q -F -e 'integration/' .bazelignore

# Declared exclusions, one per line: foreign-tree inert arrivals.
# Generators: //gazelle/javascript:gazelle and
# //gazelle/typescript:gazelle emit no targets for these classes
# (inert by design per examples/adopt-js-ts/README.md); the .js.map
# bytes are tsc sourcemap arrivals, the .vue file is a framework SFC
# arrival pending the #8 composition regions, and the .d.ts files are
# typings arrivals pending qualified TS declaration handling (#7).
cat >"$scratch/excluded.txt" <<'EOF'
examples/adopt-js-ts/app/greet.js.map
examples/adopt-js-ts/app/types.d.ts
examples/adopt-js-ts/app/widget.vue
examples/adopt-js-ts/web/app.js.map
examples/adopt-js-ts/web/types.d.ts
EOF

git ls-files |
  grep -E '\.(rs|py|js|mjs|cjs|ts|mts|cts|jsx|tsx|go|java|kt|kts|scala|cs|fs|fsx|c|h|cc|cpp|hpp|vue|svelte|astro|mdx)$|\.js\.map$' |
  grep -v -E '^integration/' |
  LC_ALL=C sort -u >"$scratch/code_applicable.txt"

bazel query "kind('source file', deps(//...))" 2>/dev/null |
  grep -E '^(@@)?//' |
  sed 's/^@@//; s|^//||; s|:|/|; s|^/||' |
  LC_ALL=C sort -u >"$scratch/code_closure.txt"

# Every exclusion must still be present and still unowned: a newly
# owned exclusion is a stale entry, not a pass.
stale=0
while read -r excluded; do
  if [[ -z "$excluded" ]]; then
    continue
  fi
  if ! grep -q -F -x -e "$excluded" "$scratch/code_applicable.txt"; then
    echo "code ownership audit failed: exclusion vanished from tree: $excluded (drop it from the exclusion list)"
    stale=1
  elif grep -q -F -x -e "$excluded" "$scratch/code_closure.txt"; then
    echo "code ownership audit failed: exclusion now owned by //...: $excluded (drop it from the exclusion list)"
    stale=1
  fi
done <"$scratch/excluded.txt"
if [[ "$stale" -ne 0 ]]; then
  exit 1
fi

comm -23 "$scratch/code_applicable.txt" "$scratch/code_closure.txt" |
  grep -v -F -x -f "$scratch/excluded.txt" >"$scratch/uncovered.txt" || true
uncovered="$(wc -l <"$scratch/uncovered.txt" | tr -d ' ')"
if [[ "$uncovered" -ne 0 ]]; then
  echo "code ownership audit failed: $uncovered code files have no //... target owner:"
  cat "$scratch/uncovered.txt"
  exit 1
fi
echo "code ownership audit: all code sources ride normal targets (5 inert adopt-js-ts arrivals excluded with generators)"
