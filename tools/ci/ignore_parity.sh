#!/usr/bin/env bash
# Ignore-list parity harness.
#
# `.gitignore` and `.bazelignore` contradict each other when disposable trees
# are listed in one but not the other: spills stay unignored by Bazel or leak
# into `git status`. `.opencode/` scope plus `node_modules/` plus `.ruff_cache/`
# plus `bazel-*` must stay mirrored with the local-only contracts documented
# in-file, so this fails on drift. File singletons (`*.profraw`, `*.profdata`,
# `opencode.json`) have no `.bazelignore` form and stay Bazel-visible by
# design; `.gitignore` plus scratch containment plus no BUILD glob owns them
# (issue #1002), so this also fails when that defense-in-depth drifts.
#
# Usage: ignore_parity.sh <gitignore> <bazelignore> <bazelrc> <local_workflows>
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"
dx_bootstrap "tools/sh/guards.sh"

dx_test_init

gitignore="${1:?usage: ignore_parity.sh <gitignore> <bazelignore> <bazelrc> <local_workflows>}"
bazelignore="${2:?usage: ignore_parity.sh <gitignore> <bazelignore> <bazelrc> <local_workflows>}"
bazelrc="${3:?usage: ignore_parity.sh <gitignore> <bazelignore> <bazelrc> <local_workflows>}"
local_workflows="${4:?usage: ignore_parity.sh <gitignore> <bazelignore> <bazelrc> <local_workflows>}"

# .gitignore carries the disposable trees plus the local-only contracts.
dx_guards_contains "$gitignore" ".gitignore lost disposable-tree entries (want node_modules plus ruff cache plus bazel outputs, issue #911)" \
  'node_modules/' \
  '.ruff_cache/' \
  '/bazel-*'
dx_guards_contains "$gitignore" ".gitignore lost the .opencode whole-dir scope (want .opencode with mirror note, issue #911)" \
  '.opencode/' \
  'mirrors'
dx_guard_absent "$gitignore" '.opencode/ralph-loop.local.md' ".gitignore regressed to the narrow .opencode entry (want whole-dir .opencode/, issue #911)"
dx_guards_contains "$gitignore" ".gitignore lost the opencode.json local-only contract (want entry plus local-only note, issue #911)" \
  'opencode.json' \
  'Local-only opencode config'
dx_guards_contains "$gitignore" ".gitignore lost the user.bazelrc local-only contract (want entry plus resolution plus canonical registry, issue #911)" \
  'user.bazelrc' \
  'local-only overlay' \
  'bcr.bazel.build'
dx_guards_contains "$gitignore" ".gitignore lost the coverage-spill defense-in-depth (want *.profraw plus *.profdata, issue #1002)" \
  '*.profraw' \
  '*.profdata'

# .bazelignore mirrors the same disposable directories (file singletons have
# no form here; only directories are listed, issue #1002).
dx_guards_contains "$bazelignore" ".bazelignore lost disposable-tree entries (want node_modules plus ruff cache plus bazel outputs, issue #911)" \
  'node_modules' \
  '.ruff_cache' \
  'bazel-*'
dx_guards_contains "$bazelignore" ".bazelignore lost the file-singleton record (want profraw plus profdata plus opencode.json plus no-form note, issue #1002)" \
  '*.profraw' \
  '*.profdata' \
  'opencode.json' \
  'no .bazelignore form'
dx_guards_contains "$bazelignore" ".bazelignore lost the .opencode whole-dir scope (want .opencode with parity note, issue #911)" \
  '.opencode/' \
  'Parity notes'
dx_guards_contains "$bazelignore" ".bazelignore lost shared disposable dirs (want cache plus direnv plus tmp plus pycache plus dist plus release, issue #911)" \
  '.cache' \
  '.direnv' \
  '.tmp' \
  '__pycache__' \
  'dist' \
  'release'

# .bazelrc try-import stays resolution-only with canonical URLs documented.
dx_guards_contains "$bazelrc" ".bazelrc lost the user.bazelrc resolution-only contract (want try-import plus resolution-only plus canonical registry, issue #911)" \
  'try-import %workspace%/user.bazelrc' \
  'resolution-only' \
  'bcr.bazel.build'

# Local-workflows doc stays the canonical-URL owner for the overlay.
dx_guards_contains "$local_workflows" "local-workflows.md lost the user.bazelrc canonical-registry record (want gitignored overlay plus bcr.bazel.build, issue #911)" \
  'user.bazelrc' \
  'bcr.bazel.build'

# Tracked-tree guard (workspace only): none of the ignored trees or file
# singletons may be committed. Skipped under `bazel test` sandbox here (no
# git checkout there); enforced on a clean tree via direct execution plus
# `bazel run`.
if ws="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  if [[ -z "$(git -C "$ws" ls-files | grep -E '(^|/)(node_modules|\.ruff_cache|\.opencode|\.cache)/' || true)" ]] &&
    [[ -z "$(git -C "$ws" ls-files | grep -E '^(dist|release)/' || true)" ]] &&
    [[ -z "$(git -C "$ws" ls-files | grep -E '(^|/)(opencode\.json|user\.bazelrc)$' || true)" ]] &&
    [[ -z "$(git -C "$ws" ls-files | grep -E '\.(profraw|profdata)$' || true)" ]]; then
    ok
  else
    bad "ignored trees are tracked (want no node_modules/.ruff_cache/.opencode/.cache plus root dist/release plus opencode.json/user.bazelrc plus no profraw/profdata in git ls-files, issue #1002)"
  fi
  if git -C "$ws" check-ignore -q "node_modules/foo" &&
    git -C "$ws" check-ignore -q ".ruff_cache/foo" &&
    git -C "$ws" check-ignore -q ".opencode/foo" &&
    git -C "$ws" check-ignore -q "opencode.json" &&
    git -C "$ws" check-ignore -q "user.bazelrc" &&
    git -C "$ws" check-ignore -q "default_123.profraw" &&
    git -C "$ws" check-ignore -q "default_123.profdata"; then
    ok
  else
    bad "git check-ignore misses an ignored path (want node_modules plus .ruff_cache plus .opencode plus opencode.json plus user.bazelrc plus profraw/profdata ignored, issue #1002)"
  fi
else
  ok
fi

dx_test_summary "ignore parity harness"
