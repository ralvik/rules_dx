#!/usr/bin/env bash
# Test-only fixture layout guards.
#
# <lang>/hello/ were whole-folder test-only fixtures sitting as siblings
# to product (<lang>/rules/, <lang>/env/) with public visibility and no
# testonly, so external @rules_dx//<lang>/hello resolved. They now live
# under <lang>/tests/fixtures/hello/ with repo-only visibility and
# testonly, plus python/javascript/typescript entries under
# <lang>/tests/fixtures/entries/.
#
# This harness machine-checks the layout without rebuilding the tree:
# old locations are gone, new locations are present with repo-only
# visibility and testonly, product rules do not depend on tests, and
# Gazelle emits testonly for fixture paths.
#
# Versioned here, run by CI via `bazel run //tools/ci:fixture_layout`.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Old sibling fixtures are gone.
old_missing=""
for lang in astro cc csharp fsharp go java javascript kotlin mdx python rust scala svelte typescript vue; do
  if [[ -e "$lang/hello" ]]; then
    old_missing="$old_missing $lang:hello-still-present"
  fi
done
for entries in python/entries javascript/entries typescript/entries; do
  if [[ -e "$entries" ]]; then
    old_missing="$old_missing $entries-still-present"
  fi
done
if [[ -z "$old_missing" ]]; then
  ok
else
  bad "old sibling fixtures still present:$old_missing"
fi

# New fixture locations exist with wrapper consumers.
new_missing=""
for lang in astro cc csharp fsharp go java javascript kotlin mdx python rust scala svelte typescript vue; do
  if [[ ! -f "$lang/tests/fixtures/hello/BUILD.bazel" ]]; then
    new_missing="$new_missing $lang:BUILD"
  elif ! grep -q -F -e "$lang/rules:defs.bzl" "$lang/tests/fixtures/hello/BUILD.bazel" && ! grep -q -F -e "javascript/rules:defs.bzl" "$lang/tests/fixtures/hello/BUILD.bazel"; then
    new_missing="$new_missing $lang:wrapper"
  fi
done
for lang in python javascript typescript; do
  if [[ ! -f "$lang/tests/fixtures/entries/BUILD.bazel" ]]; then
    new_missing="$new_missing $lang:entries-BUILD"
  fi
done
if [[ -z "$new_missing" ]]; then
  ok
else
  bad "fixture layout missing:$new_missing"
fi

# Moved packages are repo-only (no public) with testonly.
visibility_fail=""
for pkg in astro/tests/fixtures/hello cc/tests/fixtures/hello csharp/tests/fixtures/hello fsharp/tests/fixtures/hello go/tests/fixtures/hello java/tests/fixtures/hello javascript/tests/fixtures/hello kotlin/tests/fixtures/hello mdx/tests/fixtures/hello python/tests/fixtures/hello rust/tests/fixtures/hello scala/tests/fixtures/hello svelte/tests/fixtures/hello typescript/tests/fixtures/hello vue/tests/fixtures/hello python/tests/fixtures/entries javascript/tests/fixtures/entries typescript/tests/fixtures/entries; do
  if [[ ! -f "$pkg/BUILD.bazel" ]]; then
    visibility_fail="$visibility_fail $pkg:missing"
  elif grep -q -F -e '//visibility:public' "$pkg/BUILD.bazel"; then
    visibility_fail="$visibility_fail $pkg:public"
  elif ! grep -q -F -e 'default_testonly = True' "$pkg/BUILD.bazel"; then
    visibility_fail="$visibility_fail $pkg:no-testonly"
  elif ! grep -q -F -e '//:__subpackages__' "$pkg/BUILD.bazel"; then
    visibility_fail="$visibility_fail $pkg:no-repo-visibility"
  fi
done
if [[ -z "$visibility_fail" ]]; then
  ok
else
  bad "fixture visibility/testonly drifted:$visibility_fail"
fi

# Product rules do not depend on tests (Bazel enforces testonly virally;
# this pins the absence explicitly for the wrapper families).
product_deps="$(bazel query "deps(//astro/rules/... + //cc/rules/... + //csharp/rules/... + //fsharp/rules/... + //go/rules/... + //java/rules/... + //javascript/rules/... + //kotlin/rules/... + //mdx/rules/... + //python/rules/... + //rust/rules/... + //scala/rules/... + //svelte/rules/... + //typescript/rules/... + //vue/rules/...)" 2>/dev/null | grep -F -e "/tests/" || true)"
if [[ -z "$product_deps" ]]; then
  ok
else
  bad "product rules depend on tests: $product_deps"
fi

# Fixture targets are testonly (default_testonly covers the package;
# attr() reflects the effective testonly, unlike --output=build which
# omits package defaults).
if [[ -z "$(bazel query "attr(testonly, 0, //python/tests/fixtures/hello/...)" 2>/dev/null)" ]]; then
  ok
else
  bad "fixture //python/tests/fixtures/hello lost testonly (non-testonly targets remain)"
fi

if [[ -z "$(bazel query "attr(testonly, 0, //python/tests/fixtures/entries/...)" 2>/dev/null)" ]]; then
  ok
else
  bad "fixture //python/tests/fixtures/entries lost testonly (non-testonly targets remain)"
fi

# Env plans over fixtures are testonly (they depend on testonly fixtures).
if [[ -n "$(bazel query "attr(testonly, 1, //python/env:hello_lib_plan)" 2>/dev/null)" ]]; then
  ok
else
  bad "env plan //python/env:hello_lib_plan lost testonly (must be testonly over testonly fixtures)"
fi

# Gazelle auto-testonly: generated rules under tests/fixtures/testdata
# carry testonly (one probe per family; full matrix in lang_tests).
gazelle_fail=""
for lang in go python rust cc java kotlin scala csharp fsharp javascript typescript vue svelte astro mdx; do
  if ! grep -q -F -e 'tests/fixtures' "gazelle/$lang/lang.go" && ! grep -q -F -e 'tests' "gazelle/$lang/lang.go"; then
    gazelle_fail="$gazelle_fail $lang:no-fixture-testonly"
  fi
done
# At least the shared helper or per-language probes must mention fixture testonly.
if grep -rq -F -e 'isFixturePath' gazelle/ 2>/dev/null || grep -rq -F -e 'tests/fixtures' gazelle/ 2>/dev/null; then
  ok
else
  bad "Gazelle auto-testonly missing (want isFixturePath or tests/fixtures in gazelle/):$gazelle_fail"
fi

dx_test_summary "fixture layout harness"
