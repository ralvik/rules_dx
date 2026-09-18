#!/usr/bin/env bash
# Foundation-mapping guards (issues #7, #8; relates #6, #12).
#
# Rust/Python/JS-TS foundations ship thin wrappers + Gazelle + env plans;
# exact provider/import/lock/tool-graph proofs are pinned here for #7.
# Vue/Svelte/Astro/MDX ship named adapters over upstream parsers with an
# end-to-end fixture approach; exact parser/provider/region/dependency/
# test/env/quality mappings plus composition evidence stay open under #8.
# Class-to-family taxonomy stays open under #6; native config binding +
# CI scope extension stay open under #12.
#
# This harness machine-checks the qualified mappings: owner links, adapter
# boundaries, provider advertisement, fixture markers, upstream pins,
# Gazelle fixtures, env plans, lock authority, and Ty provenance.
# Upstream choices are kept; no switch is approved here.
#
# Versioned here, run by CI via `bazel run //tools/ci:foundation_maps`,
# following //tools/ci:wrapper_sources.
set -euo pipefail

if [[ -n "${BUILD_WORKSPACE_DIRECTORY:-}" ]]; then
  workspace="$BUILD_WORKSPACE_DIRECTORY"
else
  workspace="$(git rev-parse --show-toplevel)"
fi
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# #7: minimum external-consumer example workspaces exist (Rust, Python,
# JS/TS); the rest of #85's per-foundation set builds on these.
if [[ -d "examples/adopt-rust" && -d "examples/adopt-python" \
  && -d "examples/adopt-js-ts" ]]; then
  ok
else
  bad "minimum per-foundation example workspaces missing (adopt-rust/python/js-ts)"
fi

# #7: language wrappers advertise QualitySourcesInfo ( aspects gate on it).
langs_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
  if ! grep -q -F -e 'QualitySourcesInfo' "$lang/rules/defs.bzl" 2>/dev/null; then
    langs_missing="$langs_missing $lang"
  fi
done
if [[ -z "$langs_missing" ]]; then
  ok
else
  bad "language wrappers lost QualitySourcesInfo:$langs_missing"
fi

# #7: exact provider mappings stay pinned in each wrapper.
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
if [[ -z "$provider_fail" ]]; then
  ok
else
  bad "language provider mappings drifted:$provider_fail"
fi

# #7: upstream choices stay pinned; wrappers match MODULE.bazel pins.
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
if [[ -z "$upstream_fail" ]]; then
  ok
else
  bad "upstream choices drifted:$upstream_fail"
fi

# #7: Gazelle recognizer fixtures stay present (parser/lang/naming plus
# focused tests per language).
gazelle_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
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

# #7: environment plans stay present (plan + focused fixtures per language).
env_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
  if [[ ! -f "$lang/env/plan.bzl" || ! -f "$lang/env/plan_tests.bzl" ]]; then
    env_missing="$env_missing $lang"
  fi
done
if [[ -z "$env_missing" ]]; then
  ok
else
  bad "environment plans missing:$env_missing"
fi

# #7: hello builds stay present as wrapper consumers (import/search-path
# proof per language).
hello_missing=""
for lang in rust python javascript typescript go java kotlin scala csharp fsharp cc; do
  if [[ ! -f "$lang/hello/BUILD.bazel" ]]; then
    hello_missing="$hello_missing $lang:BUILD"
  elif ! grep -q -F -e "$lang/rules:defs.bzl" "$lang/hello/BUILD.bazel" && ! grep -q -F -e "javascript/rules:defs.bzl" "$lang/hello/BUILD.bazel"; then
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

# #7: lock authority stays checked in (fail-closed wiring in MODULE.bazel).
# Go hello is stdlib-only with no ecosystem lock; C/C++ has no ecosystem
# lockfile (every http_archive carries sha256/integrity).
locks_missing=""
[[ -f "rust/hello/Cargo.lock" ]] || locks_missing="$locks_missing Cargo.lock"
[[ -f "cargo-bazel-lock.json" ]] || locks_missing="$locks_missing cargo-bazel-lock"
[[ -f "MODULE.bazel.lock" ]] || locks_missing="$locks_missing MODULE.bazel.lock"
[[ -f "python/hello/uv.lock" ]] || locks_missing="$locks_missing uv.lock"
[[ -f "quality/tools/python/uv.lock" ]] || locks_missing="$locks_missing tools-uv.lock"
[[ -f "pnpm-lock.yaml" ]] || locks_missing="$locks_missing pnpm-lock"
[[ -f "quality/tools/javascript/pnpm-lock.yaml" ]] || locks_missing="$locks_missing tools-pnpm-lock"
[[ -f "third_party/jvm/maven_install.json" ]] || locks_missing="$locks_missing maven_install"
[[ -f "third_party/dotnet/paket.lock" ]] || locks_missing="$locks_missing paket.lock"
[[ -f "third_party/dotnet/paket.dependencies" ]] || locks_missing="$locks_missing paket.dependencies"
grep -q -F -e 'fail_if_repin_required' MODULE.bazel || locks_missing="$locks_missing fail-closed"
if [[ -z "$locks_missing" ]]; then
  ok
else
  bad "lock authority missing:$locks_missing"
fi

# #7: Ty artifact provenance stays pinned (plus core tool artifacts).
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

# #8: all four framework wrappers advertise QualitySourcesInfo.
fw_missing=""
for fw in vue svelte astro mdx; do
  if ! grep -q -F -e 'QualitySourcesInfo' "$fw/rules/defs.bzl" 2>/dev/null; then
    fw_missing="$fw_missing $fw"
  fi
done
if [[ -z "$fw_missing" ]]; then
  ok
else
  bad "framework wrappers lost QualitySourcesInfo:$fw_missing"
fi

# #8: mixed-framework composition fixture stays marked (M21), distinct
# from external-consumer workspaces.
if [[ -d "examples/mixed/hello" ]] \
  && grep -q -F -e 'M21' examples/mixed/hello/BUILD.bazel; then
  ok
else
  bad "mixed composition fixture lost its M21 marker"
fi

# #12: wrapper-sources pin + ownership audits stay versioned.
if [[ -f "tools/ci/wrapper_sources.sh" && -f "tools/ci/code_ownership.sh" ]]; then
  ok
else
  bad "wrapper_sources/code_ownership harnesses missing"
fi

echo "foundation maps harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
