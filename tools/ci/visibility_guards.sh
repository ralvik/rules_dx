#!/usr/bin/env bash
# Visibility hardening guard.
#
# docs/roadmap.md lists visibility hardening with no open owner, only
# scattered default_visibility. Public is external API only; everything
# else is repo-internal or narrower. See
# docs/contributing/build-conventions.md#visibility.
#
# This harness machine-checks the hardened state without rebuilding the
# tree: public defaults match the allowlist, explicit public targets are
# entry points only, lang env plans stay private, and the canonical doc
# owns the policy.
#
# Versioned here, run by CI via `bazel run //tools/ci:visibility_guards`.
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# Canonical doc owns the visibility policy.
if grep -q -F -e '## Visibility' docs/contributing/build-conventions.md &&
  grep -q -F -e '//tools/ci:visibility_guards' docs/contributing/build-conventions.md &&
  grep -q -F -e 'issue #456' docs/contributing/build-conventions.md; then
  ok
else
  bad "canonical visibility policy missing (want ## Visibility with //tools/ci:visibility_guards plus issue #456 in docs/contributing/build-conventions.md)"
fi

# Public defaults match the external-API allowlist only (21 packages):
# config, dx, env, generation, quality roots, 15 language rules, deploy/rules.
allow_public="astro/rules/BUILD.bazel cc/rules/BUILD.bazel config/BUILD.bazel csharp/rules/BUILD.bazel deploy/rules/BUILD.bazel dx/BUILD.bazel env/BUILD.bazel fsharp/rules/BUILD.bazel generation/BUILD.bazel go/rules/BUILD.bazel java/rules/BUILD.bazel javascript/rules/BUILD.bazel kotlin/rules/BUILD.bazel mdx/rules/BUILD.bazel python/rules/BUILD.bazel quality/BUILD.bazel rust/rules/BUILD.bazel scala/rules/BUILD.bazel svelte/rules/BUILD.bazel typescript/rules/BUILD.bazel vue/rules/BUILD.bazel"
public_defaults="$(grep -rl -F -e 'package(default_visibility = ["//visibility:public"])' --include='BUILD.bazel' . 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
expected="$(echo "$allow_public" | tr ' ' '\n' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$public_defaults" == "$expected" ]]; then
  ok
else
  bad "public defaults drifted (want exactly 21 external-API packages, got: $public_defaults)"
fi

# Explicit public target visibilities are entry points only (//cli/cli:dx,
# //cli/env:env plus their Cargo.toml exports and man pages). Match
# attribute-level `visibility =` (leading spaces) so package
# `default_visibility` does not count.
explicit_public_files="$(grep -rl -e '^[[:space:]]*visibility = \["//visibility:public"\]' --include='BUILD.bazel' . 2>/dev/null | sed -e 's|^\./||' | sort | tr '\n' ' ' | sed -e 's/ $//')"
if [[ "$explicit_public_files" == "cli/cli/BUILD.bazel cli/env/BUILD.bazel" ]]; then
  ok
else
  bad "explicit public targets leaked (want only cli/cli/BUILD.bazel plus cli/env/BUILD.bazel, got: $explicit_public_files)"
fi

# Language env plans stay private (no cross-package consumers).
env_fail=""
for pkg in astro cc csharp fsharp go java javascript kotlin mdx python rust scala svelte typescript vue; do
  f="$pkg/env/BUILD.bazel"
  if ! grep -q -F -e 'package(default_visibility = ["//visibility:private"])' "$f"; then
    env_fail="$env_fail $pkg:not-private"
  fi
done
if [[ -z "$env_fail" ]]; then
  ok
else
  bad "language env visibility drifted:$env_fail"
fi

# Shared cli leaves stay scoped (no public defaults).
scoped_fail=""
for pkg in cli/atomic_fs cli/digest cli/lcov cli/path cli/proto_validate cli/schema; do
  if grep -q -F -e '//visibility:public' "$pkg/BUILD.bazel"; then
    scoped_fail="$scoped_fail $pkg:public"
  elif ! grep -q -F -e '//cli:__subpackages__' "$pkg/BUILD.bazel"; then
    scoped_fail="$scoped_fail $pkg:no-cli-scope"
  fi
done
for pkg in quality/adapter quality/runner quality/result quality/markdown generation/result generation/codegen_shard env/env_shard; do
  if grep -q -F -e '//visibility:public' "$pkg/BUILD.bazel"; then
    scoped_fail="$scoped_fail $pkg:public"
  fi
done
if [[ -z "$scoped_fail" ]]; then
  ok
else
  bad "scoped leaf visibility drifted:$scoped_fail"
fi

# Internal-only trees stay repo-internal (no public defaults).
internal_fail=""
for f in tools/sh/BUILD.bazel tools/ci/BUILD.bazel tools/bazelrc/BUILD.bazel tools/coverage/BUILD.bazel tools/depcheck/BUILD.bazel docs/BUILD.bazel docs/ir/BUILD.bazel docs/ir/ir/BUILD.bazel examples/BUILD.bazel BUILD.bazel cli/BUILD.bazel; do
  if grep -q -F -e '//visibility:public' "$f"; then
    internal_fail="$internal_fail $f:public"
  elif ! grep -q -F -e '//:__subpackages__' "$f"; then
    internal_fail="$internal_fail $f:no-internal-scope"
  fi
done
if [[ -z "$internal_fail" ]]; then
  ok
else
  bad "internal tree visibility drifted:$internal_fail"
fi

dx_test_summary "visibility hardening harness"
