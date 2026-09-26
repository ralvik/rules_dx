#!/usr/bin/env bash
# Action execution constraints plus cache correctness harness.
#
# Machine-checks the as-built local-only execution boundary plus the
# shared BuildBuddy remote cache delivered here, without claiming remote
# execution support:
# - delivered: every Dx pipeline plus evaluator action carries
#   `no-remote-exec` via the single `quality/execution_requirements.bzl`
#   helper (locally cacheable, never remotely executed until
#   remote is qualified); no cache-disabling `no-remote` marker appears;
#   the `cli/bep` remote/downloader interface (`RemoteConfig` plus
#   `LocalDownloader`) stays local-only;
# - shared remote cache: every Bazel workflow configures BuildBuddy
#   (`common --remote_cache` plus API key plus PR read-only uploads)
#   through a per-job rc file exported via `BAZELRC`; the legacy
#   actions/cache disk cache stays deleted (no restore-keys, no
#   hashFiles keys, no per-host prefixes);
# - live proof: aquery ExecutionInfo shows the marker on synthetic plus
#   real lint while lint vs format ActionKeys still differ (capability
#   isolation holds); the execution-log hit/miss half stays wired in
#   quality_cache_aquery; remote execution stays unverified;
# - docs in place: action-model owns the remote boundary, testing README
#   owns the local-only else branch, github-ci matrix owns the
#   BuildBuddy record, fixture pins own the markers.
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

dx_bash_pin

synthetic="quality/aspects.bzl"
real="quality/real_aspects.bzl"
helper="quality/execution_requirements.bzl"
remote="cli/bep/src/remote.rs"
ci=".github/workflows/ci.yml"
dry_run=".github/workflows/publish-dry-run.yml"
workflows_readme="docs/testing/workflow-notes.md"
action_model="docs/quality/action-model.md"
testing_readme="docs/testing/strategy-details.md"
test_matrix="docs/testing/github-ci.md"
pins="tools/ci/tests/fixtures/action_execution_cache/pins.bzl"
pins_build="tools/ci/tests/fixtures/action_execution_cache/BUILD.bazel"
expected="tools/ci/tests/fixtures/action_execution_cache/action_execution_cache.expected"

# Helper owns the local-only marker: the single enablement point for
# remote qualification (issue #1041).
if [[ -f "$helper" ]] &&
  grep -q -F -e 'def dx_execution_requirements' "$helper" &&
  grep -q -F -e 'no-remote-exec' "$helper"; then
  ok
else
  bad "execution_requirements.bzl lost its dx_execution_requirements helper with the no-remote-exec marker"
fi

# Synthetic pipeline plus evaluator actions use the helper (two call sites).
if grep -q -F -e 'execution_requirements.bzl' "$synthetic" &&
  grep -q -F -e 'dx_execution_requirements()' "$synthetic" &&
  [[ "$(grep -c -F -e 'dx_execution_requirements()' "$synthetic")" == "2" ]]; then
  ok
else
  bad "aspects.bzl lost its two dx_execution_requirements() call sites (pipeline plus evaluator)"
fi

# Real pipeline action uses the helper (one call site).
if grep -q -F -e 'execution_requirements.bzl' "$real" &&
  grep -q -F -e 'dx_execution_requirements()' "$real" &&
  [[ "$(grep -c -F -e 'dx_execution_requirements()' "$real")" == "1" ]]; then
  ok
else
  bad "real_aspects.bzl lost its dx_execution_requirements() call site (real pipeline)"
fi

# No copy-pasted literal outside the helper: enabling remote flips one place.
if ! grep -q -F -e '{"no-remote-exec": "1"}' "$synthetic" &&
  ! grep -q -F -e '{"no-remote-exec": "1"}' "$real"; then
  ok
else
  bad "quality aspects regained a copy-pasted no-remote-exec literal (want the helper only)"
fi

# BEP/remote-downloader interface stays local-only (issue #1041): the single
# RemoteConfig plus downloader seam whose impl stays local-only.
if [[ -f "$remote" ]] &&
  grep -q -F -e 'RemoteConfig' "$remote" &&
  grep -q -F -e 'LocalDownloader' "$remote" &&
  grep -q -F -e 'is_local_only' "$remote" &&
  grep -q -F -e 'UNWIRED_REMOTE_FLAGS' "$remote" &&
  grep -q -F -e 'pub mod remote' cli/bep/src/lib.rs &&
  grep -q -F -e 'src/remote.rs' cli/bep/BUILD.bazel; then
  ok
