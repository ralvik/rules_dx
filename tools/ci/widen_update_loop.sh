#!/usr/bin/env bash
# Widen-one-requirement + dx update PR loop contract (issue #260, delivered).
#
# `dx update` is within-constraints only (live resolver execution delivered
# in #19) and Renovate is retained as fallback. The minimal first-party
# alternative is delivered: one explicit widen operation (separate from
# `dx update`, `dx bump <selector> <version>`) plus a one-dep-per-PR loop
# with an automerge on/off toggle only -- no grouping, schedule, or
# dashboard options.
#
# This harness machine-checks the delivered contract on a clean tree
# (12 checks): the all-ecosystems v1 manager set, the Renovate fallback
# retained, the never-rewrites invariant in code and docs (update keeps it,
# bump owns the single-requirement rewrite), the #19 resolver prerequisite
# delivered, the widen command plus library-first planning, the native
# bump-PR verification loop docs, the ADR 0006 narrow exception, the ADR
# 0008 exact-pin policy, the scheduled loop runner (one-dep-per-PR,
# toggle-only automerge, concurrency, fork-safety), and the upstream scope
# pins.
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
bump_request="cli/bump/src/request.rs"
bump_workflow=".github/workflows/bump.yml"

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

# Renovate fallback retained with the native loop delivered: weekly Monday
# schedule, no automerge, reviewable PRs, dashboard on.
if grep -q -F -e 'before 5am on Monday' "$renovate" \
  && grep -q -F -e '"automerge": false' "$renovate" \
  && grep -q -F -e '"platformAutomerge": false' "$renovate" \
  && grep -q -F -e '"prCreation": "not-pending"' "$renovate" \
  && grep -q -F -e '"dependencyDashboard": true' "$renovate"; then
  ok
else
  bad "renovate.json lost the retained-fallback shape (schedule/automerge/prCreation/dashboard)"
fi

# Never-rewrites invariant pinned in code: both update requirement shapes
# refuse rewriting, with the never-widen unit test present.
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

# Widen command delivered: `bump` subcommand in CLI code, the explicit
# single-requirement rewrite owner, and the bump_failed operational code.
if grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q . \
  && grep -q -F -e 'may_be_rewritten' "$bump_request" \
  && grep -q -F -e 'CODE_BUMP_FAILED' cli/cli/src/exec/common.rs \
  && grep -q -F -e 'dx_bump' cli/cli/src/exec/bump.rs; then
  ok
else
  bad "widen bump command missing in cli/ (want bump subcommand + may_be_rewritten + CODE_BUMP_FAILED + dx_bump)"
fi

# Resolver backends delivered in #19 (selector syntax, Git mappings, and
# upstream operation/report mappings decided in dx_update); audit live
# execution delivered in #18.
if grep -q -F -e 'dx_update::selector' "$contract" \
  && grep -q -F -e 'dx_update::backend' "$contract" \
  && grep -q -F -e 'dx_update::semantics' "$contract"; then
  ok
else
  bad "update contract lost its delivered resolver records"
fi

# Automation policy owns the native loop plus the Renovate fallback:
# absent-only scaffold, full manager set, update-only automerge, plus the
# delivered widen-one loop (one dep per PR, toggle-only automerge,
# scheduled runner).
if grep -q -F -e 'absent-only' "$automation" \
  && grep -q -F -e 'full manager' "$automation" \
  && grep -q -F -e 'update-only' "$automation" \
  && grep -q -F -e 'Renovate is the chosen updater' "$automation" \
  && grep -q -F -e 'dx bump' "$automation" \
  && grep -q -F -e 'one dep per PR' "$automation" \
  && grep -q -F -e 'bump.yml' "$automation"; then
  ok
else
  bad "automation.md lost the native-loop ownership (fallback pins + dx bump + one-dep-per-PR + bump.yml)"
fi

# Native bump-PR verification loop documented: regen evidence, flag-diff
# review, full verification per the automation policy, plus the explicit
# widen-then-update steps with a clean-tree reset.
if grep -q -F -e 'regen' "$workflows_doc" \
  && grep -q -F -e 'flag-diff' "$workflows_doc" \
  && grep -q -F -e 'full verification' "$workflows_doc" \
  && grep -q -F -e 'automation.md' "$workflows_doc" \
  && grep -q -F -e 'dx bump' "$workflows_doc" \
  && grep -q -F -e 'dx update' "$workflows_doc" \
  && grep -q -F -e 'clean tree' "$workflows_doc"; then
  ok
else
  bad "local-workflows.md lost the native bump-PR loop (regen/flag-diff/verification/policy + dx bump/update + clean tree)"
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

# Scheduled widen-then-update runner delivered: `dx bump` plus `widen-one`
# orchestration, one-dep-per-PR, toggle-only automerge, concurrency
# control, GITHUB_TOKEN PR creation, and fork-safety.
if [[ -f "$bump_workflow" ]] \
  && grep -q -F -e 'dx bump' "$bump_workflow" \
  && grep -q -F -e 'widen-one' "$bump_workflow" \
  && grep -q -F -e 'concurrency' "$bump_workflow" \
  && grep -q -F -e 'github.token' "$bump_workflow" \
  && grep -q -F -e 'Fork' "$bump_workflow" \
  && grep -q -F -e 'one dep per PR' "$bump_workflow"; then
  ok
else
  bad "bump.yml runner missing the widen-loop contract (dx bump + widen-one + concurrency + github.token + Fork + one-dep-per-PR)"
fi

# Upstream scope pins intact: prerelease follows upstream, transitives
# stay resolver-governed, outside-requirements is not a failure.
if grep -q -F -e 'prerelease_follows_upstream' "$semantics" \
  && grep -q -F -e 'forces_transitive_newest' "$semantics" \
  && grep -q -F -e 'outside_requirements_is_failure' "$semantics" \
  && grep -q -F -e 'prerelease_follows_upstream' cli/bump/src/version.rs; then
  ok
else
  bad "upstream-scope pins lost (prerelease/transitives/scope in update + bump)"
fi

echo "widen update loop harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
