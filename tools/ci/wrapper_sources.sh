#!/usr/bin/env bash
# Wrapper QualitySourcesInfo harness (lane A,,; relates).
#
# Production code rides normal targets (not parallel corpus lists), and
# every language/framework wrapper advertises `QualitySourcesInfo`
# normalized from its direct `srcs` so quality aspects can gate on it.
# Code files never ride corpus lists; corpus stays for target-less files
# only (docs, BUILD files, configs).
#
# This harness machine-checks the static half verifiable on a clean tree
# today: all eleven language wrappers plus the four
# framework wrappers advertise `QualitySourcesInfo`, load the shared
# forwarding helper (single-sourced normalization, never per-language
# reimplementation), bind wrapper tests, and the corpus/code-ownership
# split stays honest (no code extensions in corpus targets, no docs
# extensions in the code-ownership scope).
#
# Versioned here, run by CI via `bazel run //tools/ci:wrapper_sources`,
# following //tools/ci:depcheck_contract.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# All eleven language wrappers advertise QualitySourcesInfo normalized
# from direct srcs (single toolchain source of truth per wrapper).
langs=(rust python javascript typescript go java kotlin scala csharp fsharp cc)
for lang in "${langs[@]}"; do
  defs="$lang/rules/defs.bzl"
  if [[ -f "$defs" ]] && grep -q -F -e 'QualitySourcesInfo' "$defs"; then
    ok
  else
    bad "$defs missing or lost its QualitySourcesInfo advertisement"
  fi
done

# Framework wrappers (Vue/Svelte/Astro/MDX) advertise the same boundary;
# composition regions stay open per, but the source-ownership
# provider must already be present so aspects can gate on it.
pass_frameworks=0
for fw in vue svelte astro mdx; do
  defs="$fw/rules/defs.bzl"
  if [[ -f "$defs" ]] && grep -q -F -e 'QualitySourcesInfo' "$defs"; then
    pass_frameworks=$((pass_frameworks + 1))
  else
    bad "$defs missing or lost its QualitySourcesInfo advertisement"
  fi
done
if [[ "$pass_frameworks" -eq 4 ]]; then
  ok
else
  bad "framework wrapper coverage incomplete ($pass_frameworks/4)"
fi

# Normalization stays single-sourced: wrappers load the shared
# forwarding helper instead of reimplementing provider construction.
if grep -q -F -e 'libs/starlark' python/rules/defs.bzl &&
  grep -q -F -e 'libs/starlark' javascript/rules/defs.bzl &&
  grep -q -F -e 'libs/starlark' rust/rules/defs.bzl; then
  ok
else
  bad "language wrappers drifted off the shared forwarding helper"
fi

# Lane-A native-config plumbing: aspect_hints ride the public
# QualitySourcesInfo owner across every wrapper family (shared dx_wrap
# plus custom binary/test forwarders), so own-tree runs bind workspace-level
# native policy exactly like corpus targets bind their local configs.
if grep -q -F -e 'aspect_hints' libs/starlark/wrapper.bzl &&
  grep -q -F -e 'aspect_hints' go/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' java/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' kotlin/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' scala/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' csharp/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' fsharp/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' cc/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' python/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' rust/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' javascript/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' typescript/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' vue/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' svelte/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' astro/rules/defs.bzl &&
  grep -q -F -e 'aspect_hints' mdx/rules/defs.bzl; then
  ok
else
  bad "wrappers lost their lane-A aspect_hints forwarder plumbing"
fi

# Lane-A workspace-level native policy: root ruff/biome/rustfmt
# configs exist as checked-in sources with proof bindings on normal targets.
if [[ -f "ruff.toml" && -f "biome.json" && -f "rustfmt.toml" ]] &&
  grep -q -F -e 'ruff_config' BUILD.bazel &&
  grep -q -F -e 'biome_config' BUILD.bazel &&
  grep -q -F -e 'aspect_hints = ["//:ruff_config"]' python/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints = ["//:biome_config"]' javascript/tests/fixtures/hello/BUILD.bazel &&
  grep -q -F -e 'aspect_hints = ["//:rustfmt_config"]' rust/tests/fixtures/hello/BUILD.bazel; then
  ok
else
  bad "lane-A workspace-level native policy binding missing (root configs + proof aspect_hints)"
fi

# Libraries route via dx_wrap so aspect_hints (and CC hdrs) reach the
# QualitySourcesInfo owner, not only the private upstream.
if grep -q -F -e 'dx_wrap(name, _cc_library' cc/rules/defs.bzl; then
  ok
else
  bad "cc library bypasses dx_wrap (aspect_hints/hdrs lost)"
fi

# Manual-tag handling stays unified: test forwarders strip `manual` so both
# the private upstream and the public wrapper run under `bazel test //...`.
if grep -q -F -e '!= "manual"' cc/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' go/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' java/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' kotlin/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' scala/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' csharp/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' fsharp/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' python/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' rust/rules/defs.bzl &&
  grep -q -F -e '!= "manual"' javascript/rules/defs.bzl; then
  ok
else
  bad "test wrappers drifted on manual-tag stripping"
fi

# Upstream providers stay sealed: binary/test forwarders declare the expected
# provider set instead of accepting any target.
if ! grep -q -F -e 'upstream_providers = None' java/rules/defs.bzl &&
  ! grep -q -F -e 'upstream_providers = None' kotlin/rules/defs.bzl &&
  ! grep -q -F -e 'upstream_providers = None' scala/rules/defs.bzl &&
  ! grep -q -F -e 'upstream_providers = None' csharp/rules/defs.bzl &&
  ! grep -q -F -e 'upstream_providers = None' fsharp/rules/defs.bzl; then
  ok
else
  bad "binary/test forwarders lost their sealed upstream_providers"
fi

# Deploy boundary validates at analysis: profile uses values=, app requires
# an executable DefaultInfo target.
if grep -q -F -e 'values = VALID_DEPLOY_PROFILES' deploy/rules/defs.bzl &&
  grep -q -F -e 'providers = [DefaultInfo]' deploy/rules/defs.bzl; then
  ok
else
  bad "deploy attrs lost analysis-time validation (values/providers)"
fi

# Wrapper precision is pinned by wrapper tests alongside the wrappers.
if [[ -f python/rules/wrapper_tests.bzl && -f javascript/rules/wrapper_tests.bzl && -f rust/rules/wrapper_tests.bzl ]]; then
  ok
else
  bad "wrapper tests missing alongside language wrappers"
fi

# Corpus stays for target-less files only: no code extensions ride
# corpus targets (code rides normal targets per).
if [[ -z "$(grep -rn -E -e '\.rs"|\.py"|\.js"|\.ts"|\.go"|\.java"|\.cs"' --include='BUILD.bazel' quality/ libs/ 2>/dev/null | grep -i corpus | head -n 3 || true)" ]]; then
  ok
else
  bad "code sources riding corpus targets (must ride normal targets)"
fi

# The split stays honest in the other direction too: the code-ownership
# audit scope is code extensions only, BUILD/configs stay with
# the corpus audit (see tools/ci/code_ownership.sh vs corpus_audit.sh).
if grep -q -F -e 'rs|py|js' tools/ci/code_ownership.sh &&
  grep -q -F -e 'real_source_target' tools/ci/corpus_audit.sh; then
  ok
else
  bad "ownership-audit split drifted (code vs corpus scopes)"
fi

dx_test_summary "wrapper sources harness"
