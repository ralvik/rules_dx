#!/usr/bin/env bash
# Layer-2 adapter-less plus composition plus depcheck qualification harness.
#
# Qualifies the as-built open record with fixture evidence pinned in
# `quality/tests/fixtures/layer2_opens/pins.bzl` (plus
# `layer2_opens.expected`), without claiming delivery or Supported:
# - Layer-2 adapter-less cells stay open for Go plus Java plus Kotlin plus
#   C/C++ (no adapter claim, no runner-matrix cells, parity deferred with
#   owner plus frozen route; defaults qualified under -, digests plus
#   adapters stay owned under -; Scala plus C# plus F# delivered under #797).
#   Closed only for required core).
# - Framework regions stay classification-only for Vue plus Svelte plus Astro
#   plus MDX (frozen taxonomy plus quality-region mappings, no adapter claim,
#   no matrix cells, no curated defaults; Prettier/ESLint plugin closure
#   pending).
# - Composition evidence stays in `examples/mixed/hello/` (one wrapper per
#   container plus shared helper, no framework-to-framework imports) plus
#   `gazelle/mixed/` (disjoint plus complete partition, no fallback, no eager
#   work for unused adapters).
# - Framework-composition depcheck stays with the JS/TS pnpm route (no
#   separate framework fixtures; required-core plus admitted fixtures
# delivered in `tools/depcheck/` under).
# - Adapter-less as pass rejected per the issue alternatives; test plus
#   generation only per compatibility. Platform plus consumer plus release
#   evidence stays owned gap; no Supported claim. Backends stay provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:layer2_opens_qualification`,
# following //tools/ci:remediation_bounds_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="quality/tests/fixtures/layer2_opens/pins.bzl"
pins_build="quality/tests/fixtures/layer2_opens/BUILD.bazel"
expected="quality/tests/fixtures/layer2_opens/layer2_opens.expected"
adapters="quality/adapters.bzl"
parity="quality/parity_tests.bzl"
curated="quality/curated_defaults.bzl"
matrix="quality/testdata/runner_matrix_cases.bzl"
sources="quality/sources.bzl"
support="docs/product/support-matrix.md"
verify="docs/testing/verification-matrix.md"
framework="docs/generation/framework-adapters.md"
testing_doc="docs/quality/quality-testing.md"
depcheck_build="tools/depcheck/BUILD.bazel"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture triple stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "layer2-opens fixture missing (want $pins plus $pins_build plus $expected)"
fi

# Pins record the adapter-less plus framework inventory with rejected plus honesty lines.
# Scala plus C# plus F# delivered under #797 (no longer adapter-less).
if grep -q -F -e 'ADAPTER_LESS_GO = "go"' "$pins" &&
  grep -q -F -e 'ADAPTER_LESS_JAVA = "java"' "$pins" &&
  grep -q -F -e 'ADAPTER_LESS_KOTLIN = "kotlin"' "$pins" &&
  grep -q -F -e 'ADAPTER_LESS_C = "c"' "$pins" &&
  grep -q -F -e 'ADAPTER_LESS_CPP = "cpp"' "$pins" &&
  ! grep -q -F -e 'ADAPTER_LESS_SCALA' "$pins" &&
  ! grep -q -F -e 'ADAPTER_LESS_CSHARP' "$pins" &&
  ! grep -q -F -e 'ADAPTER_LESS_FSHARP' "$pins" &&
  grep -q -F -e 'FRAMEWORK_VUE = "vue"' "$pins" &&
  grep -q -F -e 'FRAMEWORK_SVELTE = "svelte"' "$pins" &&
  grep -q -F -e 'FRAMEWORK_ASTRO = "astro"' "$pins" &&
  grep -q -F -e 'FRAMEWORK_MDX = "mdx"' "$pins" &&
  grep -q -F -e 'REJECTED_ADAPTER_LESS_AS_PASS = "adapter-less as pass rejected"' "$pins" &&
  grep -q -F -e 'CLOSED_303_ONLY = "Closed #303 only"' "$pins" &&
  grep -q -F -e 'ADAPTERS_416_420_PARTIAL' "$pins" &&
  grep -q -F -e 'NO_SUPPORTED = "no Supported claim"' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #510' "$pins"; then
  ok
else
  bad "pins.bzl lost its adapter-less plus framework inventory with rejected plus honesty lines under issue #510"
fi

# Pins record composition plus depcheck plus owning qualifications.
if grep -q -F -e 'COMPOSITION_FIXTURE = "examples/mixed/hello"' "$pins" &&
  grep -q -F -e 'GAZELLE_MIXED = "gazelle/mixed"' "$pins" &&
  grep -q -F -e 'DEPCHECK_PNPM_ROUTE = "framework-composition depcheck stays with the JS/TS pnpm route"' "$pins" &&
  grep -q -F -e 'OWNING_JVM_COHORT = "digests plus adapters stay owned under #416"' "$pins" &&
  grep -q -F -e 'OWNING_SCALA_DOTNET_COHORT' "$pins" &&
  grep -q -F -e 'OWNING_NATIVE_COHORT' "$pins" &&
  grep -q -F -e 'OWNING_FOUNDATIONS_476_484' "$pins" &&
  grep -q -F -e 'LAYER2_OPENS_PROOF' "$pins"; then
  ok
