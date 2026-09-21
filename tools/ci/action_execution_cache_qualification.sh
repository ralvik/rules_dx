#!/usr/bin/env bash
# Action execution constraints plus cache-key correctness harness.
#
# Machine-checks the as-built local-only execution boundary plus exact-key
# disk cache delivered here, without claiming remote execution or remote
# cache support:
# - delivered: every Dx pipeline plus evaluator action carries
#   `no-remote-exec` (locally cacheable, never remotely executed until
#   remote is qualified); no cache-disabling `no-remote` marker appears;
# - disk cache: keys hash every Bazel-affecting lock/config with exact-key
#   hits only (a bust starts cold, no stale prefix reuse) across per-host
#   scopes; reusable-consumer stays cache-free;
# - live proof: aquery ExecutionInfo shows the marker on synthetic plus
#   real lint while lint vs format ActionKeys still differ (capability
#   isolation holds); the execution-log hit/miss half stays wired in
#   quality_cache_aquery; remote execution plus remote cache stay
#   unverified and unwired;
# - docs in place: action-model owns the remote boundary, testing README
#   owns the local-only else branch, github-ci matrix owns the exact-key
#   record, fixture pins own the markers.
# - CI only: no product runtime change beyond the action marker.
#
# Versioned here, run by CI via `bazel run //tools/ci:action_execution_cache_qualification`,
# following //tools/ci:quality_cache_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

synthetic="quality/aspects.bzl"
real="quality/real_aspects.bzl"
ci=".github/workflows/ci.yml"
bump=".github/workflows/bump.yml"
cache_action=".github/actions/restore-bazel-cache/action.yml"
workflows_readme=".github/workflows/README.md"
action_model="docs/quality/action-model.md"
testing_readme="docs/testing/README.md"
test_matrix="docs/testing/github-ci.md"
pins="tools/ci/tests/fixtures/action_execution_cache/pins.bzl"
pins_build="tools/ci/tests/fixtures/action_execution_cache/BUILD.bazel"
expected="tools/ci/tests/fixtures/action_execution_cache/action_execution_cache.expected"

# Synthetic pipeline plus evaluator actions carry the local-only marker.
if grep -q -F -e 'execution_requirements = {"no-remote-exec": "1"}' "$synthetic" &&
  [[ "$(grep -c -F -e 'execution_requirements = {"no-remote-exec": "1"}' "$synthetic")" == "2" ]]; then
  ok
else
  bad "aspects.bzl lost its two no-remote-exec markers (pipeline plus evaluator)"
fi

# Real pipeline action carries the local-only marker.
if grep -q -F -e 'execution_requirements = {"no-remote-exec": "1"}' "$real" &&
  [[ "$(grep -c -F -e 'execution_requirements = {"no-remote-exec": "1"}' "$real")" == "1" ]]; then
  ok
else
  bad "real_aspects.bzl lost its no-remote-exec marker (real pipeline)"
fi

# No cache-disabling marker: actions stay locally cacheable.
if ! grep -q -F -e '"no-remote": "1"' "$synthetic" &&
  ! grep -q -F -e '"no-remote": "1"' "$real"; then
  ok
else
  bad "quality actions gained a cache-disabling no-remote marker (want no-remote-exec only)"
fi

# Live synthetic proof: aquery shows the marker on the lint action.
if bazel aquery '//quality/testdata:fixture_python' \
  --aspects=//quality:aspects.bzl%lint_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null | grep -q -F -e 'ExecutionInfo: {no-remote-exec: 1}'; then
  ok
else
  bad "synthetic lint aquery lost its ExecutionInfo no-remote-exec marker"
fi

# Live real proof: aquery shows the marker on the real lint action.
real_lint="$(bazel aquery '//quality/testdata:fixture_real_python' \
  --aspects=//quality:real_aspects.bzl%real_lint_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
if printf '%s' "$real_lint" | grep -q -F -e 'ExecutionInfo: {no-remote-exec: 1}'; then
  ok
else
  bad "real lint aquery lost its ExecutionInfo no-remote-exec marker"
fi

# Capability isolation holds with the marker: lint vs format keys differ.
real_format="$(bazel aquery '//quality/testdata:fixture_real_python' \
  --aspects=//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
lint_key="$(printf '%s' "$real_lint" | grep 'ActionKey:' | head -1 || true)"
format_key="$(printf '%s' "$real_format" | grep 'ActionKey:' | head -1 || true)"
if [[ -n "$lint_key" && -n "$format_key" && "$lint_key" != "$format_key" ]]; then
  ok
else
  bad "real lint vs format ActionKeys must differ with the marker (capability isolation)"
fi

