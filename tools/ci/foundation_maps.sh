#!/usr/bin/env bash
# Foundation-mapping guards (ADR 0019 deferred record; relates,).
#
# Rust/Python/JS-TS foundations ship thin wrappers + Gazelle + env plans;
# exact provider/import/lock/tool-graph proofs are pinned here.
# Vue/Svelte/Astro/MDX ship named adapters over upstream parsers with an
# end-to-end fixture approach; exact parser/compiler, provider,
# generated-region, dependency, test, env/IDE, quality-region mappings plus
# composition evidence are pinned here.
# Required-core Rust providers/Gazelle/integration are pinned here
# (wrappers preserve CrateInfo/DepInfo/TestCrateInfo/CcInfo plus
# QualitySourcesInfo with conformance fixtures, Gazelle kinds/loads/goldens,
# env plan plus hello plus locks); native gaps stay owned under -.
# Required-core mappings (Rust providers/Gazelle/integration under plus native gaps
# under -, Python/JS mappings pinned, framework adapter mappings plus composition
# under) are pinned here.
# Admitted additional foundations (Go, C/C++, Java, Kotlin, Scala, C#, F#, Ruby)
# keep provisional upstreams with hello test runners, lock authority, and
# classification-only quality families pinned here; upgrades plus
# quality adapters (qualified under with deferred ADR 0019 routes) plus
# the C/C++ MSVC block stay owned gaps.
# Deferred/excluded record (PowerShell pending, Swift plus Bandit
# exclusion re-evidenced, host-toolchain fallback never approved) is pinned
# here per ADR 0032; retained cohorts plus exclusion evidence stay owned gaps
# with reconsideration requiring a new scope decision.
# Class-to-family taxonomy stays open under; native config binding +
# CI scope extension stay open under.
#
# This harness machine-checks the qualified mappings: owner links, adapter
# boundaries, provider advertisement, fixture markers, upstream pins,
# Gazelle fixtures, env plans, hello wrapper fixtures, lock authority, Ty
# provenance, plus the framework parser/compiler, provider, region,
# dependency, test, env/IDE, quality-region, and composition fixtures,
# plus the Rust provider/Gazelle/integration maps, the - build-script hermetic defaults, Ty/quality adapter mappings,
# and native-gap ownership, plus the - admitted test runners, lock wiring,
# quality classification, and MSVC-block ownership, plus the ADR 0019 deferred
# foundation absence, retained cohorts, exclusion evidence, and owned gaps.
# Upstream choices are kept; no switch is approved here.
#
# Versioned here, run by CI via `bazel run //tools/ci:foundation_maps`,
# following //tools/ci:wrapper_sources.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# minimum external-consumer example workspaces exist (Rust, Python,
# JS/TS); the rest of 's per-foundation set builds on these.
if [[ -d "examples/adopt-rust" && -d "examples/adopt-python" &&
  -d "examples/adopt-js-ts" ]]; then
  ok
else
  bad "minimum per-foundation example workspaces missing (adopt-rust/python/js-ts)"
fi

# language wrappers advertise QualitySourcesInfo (aspects gate on it).
langs_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc ruby; do
  if ! grep -q -F -e 'QualitySourcesInfo' "$lang/rules/defs.bzl" 2>/dev/null; then
    langs_missing="$langs_missing $lang"
  fi
done
if [[ -z "$langs_missing" ]]; then
  ok
else
  bad "language wrappers lost QualitySourcesInfo:$langs_missing"
fi

# exact provider mappings stay pinned in each wrapper.
provider_fail=""
grep -q -F -e 'CrateInfo' rust/rules/defs.bzl || provider_fail="$provider_fail rust:CrateInfo"
grep -q -F -e 'PyInfo' python/rules/defs.bzl || provider_fail="$provider_fail python:PyInfo"
grep -q -F -e 'JsInfo' javascript/rules/defs.bzl || provider_fail="$provider_fail javascript:JsInfo"
grep -q -F -e 'TsConfigInfo' typescript/rules/defs.bzl || provider_fail="$provider_fail typescript:TsConfigInfo"
grep -q -F -e 'GoInfo' go/rules/defs.bzl || provider_fail="$provider_fail go:GoInfo"
grep -q -F -e 'JavaInfo' java/rules/defs.bzl || provider_fail="$provider_fail java:JavaInfo"
grep -q -F -e 'JavaInfo' kotlin/rules/defs.bzl || provider_fail="$provider_fail kotlin:JavaInfo"
grep -q -F -e 'JavaInfo' scala/rules/defs.bzl || provider_fail="$provider_fail scala:JavaInfo"
grep -q -F -e 'DotnetAssemblyCompileInfo' csharp/rules/defs.bzl || provider_fail="$provider_fail csharp:DotnetAssembly"
grep -q -F -e 'DotnetAssemblyCompileInfo' fsharp/rules/defs.bzl || provider_fail="$provider_fail fsharp:DotnetAssembly"
grep -q -F -e 'CcInfo' cc/rules/defs.bzl || provider_fail="$provider_fail cc:CcInfo"
grep -q -F -e 'RubyFilesInfo' ruby/rules/defs.bzl || provider_fail="$provider_fail ruby:RubyFilesInfo"
if [[ -z "$provider_fail" ]]; then
  ok
else
  bad "language provider mappings drifted:$provider_fail"
fi

# upstream choices stay pinned; wrappers match MODULE.bazel pins.
upstream_fail=""
grep -q -F -e 'rules_rust 0.74.0' rust/rules/defs.bzl || upstream_fail="$upstream_fail rust"
grep -q -F -e 'aspect_rules_py 2.0.0-alpha.6' python/rules/defs.bzl || upstream_fail="$upstream_fail python"
grep -q -F -e 'aspect_rules_js 3.4.1' javascript/rules/defs.bzl || upstream_fail="$upstream_fail javascript"
grep -q -F -e 'aspect_rules_ts 3.10.0' typescript/rules/defs.bzl || upstream_fail="$upstream_fail typescript"
grep -q -F -e 'rules_go 0.63.0' go/rules/defs.bzl || upstream_fail="$upstream_fail go"
grep -q -F -e 'rules_java 9.7.0' java/rules/defs.bzl || upstream_fail="$upstream_fail java"
grep -q -F -e 'rules_kotlin 2.4.10' kotlin/rules/defs.bzl || upstream_fail="$upstream_fail kotlin"
grep -q -F -e 'rules_scala 7.3.0' scala/rules/defs.bzl || upstream_fail="$upstream_fail scala"
grep -q -F -e 'rules_dotnet 0.22.1' csharp/rules/defs.bzl || upstream_fail="$upstream_fail csharp"
grep -q -F -e 'rules_dotnet 0.22.1' fsharp/rules/defs.bzl || upstream_fail="$upstream_fail fsharp"
grep -q -F -e 'rules_cc 0.2.22' cc/rules/defs.bzl || upstream_fail="$upstream_fail cc"
grep -q -F -e 'rules_ruby 0.28.0' ruby/rules/defs.bzl || upstream_fail="$upstream_fail ruby"
grep -q -F -e 'bazel_dep(name = "rules_rust", version = "0.74.0")' MODULE.bazel || upstream_fail="$upstream_fail module:rust"
grep -q -F -e 'bazel_dep(name = "aspect_rules_py", version = "2.0.0-alpha.6")' MODULE.bazel || upstream_fail="$upstream_fail module:py"
grep -q -F -e 'bazel_dep(name = "aspect_rules_js", version = "3.4.1")' MODULE.bazel || upstream_fail="$upstream_fail module:js"
grep -q -F -e 'bazel_dep(name = "aspect_rules_ts", version = "3.10.0")' MODULE.bazel || upstream_fail="$upstream_fail module:ts"
grep -q -F -e 'bazel_dep(name = "aspect_rules_jest", version = "0.26.0")' MODULE.bazel || upstream_fail="$upstream_fail module:jest"
grep -q -F -e 'bazel_dep(name = "rules_go", version = "0.63.0")' MODULE.bazel || upstream_fail="$upstream_fail module:go"
grep -q -F -e 'bazel_dep(name = "rules_java", version = "9.7.0")' MODULE.bazel || upstream_fail="$upstream_fail module:java"
grep -q -F -e 'bazel_dep(name = "rules_kotlin", version = "2.4.10")' MODULE.bazel || upstream_fail="$upstream_fail module:kotlin"
grep -q -F -e 'bazel_dep(name = "rules_scala", version = "7.3.0")' MODULE.bazel || upstream_fail="$upstream_fail module:scala"
grep -q -F -e 'bazel_dep(name = "rules_dotnet", version = "0.22.1")' MODULE.bazel || upstream_fail="$upstream_fail module:dotnet"
grep -q -F -e 'bazel_dep(name = "rules_cc", version = "0.2.22")' MODULE.bazel || upstream_fail="$upstream_fail module:cc"
grep -q -F -e 'bazel_dep(name = "rules_jvm_external", version = "7.1")' MODULE.bazel || upstream_fail="$upstream_fail module:jvm"
grep -q -F -e 'bazel_dep(name = "rules_ruby", version = "0.28.0")' MODULE.bazel || upstream_fail="$upstream_fail module:ruby"
if [[ -z "$upstream_fail" ]]; then
  ok