else
  bad "pins.bzl lost its composition plus depcheck plus owning qualifications under issue #510"
fi

# No false adapter claim for the adapter-less cohorts: none of the cohort
# tool IDs appear in REAL_ADAPTERS. Classification exists; adapter claim
# does not (digests plus adapters stay owned under -). Scala/.NET delivered
# under #797, so its tools are excluded here.
opens_claim=""
for tool in gofumpt staticcheck govet errcheck clang-format clang-tidy cppcheck google-java-format checkstyle pmd spotbugs ktfmt ktlint detekt; do
  if grep -q -F -e "\"$tool\":" "$adapters"; then
    opens_claim="$opens_claim $tool:claimed"
  fi
done
if [[ -z "$opens_claim" ]]; then
  ok
else
  bad "false adapter claim for adapter-less cohorts:$opens_claim"
fi

# Adapter-less plus framework classes keep no runner-matrix cells
# (adapter-less as pass rejected: absence is the open record, never a pass).
# Scala/.NET cells delivered under #797, so excluded here.
opens_matrix=""
for cell in matrix_go_ matrix_java_ matrix_kotlin_ matrix_c_ matrix_cpp_ matrix_vue_ matrix_svelte_ matrix_astro_ matrix_mdx_; do
  if grep -q -F -e "$cell" "$matrix"; then
    opens_matrix="$opens_matrix $cell:claimed"
  fi
done
if [[ -z "$opens_matrix" ]]; then
  ok
else
  bad "runner-matrix claims an open class before adapters land:$opens_matrix"
fi

# Parity deferrals name owner plus frozen route for every open class.
# Scala/.NET delivered under #797, so excluded here.
opens_deferred=""
for cls in go c cpp java kotlin vue svelte astro mdx; do
  grep -q -F -e "\"$cls\":" "$parity" || opens_deferred="$opens_deferred $cls:missing"
done
if [[ -z "$opens_deferred" ]] &&
  grep -q -F -e '"go": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"java": ["ADR 0019"' "$parity" &&
  grep -q -F -e '"vue": ["ADR 0019"' "$parity" &&
  grep -q -F -e 'PARITY_DEFERRED = {' "$parity"; then
  ok
else
  bad "parity deferrals lost an open class owner/route:$opens_deferred"
fi

# Framework quality-region mappings stay classification-only: frozen
# taxonomy plus region classes with no adapter claim, no curated defaults,
# no matrix cells (claims land only with green adapter evidence).
opens_curated=""
for family in '"go": {' '"cc": {' '"java": {' '"kotlin": {' '"scala": {' '"csharp": {' '"fsharp": {' '"vue": {' '"svelte": {' '"astro": {' '"mdx": {'; do
  if grep -q -F -e "$family" "$curated"; then
    opens_curated="$opens_curated $family:claimed"
  fi
done
if [[ -z "$opens_curated" ]] &&
  grep -q -F -e '"vue"' "$sources" &&
  grep -q -F -e '"svelte"' "$sources" &&
  grep -q -F -e '"astro"' "$sources" &&
  grep -q -F -e '"mdx"' "$sources" &&
  grep -q -F -e '"vue": "vue"' "$adapters" &&
  grep -q -F -e '"svelte": "svelte"' "$adapters" &&
  grep -q -F -e '"go": "go"' "$adapters" &&
  grep -q -F -e '"java": "java"' "$adapters"; then
  ok
else
  bad "curated or taxonomy claims an open family before adapters land:$opens_curated"
fi

# Composition fixtures stay present: one wrapper per container plus shared
# helper plus hello_test with the shared helper edge and no
# framework-to-framework imports.
if [[ -f "examples/mixed/hello/Hello.vue" && -f "examples/mixed/hello/Hello.svelte" && -f "examples/mixed/hello/Hello.astro" && -f "examples/mixed/hello/Hello.mdx" && -f "examples/mixed/hello/helper.js" && -f "examples/mixed/hello/Hello.test.js" ]] &&
  grep -q -F -e 'vue_library' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e 'svelte_library' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e 'astro_library' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e 'mdx_library' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e 'helper_lib' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e 'hello_test' examples/mixed/hello/BUILD.bazel &&
  grep -q -F -e './helper.js' examples/mixed/hello/Hello.vue &&
  grep -q -F -e './helper.js' examples/mixed/hello/Hello.svelte &&
  grep -q -F -e './helper.js' examples/mixed/hello/Hello.astro &&
  grep -q -F -e './helper.js' examples/mixed/hello/Hello.mdx &&
  ! grep -q -F -e 'Hello.svelte' examples/mixed/hello/Hello.vue &&
  ! grep -q -F -e 'Hello.vue' examples/mixed/hello/Hello.svelte; then
  ok
