#!/usr/bin/env bash
# Code ownership audit (lane A slice 1): every checked-in code
# source must ride its normal Bazel target (not a parallel corpus list),
# so no production file is silently skipped by `bazel build //...` and
# `bazel test //...`. Corpus stays for target-less files only (docs,
# BUILD files, configs); code rides normal targets.
#
# This is the code-extension counterpart to the corpus audit
# (`corpus_audit.sh`,): corpus_audit proves every
# BUILD/MODULE/*.bzl/*.toml/*.md has a `real_source_target` owner;
# this harness proves every code source (rs/py/js/ts/jsx/tsx/go/java/
# kotlin/scala/csharp/fsharp/c/cxx/vue/svelte/astro/mdx) is in
# `deps(//...)` under the parent universe. A file with no target fails
# here instead of being silently skipped.
#
# Declared generated-file exclusion list (each entry names its
# generator, per): foreign-tree arrival files under
# examples/adopt-js-ts/ that the composed `//dx:generate` correctly
# classifies as inert (no targets, see examples/adopt-js-ts/README.md).
# Sourcemap/typings arrivals stay inert until TS declaration handling
# is qualified. If an excluded file gains a target, this harness
# fails asking to drop the exclusion (stale exclusions never linger).
#
# Versioned here, run by CI via `bazel run //tools/ci:code_ownership`,
# following //tools/ci:corpus_audit.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_mkscratch scratch

# Nested E2E removed, so no `integration/` carve-out. Every
# checked-in code source must ride a normal `//...` target; the former
# `.bazelignore` exclusion plus shell-copy staging are deleted.

# Declared exclusions, one per line: foreign-tree inert arrivals plus
# go-mod-less lint subjects.
# Generators: the composed `//dx:generate` emits no targets for these
# classes (inert by design per examples/adopt-js-ts/README.md); the
# .js.map bytes are tsc sourcemap arrivals and the.d.ts files are
# typings arrivals pending qualified TS declaration handling. The former
# framework SFC arrival (`widget.vue`) is owned by the composed Vue
# extension since composition landed, so it left this list.
# The bare `Sample.go` lint subjects carry no go.mod, so no valid
# `go_library` can own them; adapter unit tests consume them via the
# source-tree path, never a Bazel target. The cppcheck `Sample.c` and
# scalafix `Sample.scala` subjects are intentionally uncompilable, so
# no compiled target can own them either; their quality harnesses
# consume them the same way.
cat >"$scratch/excluded.txt" <<'EOF'
examples/adopt-js-ts/app/greet.js.map
examples/adopt-js-ts/app/types.d.ts
examples/adopt-js-ts/web/app.js.map
examples/adopt-js-ts/web/types.d.ts
go/tests/fixtures/errcheck/Sample.go
go/tests/fixtures/gofumpt/Sample.go
go/tests/fixtures/govet/Sample.go
go/tests/fixtures/staticcheck/Sample.go
cc/tests/fixtures/cppcheck/Sample.c
scala/tests/fixtures/scalafix/Sample.scala
EOF

git ls-files |
  grep -E '\.(rs|py|js|mjs|cjs|ts|mts|cts|jsx|tsx|go|java|kt|kts|scala|cs|fs|fsx|c|h|cc|cpp|hpp|vue|svelte|astro|mdx)$|\.js\.map$' |
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
echo "code ownership audit: all code sources ride normal targets (4 inert adopt-js-ts arrivals plus 6 harness-consumed lint subjects excluded with generators)"