else
  bad "upstream choices drifted:$upstream_fail"
fi

# Gazelle recognizer fixtures stay present (parser/lang/naming plus
# focused tests per language).
gazelle_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc ruby; do
  if [[ ! -f "gazelle/$lang/parser.go" || ! -f "gazelle/$lang/lang.go" || ! -f "gazelle/$lang/naming.go" ]]; then
    gazelle_missing="$gazelle_missing $lang:impl"
  fi
  if [[ ! -f "gazelle/$lang/parser_test.go" || ! -f "gazelle/$lang/lang_test.go" || ! -f "gazelle/$lang/naming_test.go" ]]; then
    gazelle_missing="$gazelle_missing $lang:tests"
  fi
done
if [[ -z "$gazelle_missing" ]]; then
  ok
else
  bad "Gazelle language fixtures missing:$gazelle_missing"
fi

# environment plans stay present (plan + focused fixtures per language).
env_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc ruby; do
  if [[ ! -f "$lang/env/plan.bzl" || ! -f "$lang/env/plan_tests.bzl" ]]; then
    env_missing="$env_missing $lang"
  fi
done
if [[ -z "$env_missing" ]]; then
  ok
else
  bad "environment plans missing:$env_missing"
fi

# hello builds stay present as wrapper consumers (import/search-path
# proof per language).
hello_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc ruby; do
  if [[ ! -f "$lang/tests/fixtures/hello/BUILD.bazel" ]]; then
    hello_missing="$hello_missing $lang:BUILD"
  elif ! grep -q -F -e "$lang/rules:defs.bzl" "$lang/tests/fixtures/hello/BUILD.bazel" && ! grep -q -F -e "javascript/rules:defs.bzl" "$lang/tests/fixtures/hello/BUILD.bazel"; then
    hello_missing="$hello_missing $lang:wrapper"
  fi
done
# TypeScript hello intentionally reuses the JavaScript binary wrapper
# (no typescript_binary); the check above accepts that shape.
if [[ -z "$hello_missing" ]]; then
  ok
else
  bad "hello wrapper fixtures missing:$hello_missing"
fi

# lock authority stays checked in (fail-closed wiring in MODULE.bazel).
# Go from_file lock is third_party/go/go.mod plus go.sum
# hello stays stdlib-only, godeps fixture proves the external dep); C/C++
# has no ecosystem lockfile (every http_archive carries sha256/integrity).
locks_missing=""
[[ -f "rust/tests/fixtures/hello/Cargo.lock" ]] || locks_missing="$locks_missing Cargo.lock"
[[ -f "cargo-bazel-lock.json" ]] || locks_missing="$locks_missing cargo-bazel-lock"
[[ -f "MODULE.bazel.lock" ]] || locks_missing="$locks_missing MODULE.bazel.lock"
[[ -f "python/tests/fixtures/hello/uv.lock" ]] || locks_missing="$locks_missing uv.lock"
[[ -f "quality/tools/python/uv.lock" ]] || locks_missing="$locks_missing tools-uv.lock"
[[ -f "pnpm-lock.yaml" ]] || locks_missing="$locks_missing pnpm-lock"
[[ -f "quality/tools/javascript/pnpm-lock.yaml" ]] || locks_missing="$locks_missing tools-pnpm-lock"
[[ -f "third_party/jvm/maven_install.json" ]] || locks_missing="$locks_missing maven_install"
[[ -f "third_party/dotnet/paket.lock" ]] || locks_missing="$locks_missing paket.lock"
[[ -f "third_party/dotnet/paket.dependencies" ]] || locks_missing="$locks_missing paket.dependencies"
[[ -f "third_party/go/go.mod" ]] || locks_missing="$locks_missing go.mod"
[[ -f "third_party/go/go.sum" ]] || locks_missing="$locks_missing go.sum"
[[ -f "third_party/ruby/Gemfile" ]] || locks_missing="$locks_missing Gemfile"
[[ -f "third_party/ruby/Gemfile.lock" ]] || locks_missing="$locks_missing Gemfile.lock"
grep -q -F -e 'fail_if_repin_required' MODULE.bazel || locks_missing="$locks_missing fail-closed"
grep -q -F -e 'gazelle_go_deps.from_file(go_mod = "//third_party/go:go.mod")' MODULE.bazel || locks_missing="$locks_missing go-from-file"
if [[ -z "$locks_missing" ]]; then
  ok
else
  bad "lock authority missing:$locks_missing"
fi

# Ty artifact provenance stays pinned (plus core tool artifacts).
tools_missing=""
grep -q -F -e '"tool": "ty"' quality/artifacts/ty.linux_x86_64.bzl || tools_missing="$tools_missing ty:tool"
grep -q -F -e '"upstream_version": "0.0.80"' quality/artifacts/ty.linux_x86_64.bzl || tools_missing="$tools_missing ty:version"
grep -q -F -e 'astral-sh/ty' quality/artifacts/ty.linux_x86_64.bzl || tools_missing="$tools_missing ty:url"
grep -q -F -e '"sha256"' quality/artifacts/ty.linux_x86_64.bzl || tools_missing="$tools_missing ty:sha"
grep -q -F -e '"licenses"' quality/artifacts/ty.linux_x86_64.bzl || tools_missing="$tools_missing ty:licenses"
[[ -f "quality/artifacts/ruff.linux_x86_64.bzl" ]] || tools_missing="$tools_missing ruff"
[[ -f "quality/artifacts/biome.linux_x86_64.bzl" ]] || tools_missing="$tools_missing biome"
[[ -f "quality/artifacts/buildifier.linux_x86_64.bzl" ]] || tools_missing="$tools_missing buildifier"
[[ -f "quality/artifacts/taplo.linux_x86_64.bzl" ]] || tools_missing="$tools_missing taplo"
[[ -f "quality/artifacts/vale.linux_x86_64.bzl" ]] || tools_missing="$tools_missing vale"
if [[ -z "$tools_missing" ]]; then
  ok
else
  bad "tool-graph provenance missing:$tools_missing"
fi

# framework provider mappings stay pinned in each wrapper (JsInfo
# preserved plus QualitySourcesInfo; only js_library/JsInfo used upstream;
# no separate binary/test wrapper).
fw_provider_fail=""
for fw in vue svelte astro mdx; do
  if ! grep -q -F -e 'JsInfo' "$fw/rules/defs.bzl" 2>/dev/null; then
    fw_provider_fail="$fw_provider_fail $fw:JsInfo"
  fi
  if ! grep -q -F -e 'QualitySourcesInfo' "$fw/rules/defs.bzl" 2>/dev/null; then
    fw_provider_fail="$fw_provider_fail $fw:QualitySourcesInfo"
  fi
  if ! grep -q -F -e 'js_library' "$fw/rules/defs.bzl" 2>/dev/null; then
    fw_provider_fail="$fw_provider_fail $fw:js_library"
  fi
done
if [[ -z "$fw_provider_fail" ]]; then
  ok
else
  bad "framework provider mappings drifted:$fw_provider_fail"
fi

# framework upstream parser/compiler choices stay pinned; wrappers match
# package.json and the JS toolchain pins in MODULE.bazel.
fw_upstream_fail=""
grep -q -F -e 'aspect_rules_js 3.4.1' vue/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail vue:ruleset"
grep -q -F -e 'aspect_rules_js 3.4.1' svelte/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail svelte:ruleset"
grep -q -F -e 'aspect_rules_js 3.4.1' astro/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail astro:ruleset"
grep -q -F -e 'aspect_rules_js 3.4.1' mdx/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail mdx:ruleset"
grep -q -F -e 'Vue 3.5.42' vue/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail vue:compiler"
grep -q -F -e 'Svelte 5.57.0' svelte/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail svelte:compiler"
grep -q -F -e '@astrojs/compiler 4.0.0' astro/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail astro:compiler"
grep -q -F -e '@mdx-js/mdx 3.1.1' mdx/rules/defs.bzl || fw_upstream_fail="$fw_upstream_fail mdx:compiler"
grep -q -F -e '"@vue/compiler-sfc": "3.5.42"' package.json || fw_upstream_fail="$fw_upstream_fail pkg:vue-compiler"
grep -q -F -e '"vue": "3.5.42"' package.json || fw_upstream_fail="$fw_upstream_fail pkg:vue"
grep -q -F -e '"svelte": "5.57.0"' package.json || fw_upstream_fail="$fw_upstream_fail pkg:svelte"
grep -q -F -e '"@astrojs/compiler": "4.0.0"' package.json || fw_upstream_fail="$fw_upstream_fail pkg:astro"
grep -q -F -e '"@mdx-js/mdx": "3.1.1"' package.json || fw_upstream_fail="$fw_upstream_fail pkg:mdx"
grep -q -F -e 'bazel_dep(name = "aspect_rules_js", version = "3.4.1")' MODULE.bazel || fw_upstream_fail="$fw_upstream_fail module:js"
grep -q -F -e 'bazel_dep(name = "aspect_rules_jest", version = "0.26.0")' MODULE.bazel || fw_upstream_fail="$fw_upstream_fail module:jest"
if [[ -z "$fw_upstream_fail" ]]; then
  ok