else
  bad "mixed composition fixtures drifted (want four containers plus helper plus hello_test with no cross imports)"
fi

# Gazelle mixed partition stays disjoint plus complete with no fallback
# and no eager work for unused adapters.
if [[ -f "gazelle/mixed/mixed.go" && -f "gazelle/mixed/mixed_test.go" ]] &&
  grep -q -F -e 'Owner' gazelle/mixed/mixed.go &&
  grep -q -F -e 'Owners' gazelle/mixed/mixed.go &&
  grep -q -F -e '.vue' gazelle/mixed/mixed.go &&
  grep -q -F -e '.svelte' gazelle/mixed/mixed.go &&
  grep -q -F -e '.astro' gazelle/mixed/mixed.go &&
  grep -q -F -e '.mdx' gazelle/mixed/mixed.go &&
  grep -q -F -e 'no generic fallback' gazelle/mixed/mixed.go &&
  grep -q -F -e 'Owner' gazelle/mixed/mixed_test.go; then
  ok
else
  bad "gazelle/mixed partition drifted (want Owner plus Owners plus four extensions with no fallback)"
fi

# Depcheck fixtures stay delivered for required-core plus admitted langs
# ; framework-composition depcheck stays with the JS/TS pnpm
# route with no separate framework fixtures.
depcheck_missing=""
for lang in rust python js ts go java kotlin scala csharp fsharp cc; do
  [[ -d "tools/depcheck/testdata/$lang" ]] || depcheck_missing="$depcheck_missing $lang:dir"
done
if [[ -z "$depcheck_missing" ]] &&
  grep -q -F -e 'name = "depcheck_test"' "$depcheck_build" &&
  grep -q -F -e 'name = "depcheck"' "$depcheck_build" &&
  grep -q -F -e 'rust_test(' "$depcheck_build" &&
  [[ ! -d "tools/depcheck/testdata/vue" && ! -d "tools/depcheck/testdata/svelte" && ! -d "tools/depcheck/testdata/astro" && ! -d "tools/depcheck/testdata/mdx" ]] &&
  grep -q -F -e 'framework-composition depcheck stays with the JS/TS pnpm route' "$verify"; then
  ok
else
  bad "depcheck fixtures drifted:$depcheck_missing (want 11 langs plus pnpm route with no framework dirs)"
fi

# Expected fixture covers the open cells with rejected plus route honesty.
# Scala/C#/F# delivered under #797.
if grep -q -F -e 'Go Layer-2 Open (adapter-less)' "$expected" &&
  grep -q -F -e 'Java Layer-2 Open (adapter-less)' "$expected" &&
  grep -q -F -e 'C++ Layer-2 Open (adapter-less)' "$expected" &&
  grep -q -F -e 'Scala/C#/F# Layer-2 Delivered under #797' "$expected" &&
  grep -q -F -e 'Vue/Svelte/Astro/MDX Layer-2 Open (regions)' "$expected" &&
  grep -q -F -e 'Vue/Svelte/Astro/MDX Examples Open (composition)' "$expected" &&
  grep -q -F -e 'Vue/Svelte/Astro/MDX Depcheck Open' "$expected" &&
  grep -q -F -e 'adapter-less as pass rejected' "$expected" &&
  grep -q -F -e 'framework-composition depcheck stays with the JS/TS pnpm route' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #510' "$expected"; then
  ok
else
  bad "layer2_opens.expected lost its open-cells plus rejected plus route record under issue #510"
fi

# Support matrix owns the qualified seed-only record under.
if grep -q -F -e 'qualified seed-only under issue #510' "$support" &&
  grep -q -F -e 'quality/tests/fixtures/layer2_opens/pins.bzl' "$support" &&
  grep -q -F -e 'layer2_opens_qualification' "$support" &&
  grep -q -F -e 'adapter-less as pass rejected' "$support"; then
  ok
else
  bad "support-matrix lost its #510 qualified seed-only record with fixtures plus harness"
fi

# Framework adapters plus quality-testing docs own their records.
if grep -q -F -e 'qualified seed-only under issue #510' "$framework" &&
  grep -q -F -e 'layer2_opens_qualification' "$framework" &&
  grep -q -F -e 'examples/mixed/hello/' "$framework" &&
  grep -q -F -e 'remaining opens under issue #510' "$testing_doc" &&
  grep -q -F -e 'framework-composition depcheck stays with the JS/TS pnpm route' "$testing_doc"; then
  ok
else
  bad "framework-adapters or quality-testing lost its #510 qualified plus route record"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'layer2_opens_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under issue #510' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:layer2_opens_qualification' "$verify" &&
  grep -q -F -e '`layer2_opens_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #510 qualified record"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "layer2_opens_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:layer2_opens_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the layer2_opens_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the fixture plus the mixed composition build green on the
# seed host (defaults change only; no adapter behavior yet).
if bazel build //quality/tests/fixtures/layer2_opens:corpus_starlark //examples/mixed/hello/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "layer2-opens live proof failed (want fixture corpus plus mixed composition green)"
fi

dx_test_summary "layer2-opens qualification harness"
