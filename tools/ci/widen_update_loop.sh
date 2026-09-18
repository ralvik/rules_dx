#!/usr/bin/env bash
# Widen-one-requirement + dx update PR loop contract (issue #260, seed slice).
#
# `dx update` is within-constraints only (live resolver execution deferred
# under #19) and Renovate proposes bumps today. The owner wants a minimal
# first-party alternative: one explicit widen operation (separate from
# `dx update`, e.g. `dx bump <selector> <version>`) plus a one-dep-per-PR
# loop with an automerge on/off toggle only -- no grouping, schedule, or
# dashboard options. No such widen-then-update loop exists yet.
#
# This harness machine-checks the contract half verifiable on a clean
# tree today (12 checks): the all-ecosystems v1 manager set, the
# Renovate fallback retained, the never-rewrites invariant in code and
# docs, the #19 resolver prerequisite, the manual bump-PR verification
# loop docs, the ADR 0006 narrow exception, the ADR 0008 exact-pin
# policy, and the no-false-claim gaps (no widen command, no loop
# workflow). The widen implementation, library-first registry clients,
# loop orchestration, scheduled runner, and renovate.json disposition
# stay open under #260 (blocked on #19 backends; dry-run planning only
# until live `dx update` lands).
#
# Versioned here, run by CI via `bazel run //tools/ci:widen_update_loop`,
# following //tools/ci:ghcr_hygiene.
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

contract="docs/cli/commands/audit-update-bazel.md"
semantics="cli/update/src/semantics.rs"
renovate="renovate.json"
automation="docs/contributing/automation.md"
workflows_doc="docs/contributing/local-workflows.md"
adr_surface="docs/decisions/0006-cli-command-surface.md"
adr_currency="docs/decisions/0008-dependency-currency.md"

# All ecosystems in v1, no phasing: Bazel modules + .bazelversion, Cargo,
# npm/pnpm, Go, GitHub Actions -- matching the current renovate manager set.
if [[ -f "$renovate" ]] \
  && grep -q -F -e '"bazel"' "$renovate" \
  && grep -q -F -e '"cargo"' "$renovate" \
  && grep -q -F -e '"github-actions"' "$renovate" \
  && grep -q -F -e '"gomod"' "$renovate" \
  && grep -q -F -e '"npm"' "$renovate"; then
  ok
else
  bad "renovate.json lost the all-ecosystems v1 manager set (bazel/cargo/github-actions/gomod/npm)"
fi

# Renovate fallback retained until the first-party loop lands: weekly
# Monday schedule, no automerge, reviewable PRs, dashboard on.
if grep -q -F -e 'before 5am on Monday' "$renovate" \
  && grep -q -F -e '"automerge": false' "$renovate" \
  && grep -q -F -e '"platformAutomerge": false' "$renovate" \
  && grep -q -F -e '"prCreation": "not-pending"' "$renovate" \
  && grep -q -F -e '"dependencyDashboard": true' "$renovate"; then
  ok
else
  bad "renovate.json lost the retained-fallback shape (schedule/automerge/prCreation/dashboard)"
fi

# Never-rewrites invariant pinned in code: both requirement shapes refuse
# rewriting, with the never-widen unit test present.
if grep -q -F -e 'pub fn may_be_rewritten' "$semantics" \
  && grep -q -F -e 'declared_requirements_are_never_rewritten' "$semantics" \
  && grep -q -F -e 'must constrain, never widen' "$semantics"; then
  ok
else
  bad "semantics.rs lost the never-rewrites pin (may_be_rewritten + never-widen test)"
fi

# Never-rewrites contract in docs: update refreshes locks without
# widening or replacing declared requirements; exact pins stay constraints.
if grep -q -F -e 'without widening or replacing' "$contract" \
  && grep -q -F -e 'Exact requirement pins remain constraints' "$contract"; then
  ok
else
  bad "update contract lost the without-widening clause or the exact-pin constraint"
fi

# No widen command claimed: no `bump` subcommand string and no widen-one
# function exist in CLI code yet; the explicit operation stays open under #260.
if ! grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q . \
  && ! grep -rn -E -e 'fn widen' --include='*.rs' cli/ 2>/dev/null | grep -q .; then
  ok
else
  bad "widen-shaped symbols appeared in cli/ without the #260 implementation landing"
fi

# Blocked on resolver backends: selector syntax, Git mappings, and
# upstream operation/report mappings stay open, never decided here.
if [[ "$(grep -c -F -e 'open work' "$contract")" -ge 4 ]]; then
  ok
else
  bad "update contract lost its open resolver-prerequisite records (need >=4 open-work references)"
fi

# Automation policy owns the Renovate fallback: absent-only scaffold,
# full manager set, update-only automerge.
if grep -q -F -e 'absent-only' "$automation" \
  && grep -q -F -e 'full manager' "$automation" \
  && grep -q -F -e 'update-only' "$automation" \
  && grep -q -F -e 'Renovate is the chosen updater' "$automation"; then
  ok
else
  bad "automation.md lost the Renovate-fallback ownership (absent-only/manager/update-only/chosen-updater)"
fi

# Manual bump-PR verification loop documented: regen evidence,
# flag-diff review, full verification per the automation policy.
if grep -q -F -e 'regen' "$workflows_doc" \
  && grep -q -F -e 'flag-diff' "$workflows_doc" \
  && grep -q -F -e 'full verification' "$workflows_doc" \
  && grep -q -F -e 'automation.md' "$workflows_doc"; then
  ok
else
  bad "local-workflows.md lost the bump-PR verification loop (regen/flag-diff/verification/policy)"
fi

# ADR 0006 owns the narrow exception: `dx update` continues independent
# sets without a repository-wide rollback or a private scheduler.
if grep -q -F -e 'narrow exception' "$adr_surface" \
  && grep -q -F -e 'audit-update-bazel.md' "$adr_surface"; then
  ok
else
  bad "ADR 0006 lost the update narrow-exception pin or its contract link"
fi

# ADR 0008 owns the library-first pinning policy: latest stable,
# pinned exactly, checksummed where applicable.
if grep -q -F -e 'pinned exactly' "$adr_currency" \
  && grep -q -F -e 'latest stable' "$adr_currency"; then
  ok
else
  bad "ADR 0008 lost the exact-pin policy (pinned exactly + latest stable)"
fi

# No loop workflow claimed: no scheduled widen-then-update runner exists
# in workflows yet; one-dep-per-PR, toggle-only automerge, concurrency,
# and fork-safety stay open under #260.
if ! grep -rn -F -e 'dx bump' .github/workflows/ 2>/dev/null | grep -q . \
  && ! grep -rln -F -e 'widen-one' .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "a widen-loop runner appeared in workflows without the #260 implementation landing"
fi

# Upstream scope pins intact: prerelease follows upstream, transitives
# stay resolver-governed, outside-requirements is not a failure.
if grep -q -F -e 'prerelease_follows_upstream' "$semantics" \
  && grep -q -F -e 'forces_transitive_newest' "$semantics" \
  && grep -q -F -e 'outside_requirements_is_failure' "$semantics"; then
  ok
else
  bad "semantics.rs lost the upstream-scope pins (prerelease/transitives/scope)"
fi

echo "widen update loop harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