else
  bad "framework upstream choices drifted:$fw_upstream_fail"
fi

# framework Gazelle fixtures stay present (parser/lang/naming plus
# focused tests, stdlib, and testdata generation goldens per adapter; mixed
# ownership partition stays present).
fw_gazelle_missing=""
for fw in vue svelte astro mdx; do
  if [[ ! -f "gazelle/$fw/parser.go" || ! -f "gazelle/$fw/lang.go" || ! -f "gazelle/$fw/naming.go" || ! -f "gazelle/$fw/stdlib.go" ]]; then
    fw_gazelle_missing="$fw_gazelle_missing $fw:impl"
  fi
  if [[ ! -f "gazelle/$fw/parser_test.go" || ! -f "gazelle/$fw/lang_test.go" || ! -f "gazelle/$fw/naming_test.go" ]]; then
    fw_gazelle_missing="$fw_gazelle_missing $fw:tests"
  fi
  if [[ ! -f "gazelle/$fw/testdata/source_only/pkg/demo/BUILD.in" || ! -f "gazelle/$fw/testdata/source_only/pkg/demo/BUILD.out" ]]; then
    fw_gazelle_missing="$fw_gazelle_missing $fw:testdata"
  fi
  if ! grep -q -F -e 'SupportedExts' "gazelle/$fw/naming.go" 2>/dev/null; then
    fw_gazelle_missing="$fw_gazelle_missing $fw:exts"
  fi
  if ! grep -q -F -e "${fw}_library" "gazelle/$fw/testdata/source_only/pkg/demo/BUILD.out" 2>/dev/null; then
    fw_gazelle_missing="$fw_gazelle_missing $fw:kind"
  fi
done
[[ -f "gazelle/mixed/mixed.go" && -f "gazelle/mixed/mixed_test.go" ]] || fw_gazelle_missing="$fw_gazelle_missing mixed:impl"
if [[ -z "$fw_gazelle_missing" ]]; then
  ok
else
  bad "framework Gazelle fixtures missing:$fw_gazelle_missing"
fi

# framework environment plans stay present (plan + focused fixtures plus
# hello plan target per adapter).
fw_env_missing=""
for fw in vue svelte astro mdx; do
  if [[ ! -f "$fw/env/plan.bzl" || ! -f "$fw/env/plan_tests.bzl" || ! -f "$fw/env/BUILD.bazel" ]]; then
    fw_env_missing="$fw_env_missing $fw:files"
  elif ! grep -q -F -e 'JsInfo' "$fw/env/plan.bzl" || ! grep -q -F -e 'QualitySourcesInfo' "$fw/env/plan.bzl"; then
    fw_env_missing="$fw_env_missing $fw:providers"
  elif ! grep -q -F -e 'EXPECTED_ENV_PLAN_OBSERVATIONS' "$fw/env/plan_tests.bzl"; then
    fw_env_missing="$fw_env_missing $fw:tests"
  elif ! grep -q -F -e 'hello_lib_plan' "$fw/env/BUILD.bazel"; then
    fw_env_missing="$fw_env_missing $fw:plan-target"
  fi
done
if [[ -z "$fw_env_missing" ]]; then
  ok
else
  bad "framework environment plans missing:$fw_env_missing"
fi

# framework hello builds stay present as wrapper consumers (container +
# shared helper + upstream parser/compiler test per adapter).
fw_hello_missing=""
for fw in vue svelte astro mdx; do
  case "$fw" in
    vue) container="Hello.vue" ;;
    svelte) container="Hello.svelte" ;;
    astro) container="Hello.astro" ;;
    mdx) container="Hello.mdx" ;;
  esac
  if [[ ! -f "$fw/tests/fixtures/hello/BUILD.bazel" || ! -f "$fw/tests/fixtures/hello/$container" || ! -f "$fw/tests/fixtures/hello/Hello.test.js" || ! -f "$fw/tests/fixtures/hello/helper.js" ]]; then
    fw_hello_missing="$fw_hello_missing $fw:files"
  elif ! grep -q -F -e "${fw}/rules:defs.bzl" "$fw/tests/fixtures/hello/BUILD.bazel"; then
    fw_hello_missing="$fw_hello_missing $fw:wrapper"
  elif ! grep -q -F -e 'javascript_test' "$fw/tests/fixtures/hello/BUILD.bazel"; then
    fw_hello_missing="$fw_hello_missing $fw:test-kind"
  fi
done
if [[ -z "$fw_hello_missing" ]]; then
  ok
else
  bad "framework hello fixtures missing:$fw_hello_missing"
fi

# framework test semantics stay pinned (javascript_test over the
# upstream parser/compiler with container plus compiler npm data; no
# separate framework test wrapper; Hello.test.js exercises the regions).
fw_test_fail=""
grep -q -F -e '//:node_modules/@vue/compiler-sfc' vue/tests/fixtures/hello/BUILD.bazel || fw_test_fail="$fw_test_fail vue:compiler-data"
grep -q -F -e '//:node_modules/svelte' svelte/tests/fixtures/hello/BUILD.bazel || fw_test_fail="$fw_test_fail svelte:compiler-data"
grep -q -F -e '//:node_modules/@astrojs/compiler' astro/tests/fixtures/hello/BUILD.bazel || fw_test_fail="$fw_test_fail astro:compiler-data"
grep -q -F -e '//:node_modules/@mdx-js/mdx' mdx/tests/fixtures/hello/BUILD.bazel || fw_test_fail="$fw_test_fail mdx:compiler-data"
grep -q -F -e 'parse' vue/tests/fixtures/hello/Hello.test.js || fw_test_fail="$fw_test_fail vue:parse-test"
grep -q -F -e 'parse' svelte/tests/fixtures/hello/Hello.test.js || fw_test_fail="$fw_test_fail svelte:parse-test"
grep -q -F -e 'parse' astro/tests/fixtures/hello/Hello.test.js || fw_test_fail="$fw_test_fail astro:parse-test"
grep -q -F -e 'compile' mdx/tests/fixtures/hello/Hello.test.js || fw_test_fail="$fw_test_fail mdx:compile-test"
if grep -R -q -F -e 'vue_test' vue/tests/fixtures/hello/BUILD.bazel svelte/tests/fixtures/hello/BUILD.bazel astro/tests/fixtures/hello/BUILD.bazel mdx/tests/fixtures/hello/BUILD.bazel 2>/dev/null; then
  fw_test_fail="$fw_test_fail unexpected-vue_test"
fi
if grep -R -q -F -e 'svelte_test' vue/tests/fixtures/hello/BUILD.bazel svelte/tests/fixtures/hello/BUILD.bazel astro/tests/fixtures/hello/BUILD.bazel mdx/tests/fixtures/hello/BUILD.bazel 2>/dev/null; then
  fw_test_fail="$fw_test_fail unexpected-svelte_test"
fi
if grep -R -q -F -e 'astro_test' vue/tests/fixtures/hello/BUILD.bazel svelte/tests/fixtures/hello/BUILD.bazel astro/tests/fixtures/hello/BUILD.bazel mdx/tests/fixtures/hello/BUILD.bazel 2>/dev/null; then
  fw_test_fail="$fw_test_fail unexpected-astro_test"
fi
if grep -R -q -F -e 'mdx_test' vue/tests/fixtures/hello/BUILD.bazel svelte/tests/fixtures/hello/BUILD.bazel astro/tests/fixtures/hello/BUILD.bazel mdx/tests/fixtures/hello/BUILD.bazel 2>/dev/null; then
  fw_test_fail="$fw_test_fail unexpected-mdx_test"
fi
if [[ -z "$fw_test_fail" ]]; then
  ok
else
  bad "framework test mappings drifted:$fw_test_fail"
fi

# framework quality-region mappings stay pinned (frozen semantic classes
# plus one classification-only policy family per container; wrappers carry
# the matching quality_specs).
fw_quality_fail=""
for fw in vue svelte astro mdx; do
  grep -q -F -e "\"$fw\"" quality/sources.bzl || fw_quality_fail="$fw_quality_fail $fw:class"
  grep -q -F -e "\"$fw\": \"$fw\"" quality/adapters.bzl || fw_quality_fail="$fw_quality_fail $fw:family"
  grep -q -F -e "quality_specs" "$fw/rules/defs.bzl" || fw_quality_fail="$fw_quality_fail $fw:specs"