else
  bad "cli/bep lost its remote/downloader interface (want RemoteConfig plus LocalDownloader, local-only)"
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
# Keys are attributed by mnemonic, not by output order: aquery may list
# both capability actions in either order.
real_format="$(bazel aquery '//quality/testdata:fixture_real_python' \
  --aspects=//quality:real_aspects.bzl%real_format_aspect \
  --output_groups=dx_results --output=text --noshow_progress 2>/dev/null || true)"
lint_key="$(printf '%s' "$real_lint" | awk '/Mnemonic: DxRealQualityLint/{want=1} want && /ActionKey:/{print; exit}' || true)"
format_key="$(printf '%s' "$real_format" | awk '/Mnemonic: DxRealQualityFormat/{want=1} want && /ActionKey:/{print; exit}' || true)"
if [[ -n "$lint_key" && -n "$format_key" && "$lint_key" != "$format_key" ]]; then
  ok
else
  bad "real lint vs format ActionKeys must differ with the marker (capability isolation)"
fi

# Exact-key spirit survives as absence: no restore-keys prefix fallback
# anywhere in owned workflows (the legacy disk cache is deleted).
if dx_tree_absent 'restore-keys:' -- .github/workflows/; then
  ok
else
  bad "workflows regained a restore-keys prefix fallback (want the deleted disk cache gone)"
fi

# Branch-scoped poison isolation is preserved on the shared remote cache
# (issue #1059): PR runs never upload results, the setup-bazel download
# cache saves only off pull requests, and the rc file is delivered
# through GITHUB_ENV only when the BuildBuddy key exists.
if grep -q -F -e 'common --noremote_upload_local_results' "$ci" &&
  grep -q -F -e "cache-save: \${{ github.event_name != 'pull_request' }}" "$ci" &&
  grep -q -F -e 'if [ -z "${BUILDBUDDY_API_KEY}" ]' "$ci" &&
  grep -q -F -e 'echo "BAZELRC=${rc}" >> "${GITHUB_ENV}"' "$ci"; then
  ok
else
  bad "ci.yml lost the remote-cache poison isolation (want PR read-only uploads plus cache-save off plus keyless skip plus BAZELRC delivery, issue #1059)"
fi

# Delivery plus rationale stay explicit: the per-job configure step owns
# the rc content in every Bazel workflow and the policy note owns the
# dx --nohome_rc rationale (single-source intent, issue #953).
if grep -q -F -e 'Configure BuildBuddy remote cache' "$ci" &&
  grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' "$ci" &&
  grep -q -F -e 'per-job rc file' "$workflows_readme" &&
  grep -q -F -e 'nohome_rc' "$workflows_readme"; then
  ok
else
  bad "ci.yml or workflow notes lost the BuildBuddy rc-delivery record (want configure step plus policy note)"
fi