# Exact-key disk cache: no prefix fallback in owned workflows or the shared
# restore action.
if ! grep -q -F -e 'restore-keys:' "$ci" &&
  ! grep -q -F -e 'restore-keys:' "$bump" &&
  ! grep -q -F -e 'restore-keys:' "$cache_action"; then
  ok
else
  bad "workflows regained a prefix fallback (want exact key only, bust starts cold)"
fi

# Exact-key record stays explicit in the single-source restore action plus
# the policy note (issue #953): the hashFiles list lives once in the
# composite while ci.yml passes only per-host prefixes.
if grep -q -F -e 'exact key only, bust starts cold' "$cache_action" &&
  [[ "$(grep -c -F -e './.github/actions/restore-bazel-cache' "$ci")" -ge "20" ]] &&
  grep -q -F -e 'exact key only with no prefix fallback' "$workflows_readme"; then
  ok
else
  bad "restore-bazel-cache plus ci.yml plus workflows README lost the exact-key-only single-source record"
fi

# Cache keys stay comprehensive across locks plus configs plus toolchains
# (single-sourced in the restore action, issue #953).
if grep -q -F -e "MODULE.bazel.lock" "$cache_action" &&
  grep -q -F -e ".bazelrc" "$cache_action" &&
  grep -q -F -e "tools/bazelrc/preset.bazelrc" "$cache_action" &&
  grep -q -F -e ".bazelversion" "$cache_action" &&
  grep -q -F -e "cargo-bazel-lock.json" "$cache_action" &&
  grep -q -F -e "Cargo.lock" "$cache_action" &&
  grep -q -F -e "pnpm-lock.yaml" "$cache_action" &&
  grep -q -F -e "maven_install.json" "$cache_action" &&
  grep -q -F -e "go.mod" "$cache_action" &&
  grep -q -F -e "uv.lock" "$cache_action" &&
  grep -q -F -e "pyproject.toml" "$cache_action"; then
  ok
else
  bad "restore-bazel-cache lost a comprehensive lock/config key (want MODULE plus bazelrc plus toolchain plus resolver locks)"
fi

# Per-host cache scopes stay pinned per platform family: ci.yml passes the
# prefix per job while the shared action owns the exact-key shape.
if grep -q -F -e 'bazel-seed-' "$ci" &&
  grep -q -F -e 'bazel-arm64-' "$ci" &&
  grep -q -F -e 'bazel-musl-x86_64-' "$ci" &&
  grep -q -F -e 'bazel-musl-arm64-' "$ci" &&
  grep -q -F -e 'bazel-macos-arm64-' "$ci" &&
  grep -q -F -e 'bazel-macos-x86_64-' "$ci" &&
  grep -q -F -e 'bazel-windows-x86_64-' "$ci" &&
  grep -q -F -e 'actions/cache' "$cache_action"; then
  ok
else
  bad "ci.yml lost a per-host Bazel disk-cache scope (seed plus arm64 plus musl pair plus macos pair plus windows)"
fi

# Local execution-log half stays wired next to the aquery shape half.
if grep -q -F -e 'execution_log_json_file' tools/ci/quality_cache_aquery.sh &&
  grep -q -F -e 'cache hit' tools/ci/quality_cache_aquery.sh; then
  ok
else
  bad "quality_cache_aquery lost its local execution-log hit/miss half"
fi

# No remote flags in owned config or workflows (local-only execution).
if ! grep -rn -F -e '--remote_cache' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e '--remote_executor' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q . &&
  ! grep -rn -F -e '--bes_backend' .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "a remote cache/executor flag appeared (local-only execution)"
fi

# Action model owns the remote boundary: what is safe vs local-only.
if grep -q -F -e 'no-remote-exec' "$action_model" &&
  grep -q -F -e 'local-only' "$action_model" &&
  grep -q -F -e 'remote' "$action_model"; then
  ok
else
  bad "action-model lost its local-only remote-boundary record with the no-remote-exec marker"
fi

# Testing README plus matrix own the exact-key plus local-only record.
if grep -q -F -e 'locally sandbox-tested but remote behavior remains unverified' "$testing_readme" &&
  grep -q -F -e 'exact' "$test_matrix" &&
  grep -q -F -e 'no `--remote_cache`' "$test_matrix"; then
  ok
else
  bad "testing README or github-ci matrix lost the exact-key plus local-only remote record"
fi

# Fixture files stay present with the marker plus cache pins.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]] &&
  grep -q -F -e 'no-remote-exec' "$pins" &&
  grep -q -F -e 'exact key only, bust starts cold' "$pins" &&
  grep -q -F -e 'bazel-seed-' "$pins"; then
  ok
else
  bad "action_execution_cache fixture missing (want pins.bzl plus BUILD.bazel plus expected with marker plus cache pins)"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/action_execution_cache/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "action_execution_cache fixture failed to build (want green on the seed host)"
fi

dx_test_summary "action execution cache qualification harness"