done
if [[ -z "$fw_quality_fail" ]]; then
  ok
else
  bad "framework quality-region mappings drifted:$fw_quality_fail"
fi

# mixed-framework composition stays pinned (; one wrapper per
# container plus the shared helper; per-container helper edge with no
# framework-to-framework imports; disjoint gazelle/mixed partition).
fw_mixed_fail=""
[[ -d "examples/mixed/hello" ]] || fw_mixed_fail="$fw_mixed_fail missing-dir"
grep -q -F -e '' examples/mixed/hello/BUILD.bazel || fw_mixed_fail="$fw_mixed_fail -marker"
for fw in vue svelte astro mdx; do
  grep -q -F -e "$fw/rules:defs.bzl" examples/mixed/hello/BUILD.bazel || fw_mixed_fail="$fw_mixed_fail mixed:$fw-wrapper"
  [[ -f "examples/mixed/hello/Hello.${fw#vue:}" ]] 2>/dev/null || true
done
[[ -f "examples/mixed/hello/Hello.vue" && -f "examples/mixed/hello/Hello.svelte" && -f "examples/mixed/hello/Hello.astro" && -f "examples/mixed/hello/Hello.mdx" ]] || fw_mixed_fail="$fw_mixed_fail mixed:containers"
grep -q -F -e 'helper_lib' examples/mixed/hello/BUILD.bazel || fw_mixed_fail="$fw_mixed_fail mixed:helper"
grep -q -F -e 'Owner' gazelle/mixed/mixed.go || fw_mixed_fail="$fw_mixed_fail mixed:partition"
if [[ -z "$fw_mixed_fail" ]]; then
  ok
else
  bad "mixed composition fixtures missing:$fw_mixed_fail"
fi