# No cache-key hashFiles list survives: the disk cache is deleted, so
# invalidation belongs to BuildBuddy and no owned workflow keys an
# actions/cache entry (hermetic line count: BSD grep lacks --include,
# issue #1006).
if [[ "$(dx_hermetic_grep tree-count --fixed --roots .github/workflows .github/actions -- "hashFiles('")" == "0" ]]; then
  ok
else
  bad "owned workflows regained a cache-key hashFiles list (want BuildBuddy owning invalidation, issue #1004)"
fi

# Every Bazel workflow carries the full BuildBuddy wiring (URL plus API
# key plus BAZELRC delivery); the secretless dry run must stay local.
for wf in ci reusable-consumer reusable-docs bump; do
  if grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' ".github/workflows/$wf.yml" &&
    grep -q -F -e 'BUILDBUDDY_API_KEY' ".github/workflows/$wf.yml" &&
    grep -q -F -e 'BAZELRC=' ".github/workflows/$wf.yml"; then
    ok
  else
    bad ".github/workflows/$wf.yml lost the BuildBuddy remote-cache wiring (want URL plus key plus BAZELRC)"
  fi
done
if ! grep -q -F -e 'BUILDBUDDY_API_KEY' "$dry_run"; then
  ok
else
  bad "publish-dry-run.yml gained the BuildBuddy key (dry run stays secretless)"
fi

# Per-host disk-cache scopes stay deleted: no prefix keys and no
# actions/cache usage remain, and the setup-bazel bazelisk cache is the
# only GitHub-hosted cache left (keyed by .bazelversion, saves off PRs).
if dx_tree_absent 'prefix: bazel-' -- .github/workflows/ &&
  dx_tree_absent 'actions/cache' -- .github/workflows/ &&
  grep -q -F -e 'bazelisk-cache: true' "$ci" &&
  grep -q -F -e "cache-save: \${{ github.event_name != 'pull_request' }}" "$ci"; then
  ok
else
  bad "workflows lost the no-disk-cache record (want no prefix scopes plus no actions/cache plus setup-bazel bazelisk cache)"
fi

# Local execution-log half stays wired next to the aquery shape half.
if grep -q -F -e 'execution_log_json_file' tools/ci/quality_cache_aquery.sh &&
  grep -q -F -e 'cache hit' tools/ci/quality_cache_aquery.sh; then
  ok
else
  bad "quality_cache_aquery lost its local execution-log hit/miss half"
fi

# Remote cache is wired; remote execution plus BES stay absent
# (local-only execution; hermetic tree search: BSD grep lacks
# --exclude-dir, issue #1006).
if grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' "$ci" &&
  dx_tree_absent '--remote_executor' -- .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/ &&
  dx_tree_absent '--bes_backend' -- .bazelrc tools/bazelrc/preset.bazelrc .github/workflows/; then
  ok
else
  bad "the BuildBuddy remote cache or the local-only boundary broke (want remote_cache wired, executor plus BES absent)"
fi

# Action model owns the remote boundary: what is safe vs local-only, with
# the helper plus downloader interface as the single enablement points.
if grep -q -F -e 'no-remote-exec' "$action_model" &&
  grep -q -F -e 'local-only' "$action_model" &&
  grep -q -F -e 'remote' "$action_model" &&
  grep -q -F -e 'local per-cell determinism' "$action_model" &&
  grep -q -F -e 'execution_requirements.bzl' "$action_model" &&
  grep -q -F -e 'dx_bep::remote' "$action_model"; then
  ok
else
  bad "action-model lost its local-only remote-boundary record with the no-remote-exec marker"
fi

# Testing README plus matrix own the local-execution plus BuildBuddy record.
if grep -q -F -e 'locally sandbox-tested but remote behavior remains unverified' "$testing_readme" &&
  grep -q -F -e 'local' "$testing_readme" &&
  grep -q -F -e 'per-cell determinism' "$testing_readme" &&
  grep -q -F -e 'common --remote_cache' "$test_matrix" &&
  grep -q -F -e 'BAZELRC' "$test_matrix"; then
  ok
else
  bad "strategy-details or github-ci lost the local-execution plus BuildBuddy remote-cache record"
fi

# Fixture files stay present with the marker plus helper/interface plus
# remote-cache pins.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]] &&
  grep -q -F -e 'no-remote-exec' "$pins" &&
  grep -q -F -e 'execution_requirements.bzl' "$pins" &&
  grep -q -F -e 'cli/bep/src/remote.rs' "$pins" &&
  grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' "$pins" &&
  grep -q -F -e 'BAZELRC' "$pins" &&
  grep -q -F -e 'noremote_upload_local_results' "$pins" &&
  grep -q -F -e 'no actions/cache, no restore-keys, no hashFiles keys' "$pins" &&
  grep -q -F -e '--remote_executor' "$pins"; then
  ok
else
  bad "action_execution_cache fixture missing (want pins.bzl plus BUILD.bazel plus expected with marker plus remote-cache pins)"
fi

# Live proof: the fixture package builds green on the seed host.
if bazel build //tools/ci/tests/fixtures/action_execution_cache/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "action_execution_cache fixture failed to build (want green on the seed host)"
fi

dx_test_summary "action execution cache qualification harness"