# wrapper-sources pin + ownership audits stay versioned.
if [[ -f "tools/ci/wrapper_sources.sh" && -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "wrapper_sources/code_ownership harnesses missing"
fi

# Rust provider plus Gazelle maps stay pinned (wrappers preserve
# upstream CrateInfo/DepInfo/TestCrateInfo/CcInfo plus QualitySourcesInfo
# with conformance fixtures, Gazelle kinds/loads/goldens, env plan plus
# hello plus locks with owning docs).
rust470_fail=""
grep -q -F -e 'CrateInfo' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:CrateInfo"
grep -q -F -e 'DepInfo' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:DepInfo"
grep -q -F -e 'TestCrateInfo' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:TestCrateInfo"
grep -q -F -e 'CcInfo' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:CcInfo"
grep -q -F -e 'QualitySourcesInfo' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:QualitySources"
grep -q -F -e '_DX_RUST_CRATE_PROVIDERS' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:crate-map"
grep -q -F -e '_DX_FORWARD_PROVIDES' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:forward"
grep -q -F -e '_DX_CC_FORWARD_PROVIDES' rust/rules/defs.bzl || rust470_fail="$rust470_fail provider:cc-forward"
grep -q -F -e 'def rust_library' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:library"
grep -q -F -e 'def rust_binary' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:binary"
grep -q -F -e 'def rust_test' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:test"
grep -q -F -e 'def rust_proc_macro' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:proc-macro"
grep -q -F -e 'def rust_shared_library' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:shared"
grep -q -F -e 'def rust_static_library' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:static"
grep -q -F -e 'def dx_rust_crate' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:dx-crate"
grep -q -F -e 'def rustfmt_test' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:rustfmt"
grep -q -F -e 'def rust_clippy_test' rust/rules/defs.bzl || rust470_fail="$rust470_fail wrapper:clippy"
grep -q -F -e '@rules_rust//rust:defs.bzl' rust/rules/defs.bzl || rust470_fail="$rust470_fail upstream:load"
[[ -f "rust/rules/probe.bzl" ]] || rust470_fail="$rust470_fail fixture:probe"
grep -q -F -e 'dx_wrapper_subject' rust/rules/probe.bzl || rust470_fail="$rust470_fail fixture:subject"
grep -q -F -e 'dx_wrapper_cc_subject' rust/rules/probe.bzl || rust470_fail="$rust470_fail fixture:cc-subject"
[[ -f "rust/rules/wrapper_tests.bzl" ]] || rust470_fail="$rust470_fail fixture:wrapper-tests"
grep -q -F -e 'EXPECTED_OBSERVATIONS' rust/rules/wrapper_tests.bzl || rust470_fail="$rust470_fail fixture:observations"
grep -q -F -e 'preserved_deps=True' rust/rules/wrapper_tests.bzl || rust470_fail="$rust470_fail fixture:preserved-deps"
grep -q -F -e 'preserved_srcs=True' rust/rules/wrapper_tests.bzl || rust470_fail="$rust470_fail fixture:preserved-srcs"
grep -q -F -e 'tools_pinned=True' rust/rules/wrapper_tests.bzl || rust470_fail="$rust470_fail fixture:tools-pinned"
grep -q -F -e 'wrapper_has_quality_sources=True' rust/rules/wrapper_tests.bzl || rust470_fail="$rust470_fail fixture:quality-sources"
grep -q -F -e 'dx_wrapper_conformance_tests' rust/tests/fixtures/hello/BUILD.bazel || rust470_fail="$rust470_fail hello:conformance"
grep -q -F -e 'dx_wrapper_registry_tests' rust/tests/fixtures/hello/BUILD.bazel || rust470_fail="$rust470_fail hello:registry"
grep -q -F -e 'hello_lib_subject' rust/tests/fixtures/hello/BUILD.bazel || rust470_fail="$rust470_fail hello:lib-subject"
grep -q -F -e 'hello_cdylib_subject' rust/tests/fixtures/hello/BUILD.bazel || rust470_fail="$rust470_fail hello:cdylib-subject"
grep -q -F -e 'hello_staticlib_subject' rust/tests/fixtures/hello/BUILD.bazel || rust470_fail="$rust470_fail hello:staticlib-subject"
grep -q -F -e 'rust_library' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:library-kind"
grep -q -F -e 'rust_binary' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:binary-kind"
grep -q -F -e 'rust_test' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:test-kind"
grep -q -F -e 'rust_proc_macro' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:proc-macro-kind"
grep -q -F -e 'rust_shared_library' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:shared-kind"
grep -q -F -e 'rust_static_library' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:static-kind"
grep -q -F -e 'cargo_build_script' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:script-kind"
grep -q -F -e 'dx_rust_crate' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:dx-crate-kind"
grep -q -F -e '//rust/rules:defs.bzl' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:dx-load"
grep -q -F -e '//cargo:defs.bzl' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:cargo-load"
grep -q -F -e 'crate_deps' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:crate-deps"
grep -q -F -e '"aliases"' gazelle/rust/lang.go || rust470_fail="$rust470_fail gazelle:aliases"
[[ -f "gazelle/rust/parser.go" && -f "gazelle/rust/cargo.go" && -f "gazelle/rust/corpus.go" && -f "gazelle/rust/manifest.go" && -f "gazelle/rust/native_config.go" && -f "gazelle/rust/layout.go" ]] || rust470_fail="$rust470_fail gazelle:impl"
[[ -f "gazelle/rust/parser_test.go" && -f "gazelle/rust/cargo_test.go" && -f "gazelle/rust/manifest_test.go" && -f "gazelle/rust/layout_test.go" && -f "gazelle/rust/native_config_test.go" ]] || rust470_fail="$rust470_fail gazelle:tests"
[[ -f "gazelle/rust/testdata/cargo/crates/app/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:app-golden"
[[ -f "gazelle/rust/testdata/cargo/crates/cdylib/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:cdylib-golden"
[[ -f "gazelle/rust/testdata/cargo/crates/proc_macro/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:proc-macro-golden"
[[ -f "gazelle/rust/testdata/cargo/crates/scripted/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:scripted-golden"
[[ -f "gazelle/rust/testdata/cargo/crates/shapes/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:shapes-golden"
[[ -f "gazelle/rust/testdata/cargo/crates/staticlib/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:staticlib-golden"
[[ -f "gazelle/rust/testdata/source_only/crates/demo/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:source-only-golden"
[[ -f "gazelle/rust/testdata/merge/crates/merge/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:merge-golden"
[[ -f "gazelle/rust/testdata/native_config/crates/full/BUILD.out" ]] || rust470_fail="$rust470_fail gazelle:native-config-golden"
grep -q -F -e 'RustEnvPlanInfo' rust/env/plan.bzl || rust470_fail="$rust470_fail env:plan-info"
grep -q -F -e 'rust_env_plan' rust/env/plan.bzl || rust470_fail="$rust470_fail env:plan-rule"
grep -q -F -e 'EXPECTED_ENV_PLAN_OBSERVATIONS' rust/env/plan_tests.bzl || rust470_fail="$rust470_fail env:observations"
grep -q -F -e 'via_test_crate' rust/env/plan_tests.bzl || rust470_fail="$rust470_fail env:via-test-crate"
grep -q -F -e 'hello_lib_plan' rust/env/BUILD.bazel || rust470_fail="$rust470_fail env:lib-plan"
grep -q -F -e 'hello_cdylib_plan' rust/env/BUILD.bazel || rust470_fail="$rust470_fail env:cdylib-plan"
grep -q -F -e 'env_plan_tests' rust/env/BUILD.bazel || rust470_fail="$rust470_fail env:tests"
[[ -f "rust/tests/fixtures/hello/Cargo.toml" ]] || rust470_fail="$rust470_fail hello:manifest"
[[ -f "rust/tests/fixtures/hello/Cargo.lock" ]] || rust470_fail="$rust470_fail hello:lock"
[[ -f "rust/tests/fixtures/hello/src/lib.rs" && -f "rust/tests/fixtures/hello/src/main.rs" && -f "rust/tests/fixtures/hello/src/derive.rs" && -f "rust/tests/fixtures/hello/src/cdylib.rs" && -f "rust/tests/fixtures/hello/src/staticlib.rs" ]] || rust470_fail="$rust470_fail hello:sources"
[[ -f "cargo-bazel-lock.json" ]] || rust470_fail="$rust470_fail lock:cargo-bazel"
[[ -f "MODULE.bazel.lock" ]] || rust470_fail="$rust470_fail lock:module"
grep -q -F -e 'bazel_dep(name = "rules_rust", version = "0.74.0")' MODULE.bazel || rust470_fail="$rust470_fail module:rules-rust"
grep -q -F -e 'cargo_lockfile = "//rust/tests/fixtures/hello:Cargo.lock"' MODULE.bazel || rust470_fail="$rust470_fail module:cargo-lockfile"
grep -q -F -e 'lockfile = "//:cargo-bazel-lock.json"' MODULE.bazel || rust470_fail="$rust470_fail module:crate-lockfile"
grep -q -F -e 'Language Mapping Qualification' docs/generation/foundation-qualification.md || rust470_fail="$rust470_fail docs:generation"
grep -q -F -e 'Language Mapping Qualification' docs/environments/foundation-qualification.md || rust470_fail="$rust470_fail docs:environments"
grep -q -F -e '#470' docs/native-toolchains.md || rust470_fail="$rust470_fail docs:native"
if [[ -z "$rust470_fail" ]]; then
  ok
else
  bad "Rust provider/Gazelle/integration maps drifted:$rust470_fail"
fi

# Issue #948: admitted wrapper conformance stays pinned (unit contract tests
# alongside each admitted wrapper, wired in rules/BUILD.bazel over the hello
# wrapper-consumer fixtures, mirroring the Rust wrapper_tests pattern).
wrap948_fail=""
for lang in go java kotlin scala csharp fsharp cc; do
  [[ -f "$lang/rules/wrapper_tests.bzl" ]] || wrap948_fail="$wrap948_fail $lang:file"
  grep -q -F -e "${lang}_wrapper_contract_tests" "$lang/rules/wrapper_tests.bzl" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:macro"
  grep -q -F -e 'KNOWN_SEMANTIC_FILE_CLASSES' "$lang/rules/wrapper_tests.bzl" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:class"
  grep -q -F -e 'dx_forwarded_test_kwargs' "$lang/rules/wrapper_tests.bzl" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:test-kwargs"
  grep -q -F -e 'dx_effective_visibility' "$lang/rules/wrapper_tests.bzl" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:visibility"
  grep -q -F -e 'wrapper_contract_tests' "$lang/rules/BUILD.bazel" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:build"
  grep -q -F -e 'wrapper_tests.bzl' "$lang/rules/BUILD.bazel" 2>/dev/null || wrap948_fail="$wrap948_fail $lang:exports"
done
grep -q -F -e 'go_effective_srcs' go/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail go:helper"
grep -q -F -e 'java_javacopts_with_werror' java/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail java:helper"
grep -q -F -e 'kotlin_kotlinc_opts_with_werror' kotlin/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail kotlin:helper"
grep -q -F -e 'scala_scalacopts_with_werror' scala/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail scala:helper"
grep -q -F -e 'csharp_tfm_with_defaults' csharp/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail csharp:helper"
grep -q -F -e 'fsharp_tfm_with_defaults' fsharp/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail fsharp:helper"
grep -q -F -e 'cc_copts_with_werror' cc/rules/wrapper_tests.bzl || wrap948_fail="$wrap948_fail cc:helper"
if [[ -z "$wrap948_fail" ]]; then
  ok
else
  bad "admitted wrapper conformance drifted:$wrap948_fail"
fi

# -: Rust build-script hermetic defaults stay pinned (generation contract
# plus Gazelle emission plus focused Go fixtures).
script_fail=""
grep -q -F -e 'use_default_shell_env = False' docs/generation/rust.md || script_fail="$script_fail contract:shell-env"
grep -q -F -e 'use_cc_toolchain = True' docs/generation/rust.md || script_fail="$script_fail contract:cc"
grep -q -F -e 'emit_warnings = True' docs/generation/rust.md || script_fail="$script_fail contract:warnings"
grep -q -F -e 'use_cc_toolchain", 1' gazelle/rust/lang.go || script_fail="$script_fail lang:cc"
grep -q -F -e 'use_default_shell_env", 0' gazelle/rust/lang.go || script_fail="$script_fail lang:shell-env"
grep -q -F -e 'emit_warnings", true' gazelle/rust/lang.go || script_fail="$script_fail lang:warnings"
grep -q -F -e 'use_cc_toolchain' gazelle/rust/lang_test.go || script_fail="$script_fail test:cc"
grep -q -F -e 'use_default_shell_env' gazelle/rust/lang_test.go || script_fail="$script_fail test:shell-env"
grep -q -F -e 'emit_warnings' gazelle/rust/lang_test.go || script_fail="$script_fail test:warnings"
if [[ -z "$script_fail" ]]; then
  ok
else
  bad "build-script hermetic defaults drifted:$script_fail"
fi

# -: required-core quality adapter mappings stay pinned (Ty plus
# Ruff/pydoclint for Python; Biome/ESLint/Prettier/tsc for JS/TS).
quality_fail=""
grep -q -F -e '"ty": {"typecheck": ["python", "python_stub"]}' quality/adapters.bzl || quality_fail="$quality_fail ty:adapter"
grep -q -F -e '"ruff": {"format": ["python", "python_stub"], "lint": ["python", "python_stub"]}' quality/adapters.bzl || quality_fail="$quality_fail ruff:adapter"
grep -q -F -e '"pydoclint": {"lint": ["python", "python_stub"]}' quality/adapters.bzl || quality_fail="$quality_fail pydoclint:adapter"
grep -q -F -e '"biome": {' quality/adapters.bzl || quality_fail="$quality_fail biome:adapter"
grep -q -F -e '"eslint": {"lint": ["javascript", "jsx"]}' quality/adapters.bzl || quality_fail="$quality_fail eslint:adapter"
grep -q -F -e '"prettier": {"format": ["javascript", "json", "jsx", "typescript", "tsx"]}' quality/adapters.bzl || quality_fail="$quality_fail prettier:adapter"
grep -q -F -e '"tsc": {"typecheck": ["typescript", "tsx"]}' quality/adapters.bzl || quality_fail="$quality_fail tsc:adapter"
[[ -f "quality/adapter/src/parsers/ty.rs" ]] || quality_fail="$quality_fail ty:parser"
[[ -f "quality/adapter/src/parsers/ruff.rs" ]] || quality_fail="$quality_fail ruff:parser"
[[ -f "quality/adapter/src/parsers/biome.rs" ]] || quality_fail="$quality_fail biome:parser"
[[ -f "quality/adapter/src/parsers/eslint.rs" ]] || quality_fail="$quality_fail eslint:parser"
[[ -f "quality/adapter/src/parsers/prettier.rs" ]] || quality_fail="$quality_fail prettier:parser"
[[ -f "quality/adapter/src/parsers/tsc.rs" ]] || quality_fail="$quality_fail tsc:parser"
grep -q -F -e 'parse_ty' quality/adapter/src/parsers/ty.rs || quality_fail="$quality_fail ty:parse"
grep -q -F -e 'parse_biome' quality/adapter/src/parsers/biome.rs || quality_fail="$quality_fail biome:parse"
grep -q -F -e 'parse_eslint' quality/adapter/src/parsers/eslint.rs || quality_fail="$quality_fail eslint:parse"
if [[ -z "$quality_fail" ]]; then
  ok
else
  bad "required-core quality mappings drifted:$quality_fail"
fi

# -: required-core native gaps stay owned (no Supported claim; detail lives
# in owning issues).
gaps_fail=""
grep -q -F -e '#471' docs/native-toolchains.md || gaps_fail="$gaps_fail native:tracking"
grep -q -F -e 'Can the kept CC opt-out execute successfully?' docs/native-toolchains.md || gaps_fail="$gaps_fail native:cc-optout"
grep -q -F -e 'Can third-party scripts retain a declared hermetic closure?' docs/native-toolchains.md || gaps_fail="$gaps_fail native:shell-env"
grep -q -F -e 'Can bindgen/CXX use one upstream graph?' docs/native-toolchains.md || gaps_fail="$gaps_fail native:bindgen-cxx"
grep -q -F -e 'Can IDE setup preserve exact context' docs/native-toolchains.md || gaps_fail="$gaps_fail native:exact-target"
if grep -q -E -e '^\| .* \| Supported' docs/product/support-matrix.md; then
  gaps_fail="$gaps_fail unexpected-supported"
fi
if [[ -z "$gaps_fail" ]]; then
  ok
else
  bad "required-core native gaps unowned:$gaps_fail"
fi

# -: admitted test runners stay pinned (go test with package embed,
# ScalaTest via the managed route, JUnit 4 seed for JVM, plain assert seed
# for cc plus hello smoke for csharp/fsharp, with the xUnit v3 4.0.0 mapping
# qualified under plus the GoogleTest v1.18.0 mapping qualified under
# plus the ScalaTest 3.2.20 mapping qualified under).
runner_fail=""
grep -q -F -e 'go_test' go/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail go:kind"
grep -q -F -e 'embed' go/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail go:embed"
grep -q -F -e 'cc_test' cc/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail cc:kind"
grep -q -F -e 'assert' cc/tests/fixtures/hello/hello_test.cc || runner_fail="$runner_fail cc:plain"
grep -q -F -e 'java_test' java/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail java:kind"
grep -q -F -e 'test_class' java/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail java:class"
grep -q -F -e 'org.junit.Test' java/tests/fixtures/hello/HelloTest.java || runner_fail="$runner_fail java:junit4"
grep -q -F -e 'kotlin_test' kotlin/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail kotlin:kind"
grep -q -F -e '@maven//:junit_junit' kotlin/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail kotlin:maven"
grep -q -F -e 'org.junit.Test' kotlin/tests/fixtures/hello/HelloTest.kt || runner_fail="$runner_fail kotlin:junit4"
grep -q -F -e 'junit:junit:4.13.2' MODULE.bazel || runner_fail="$runner_fail module:junit4"
grep -q -F -e 'scala_test' scala/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail scala:kind"
grep -q -F -e 'AnyFlatSpec' scala/tests/fixtures/hello/HelloTest.scala || runner_fail="$runner_fail scala:scalatest"
grep -q -F -e '@maven//:org_scalatest_scalatest_2_13' scala/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail scala:maven"
grep -q -F -e 'org.scalatest:scalatest_2.13:3.2.20' MODULE.bazel || runner_fail="$runner_fail module:scalatest"
grep -q -F -e 'scala_version = "2.13.18"' MODULE.bazel || runner_fail="$runner_fail module:scala-version"
grep -q -F -e 'SCALATEST_VERSION = "3.2.20"' scala/tests/fixtures/scalatest/pins.bzl || runner_fail="$runner_fail scala:pin"
grep -q -F -e 'scala_test' scala/tests/fixtures/scalatest/pins.bzl || runner_fail="$runner_fail scala:map"
grep -q -F -e 'scalatest_qualification' docs/generation/foundation-qualification.md || runner_fail="$runner_fail gen:scalatest-qual"
grep -q -F -e 'csharp_test' csharp/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail csharp:kind"
grep -q -F -e 'static int Main' csharp/tests/fixtures/hello/HelloTest.cs || runner_fail="$runner_fail csharp:plain"
grep -q -F -e 'fsharp_test' fsharp/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail fsharp:kind"
grep -q -F -e 'EntryPoint' fsharp/tests/fixtures/hello/HelloTest.fs || runner_fail="$runner_fail fsharp:plain"
grep -q -F -e 'xUnit' csharp/rules/defs.bzl || runner_fail="$runner_fail csharp:open"
grep -q -F -e 'xUnit' fsharp/rules/defs.bzl || runner_fail="$runner_fail fsharp:open"
grep -q -F -e 'nuget xunit.v3 4.0.0' third_party/dotnet/paket.dependencies || runner_fail="$runner_fail paket:xunit"
grep -q -F -e 'nuget xunit.analyzers 2.0.0' third_party/dotnet/paket.dependencies || runner_fail="$runner_fail paket:analyzers"
grep -q -F -e 'xunit.v3 (4.0)' third_party/dotnet/paket.lock || runner_fail="$runner_fail lock:xunit"
grep -q -F -e 'csharp_test' csharp/tests/fixtures/xunit/BUILD.bazel || runner_fail="$runner_fail csharp-xunit:kind"
grep -q -F -e '@paket.main//xunit.v3' csharp/tests/fixtures/xunit/BUILD.bazel || runner_fail="$runner_fail csharp-xunit:pin"
grep -q -F -e '[Fact]' csharp/tests/fixtures/xunit/GreeterTest.cs || runner_fail="$runner_fail csharp-xunit:fact"
grep -q -F -e 'ConsoleRunner.Run' csharp/tests/fixtures/xunit/XunitEntryPoint.cs || runner_fail="$runner_fail csharp-xunit:entry"
grep -q -F -e 'fsharp_test' fsharp/tests/fixtures/xunit/BUILD.bazel || runner_fail="$runner_fail fsharp-xunit:kind"
grep -q -F -e '@paket.main//xunit.v3' fsharp/tests/fixtures/xunit/BUILD.bazel || runner_fail="$runner_fail fsharp-xunit:pin"
grep -q -F -e '[<Fact>]' fsharp/tests/fixtures/xunit/GreeterTest.fs || runner_fail="$runner_fail fsharp-xunit:fact"
grep -q -F -e 'ConsoleRunner.Run' fsharp/tests/fixtures/xunit/XunitEntryPoint.fs || runner_fail="$runner_fail fsharp-xunit:entry"
grep -q -F -e 'bazel_dep(name = "googletest", version = "1.18.0")' MODULE.bazel || runner_fail="$runner_fail module:gtest"
grep -q -F -e 'GoogleTest' cc/rules/defs.bzl || runner_fail="$runner_fail cc:gtest-open"
grep -q -F -e 'GTEST_VERSION = "1.18.0"' cc/tests/fixtures/googletest/pins.bzl || runner_fail="$runner_fail gtest:pin"
grep -q -F -e '"-std=c++17"' cc/tests/fixtures/googletest/pins.bzl || runner_fail="$runner_fail gtest:floor"
grep -q -F -e 'cc_test' cc/tests/fixtures/googletest/BUILD.bazel || runner_fail="$runner_fail gtest:kind"
grep -q -F -e '@googletest//:gtest_main' cc/tests/fixtures/googletest/BUILD.bazel || runner_fail="$runner_fail gtest:dep"
grep -q -F -e '"-std=c++17"' cc/tests/fixtures/googletest/BUILD.bazel || runner_fail="$runner_fail gtest:copts"
grep -q -F -e 'TEST(GreeterTest' cc/tests/fixtures/googletest/greeter_test.cc || runner_fail="$runner_fail gtest:test"
grep -q -F -e 'static_assert(__cplusplus >= 201703L' cc/tests/fixtures/googletest/greeter_test.cc || runner_fail="$runner_fail gtest:static-assert"
grep -q -F -e 'std::optional' cc/tests/fixtures/googletest/greeter.h || runner_fail="$runner_fail gtest:optional"
grep -q -F -e 'ruby_test' ruby/tests/fixtures/hello/BUILD.bazel || runner_fail="$runner_fail ruby:kind"
grep -q -F -e 'Hello.hello' ruby/tests/fixtures/hello/hello_test.rb || runner_fail="$runner_fail ruby:plain"
grep -q -F -e 'RSPEC_VERSION = "3.13.0"' ruby/tests/fixtures/rspec/pins.bzl || runner_fail="$runner_fail rspec:pin"
grep -q -F -e 'ruby_test' ruby/tests/fixtures/rspec/BUILD.bazel || runner_fail="$runner_fail rspec:kind"
grep -q -F -e '@bundle//bin:rspec' ruby/tests/fixtures/rspec/BUILD.bazel || runner_fail="$runner_fail rspec:dep"
grep -q -F -e 'RSpec.describe' ruby/tests/fixtures/rspec/greeter_spec.rb || runner_fail="$runner_fail rspec:spec"
grep -q -F -e 'rspec_qualification' docs/generation/foundation-qualification.md || runner_fail="$runner_fail gen:rspec-qual"
if [[ -z "$runner_fail" ]]; then
  ok
else
  bad "admitted test runners drifted:$runner_fail"
fi

# -: admitted lock wiring stays pinned (JVM shares maven_install.json
# fail-closed qualified seed-only under,.NET shares paket.main qualified
# seed-only under, Go go.mod/go.sum qualified seed-only under,
# C/C++ hash wiring qualified seed-only under with no ecosystem lockfile).
lock304_fail=""
grep -q -F -e 'maven_install.json' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:maven"
grep -q -F -e 'paket.lock' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:paket"
grep -q -F -e 'go.mod' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:go"
grep -q -F -e 'Gemfile.lock' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:ruby"
grep -q -F -e 'C/C++ none' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:cc"
grep -q -F -e '@paket.main//fsharp.core' fsharp/tests/fixtures/hello/BUILD.bazel || lock304_fail="$lock304_fail fsharp:paket"
grep -q -F -e 'paket.main' MODULE.bazel || lock304_fail="$lock304_fail module:paket"
grep -q -F -e 'lock_file = "//third_party/jvm:maven_install.json"' MODULE.bazel || lock304_fail="$lock304_fail module:maven"
grep -q -F -e 'FSHARP_CORE_VERSION = "10.1.201"' csharp/tests/fixtures/paket/pins.bzl || lock304_fail="$lock304_fail paket:pin"
grep -q -F -e 'packages.lock.json rejected' csharp/tests/fixtures/paket/pins.bzl || lock304_fail="$lock304_fail paket:rejected"
grep -q -F -e 'paket_qualification' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:paket-qual"
grep -q -F -e 'RULES_JVM_EXTERNAL_VERSION = "7.1"' third_party/jvm/pins.bzl || lock304_fail="$lock304_fail maven:pin"
grep -q -F -e 'fail_if_repin_required = True' MODULE.bazel || lock304_fail="$lock304_fail maven:fail-closed"
grep -q -F -e 'maven_lock_qualification' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail maven:gen"
grep -q -F -e 'CC_HASH_ATTR = "sha256"' cc/tests/fixtures/hermetic/pins.bzl || lock304_fail="$lock304_fail cc:pin"
grep -q -F -e 'system packages rejected' cc/tests/fixtures/hermetic/pins.bzl || lock304_fail="$lock304_fail cc:rejected"
grep -q -F -e 'cc_hermetic_qualification' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:cc-qual"
grep -q -F -e 'GO_CMP_VERSION = "v0.6.0"' go/tests/fixtures/godeps/pins.bzl || lock304_fail="$lock304_fail godeps:pin"
grep -q -F -e 'hand module tags rejected' go/tests/fixtures/godeps/pins.bzl || lock304_fail="$lock304_fail godeps:rejected"
grep -q -F -e 'godeps_qualification' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:godeps-qual"
grep -q -F -e 'RSPEC_VERSION = "3.13.0"' ruby/tests/fixtures/rspec/pins.bzl || lock304_fail="$lock304_fail rspec:pin"
grep -q -F -e 'git gems rejected' ruby/tests/fixtures/gems/pins.bzl || lock304_fail="$lock304_fail gems:rejected"
grep -q -F -e 'gems_qualification' docs/generation/foundation-qualification.md || lock304_fail="$lock304_fail gen:gems-qual"
if [[ -z "$lock304_fail" ]]; then
  ok
else
  bad "admitted lock wiring drifted:$lock304_fail"
fi

# Lock currency recheck (issue #932): directives stay in parity with a
# dated verification, not just existence. Go SDK minor stays >= every
# go.mod directive in the tree (third_party floor plus examples); the
# buildtools require entry matches the godeps pin; Paket/Maven coordinates
# match their pins; each currency recheck date stays present and valid.
currency_fail=""
sdk_minor="$(grep -o -E -e 'go_sdk\.download\(version = "[^"]+"' MODULE.bazel | head -1 | grep -o -E -e '[0-9]+\.[0-9]+\.[0-9]+' || true)"
sdk_mm="$(echo "$sdk_minor" | cut -d. -f1,2)"
for gomod in third_party/go/go.mod examples/adopt-go/go.mod; do
  directive="$(grep -o -E -e '^go [0-9]+\.[0-9]+(\.[0-9]+)?' "$gomod" 2>/dev/null | head -1 | cut -d' ' -f2 || true)"
  if [[ -z "$directive" ]]; then
    currency_fail="$currency_fail $gomod:missing-directive"
    continue
  fi
  d_mm="$(echo "$directive" | cut -d. -f1,2)"
  s_maj="${sdk_mm%%.*}"; s_min="${sdk_mm#*.}"
  d_maj="${d_mm%%.*}"; d_min="${d_mm#*.}"
  if [[ "$s_maj" -gt "$d_maj" ]] || { [[ "$s_maj" == "$d_maj" ]] && [[ "$s_min" -ge "$d_min" ]]; }; then
    true
  else
    currency_fail="$currency_fail $gomod:directive-$directive-ahead-of-sdk-$sdk_minor"
  fi
done
buildtools_gomod="$(grep -o -E -e 'buildtools v[^ ]+' third_party/go/go.mod | head -1 | cut -d' ' -f2 || true)"
buildtools_pin="$(grep -o -E -e '^BUILDTOOLS_VERSION = "[^"]+"' go/tests/fixtures/godeps/pins.bzl | head -1 | cut -d'"' -f2 || true)"
[[ -n "$buildtools_gomod" && "$buildtools_gomod" == "$buildtools_pin" ]] || currency_fail="$currency_fail buildtools:$buildtools_gomod-vs-$buildtools_pin"
fsharp_gomod="$(grep -o -E -e 'nuget FSharp.Core [^ ]+' third_party/dotnet/paket.dependencies | head -1 | awk '{print $3}' || true)"
fsharp_pin="$(grep -o -E -e '^FSHARP_CORE_VERSION = "[^"]+"' csharp/tests/fixtures/paket/pins.bzl | head -1 | cut -d'"' -f2 || true)"
[[ -n "$fsharp_gomod" && "$fsharp_gomod" == "$fsharp_pin" ]] || currency_fail="$currency_fail fsharp:$fsharp_gomod-vs-$fsharp_pin"
rspec_gemfile="$(grep -o -E -e 'gem \"rspec\", \"[^\"]+\"' third_party/ruby/Gemfile | head -1 | grep -o -E -e '[0-9]+\.[0-9]+\.[0-9]+' || true)"
rspec_pin="$(grep -o -E -e '^RSPEC_VERSION = "[^"]+"' ruby/tests/fixtures/rspec/pins.bzl | head -1 | cut -d'"' -f2 || true)"
[[ -n "$rspec_gemfile" && "$rspec_gemfile" == "$rspec_pin" ]] || currency_fail="$currency_fail rspec:$rspec_gemfile-vs-$rspec_pin"
for pinfile in go/tests/fixtures/godeps/pins.bzl third_party/jvm/pins.bzl csharp/tests/fixtures/paket/pins.bzl ruby/tests/fixtures/rspec/pins.bzl ruby/tests/fixtures/gems/pins.bzl; do
  date="$(grep -o -E -e '_CURRENCY_RECHECK = "[^"]+"' "$pinfile" | head -1 | cut -d'"' -f2 || true)"
  if [[ "$date" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] && [[ "$date" > "2026-01-01" ]] && [[ "$date" < "2027-06-01" ]]; then
    true
  else
    currency_fail="$currency_fail $pinfile:recheck-$date"
  fi
done
if [[ -z "$currency_fail" ]]; then
  ok
else
  bad "lock currency drifted:$currency_fail"
fi

# -: admitted quality classification stays pinned (families exist,
# no adapter claims admitted classes yet; adapter side qualified under 
# with deferred ADR 0019 routes).
class304_fail=""
for cls in go c cpp java kotlin scala csharp fsharp; do
  grep -q -F -e "\"$cls\":" quality/adapters.bzl || class304_fail="$class304_fail $cls:family"
done
grep -q -F -e 'no adapter claims go yet' quality/adapters.bzl || class304_fail="$class304_fail go:open"
grep -q -F -e 'no adapter claims c/cpp yet' quality/adapters.bzl || class304_fail="$class304_fail cc:open"
grep -q -F -e 'adapter claims java yet' quality/adapters.bzl || class304_fail="$class304_fail java:open"
grep -q -F -e 'no adapter claims kotlin' quality/adapters.bzl || class304_fail="$class304_fail kotlin:open"
grep -q -F -e 'no adapter claims scala yet' quality/adapters.bzl || class304_fail="$class304_fail scala:open"
grep -q -F -e 'no adapter claims csharp yet' quality/adapters.bzl || class304_fail="$class304_fail csharp:open"
grep -q -F -e 'no adapter claims fsharp yet' quality/adapters.bzl || class304_fail="$class304_fail fsharp:open"
grep -q -F -e '#476-#484' docs/tools/foundation-qualification.md || class304_fail="$class304_fail tools:tracking"
grep -q -F -e 'issue #307' docs/tools/foundation-qualification.md || class304_fail="$class304_fail tools:adapter-tracking"
if [[ -z "$class304_fail" ]]; then
  ok
else
  bad "admitted quality classification drifted:$class304_fail"
fi

# -: admitted foundations stay owned (docs track -; C/C++ MSVC block
# owned with no Supported claim).
own304_fail=""
grep -q -F -e '#476-#484' docs/generation/foundation-qualification.md || own304_fail="$own304_fail gen:tracking"
grep -q -F -e '#476-#484' docs/environments/foundation-qualification.md || own304_fail="$own304_fail env:tracking"
grep -q -F -e '#476-#484' docs/native-toolchains.md || own304_fail="$own304_fail native:tracking"
grep -q -F -e 'MSVC interop' docs/native-toolchains.md || own304_fail="$own304_fail native:msvc"
grep -q -F -e 'SDK licensing' docs/native-toolchains.md || own304_fail="$own304_fail native:sdk"
if grep -q -E -e '^\| .* \| Supported' docs/product/support-matrix.md; then
  own304_fail="$own304_fail unexpected-supported"
fi
if [[ -z "$own304_fail" ]]; then
  ok
else
  bad "admitted foundations unowned:$own304_fail"
fi

# ADR 0032: Ruby plus PowerShell delivered, Swift still absent (no swift
# dirs, wrappers, Gazelle extensions, env plans, hello builds, or MODULE deps;
# ruby/ plus powershell/ dirs plus MODULE deps delivered).
abs305_fail=""
for d in swift; do
  [[ ! -d "$d" ]] || abs305_fail="$abs305_fail $d:dir"
  [[ ! -d "gazelle/$d" ]] || abs305_fail="$abs305_fail $d:gazelle"
  [[ ! -f "$d/rules/defs.bzl" ]] || abs305_fail="$abs305_fail $d:wrapper"
  [[ ! -f "$d/env/plan.bzl" ]] || abs305_fail="$abs305_fail $d:env"
  [[ ! -f "$d/tests/fixtures/hello/BUILD.bazel" ]] || abs305_fail="$abs305_fail $d:hello"
done
# Ruby plus PowerShell delivery: dirs, wrappers, env plans, hello builds
# (plus Ruby Gazelle extension plus RSpec fixtures, plus PowerShell Pester
# plus Gallery fixtures), and MODULE deps land here.
for d in ruby powershell; do
  [[ -d "$d" ]] || abs305_fail="$abs305_fail $d:dir-missing"
  [[ -f "$d/rules/defs.bzl" ]] || abs305_fail="$abs305_fail $d:wrapper-missing"
  [[ -f "$d/env/plan.bzl" ]] || abs305_fail="$abs305_fail $d:env-missing"
  [[ -f "$d/tests/fixtures/hello/BUILD.bazel" ]] || abs305_fail="$abs305_fail $d:hello-missing"
done
[[ -d "gazelle/ruby" ]] || abs305_fail="$abs305_fail ruby:gazelle-missing"
if ! grep -q -F -e 'rules_ruby' MODULE.bazel; then
  abs305_fail="$abs305_fail module:rules_ruby-missing"
fi
if ! grep -q -F -e 'rules_powershell' MODULE.bazel; then
  abs305_fail="$abs305_fail module:rules_powershell-missing"
fi
if grep -q -i -F -e 'swift' MODULE.bazel; then
  abs305_fail="$abs305_fail module:swift"
fi
grep -q -F -e 'no `swift/`' docs/generation/foundation-qualification.md || abs305_fail="$abs305_fail gen:absence"
grep -q -F -e 'no `swift/`' docs/environments/foundation-qualification.md || abs305_fail="$abs305_fail env:absence"
if [[ -z "$abs305_fail" ]]; then
  ok
else
  bad "deferred foundation absence drifted:$abs305_fail"
fi

# ADR 0032: Ruby/PowerShell quality classification stays pinned (families exist,
# RuboCop/StandardRB plus PSScriptAnalyzer claimed; parity no longer defers
# ruby/powershell; SwiftFormat plus Bandit stay unclaimed pending #800/#801).
class305_fail=""
for cls in ruby powershell; do
  grep -q -F -e "\"$cls\":" quality/adapters.bzl || class305_fail="$class305_fail $cls:family"
done
grep -q -F -e '"rubocop":' quality/adapters.bzl || class305_fail="$class305_fail missing:rubocop"
grep -q -F -e '"standardrb":' quality/adapters.bzl || class305_fail="$class305_fail missing:standardrb"
grep -q -F -e '"psscriptanalyzer":' quality/adapters.bzl || class305_fail="$class305_fail missing:psscriptanalyzer"
if grep -q -F -e '"ruby": ["ADR 0019"' quality/parity_tests.bzl; then
  class305_fail="$class305_fail unexpected:ruby-parity"
fi
if grep -q -F -e '"powershell": ["ADR 0019"' quality/parity_tests.bzl; then
  class305_fail="$class305_fail unexpected:powershell-parity"
fi
if grep -q -F -e '"swiftformat":' quality/adapters.bzl; then
  class305_fail="$class305_fail unexpected:swiftformat"
fi
if grep -q -F -e '"bandit":' quality/adapters.bzl; then
  class305_fail="$class305_fail unexpected:bandit"
fi
if [[ -z "$class305_fail" ]]; then
  ok
else
  bad "deferred quality classification drifted:$class305_fail"
fi

# ADR 0032: retained cohorts plus exclusions stay pinned (Ruby closure,
# PowerShell module+runtime, Swift/SwiftFormat exclusion plus Bandit
# re-selection, host-toolchain fallback never approved).
cohort305_fail=""
grep -q -F -e 'Release-assembled Ruby closure' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:ruby-route"
grep -q -F -e 'RuboCop, StandardRB' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:rubocop"
grep -q -F -e 'Exact upstream module plus portable PowerShell runtime' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:pwsh-route"
grep -q -F -e 'PSScriptAnalyzer' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:psscript"
grep -q -F -e 'Decided route: RuboCop and StandardRB take the' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:ruby-decided"
grep -q -F -e 'Decided route: PSScriptAnalyzer takes the' docs/tools/tool-acquisition.md || cohort305_fail="$cohort305_fail acquire:pwsh-decided"
grep -q -F -e 'Swift, including SwiftFormat, is excluded from v1 by' docs/tools/tool-baseline.md || cohort305_fail="$cohort305_fail baseline:swift"
grep -q -F -e 'that route is forbidden' docs/tools/tool-baseline.md || cohort305_fail="$cohort305_fail baseline:host-forbidden"
grep -q -F -e 'Bandit excluded from v1 by' docs/tools/tool-baseline.md || cohort305_fail="$cohort305_fail baseline:bandit"
grep -q -F -e 'retained RuboCop/StandardRB plus' docs/tools/foundation-qualification.md || cohort305_fail="$cohort305_fail tools:retained"
grep -q -F -e 'Swift/SwiftFormat stay excluded' docs/tools/foundation-qualification.md || cohort305_fail="$cohort305_fail tools:excluded"
if [[ -z "$cohort305_fail" ]]; then
  ok
else
  bad "deferred retained cohorts drifted:$cohort305_fail"
fi

# ADR 0032: Ruby plus PowerShell delivered plus Swift record stays owned (docs cite
# ADR 0032; remaining gaps owned with no Supported claim; reconsideration
# needs a new decision).
own305_fail=""
grep -q -F -e 'decided by [ADR 0032]' docs/generation/foundation-qualification.md || own305_fail="$own305_fail gen:tracking"
grep -q -F -e 'decided by [ADR 0032]' docs/environments/foundation-qualification.md || own305_fail="$own305_fail env:tracking"
grep -q -F -e 'decided by [ADR 0032]' docs/tools/foundation-qualification.md || own305_fail="$own305_fail tools:tracking"
grep -q -F -e 'Bundle contents, lock inputs' docs/tools/tool-acquisition.md || own305_fail="$own305_fail acquire:ruby-gaps"
grep -q -F -e 'console-parse versus library-API' docs/tools/tool-acquisition.md || own305_fail="$own305_fail acquire:pwsh-gaps"
grep -q -F -e 'no adapter claims `ruby` yet' docs/tools/tool-acquisition.md || own305_fail="$own305_fail acquire:ruby-open"
grep -q -F -e 'no adapter claims `powershell` yet' docs/tools/tool-acquisition.md || own305_fail="$own305_fail acquire:pwsh-open"
grep -q -F -e 'issue #307' docs/tools/foundation-qualification.md || own305_fail="$own305_fail tools:adapter-tracking"
if grep -q -E -e '^\| .* \| Supported' docs/product/support-matrix.md; then
  own305_fail="$own305_fail unexpected-supported"
fi
if [[ -z "$own305_fail" ]]; then
  ok
else
  bad "deferred/excluded record unowned:$own305_fail"
fi

dx_test_summary "foundation maps harness"
