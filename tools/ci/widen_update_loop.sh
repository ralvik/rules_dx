#!/usr/bin/env bash
# Widen-one-requirement + dx update PR loop contract (delivered).
#
# `dx update` is within-constraints only (live resolver execution delivered
# in). The first-party
# apply path is delivered as the sole updater (native-only,):
# one explicit widen operation (separate from
# `dx update`, `dx bump <selector> <version>`) plus a one-dep-per-PR loop
# with an automerge on/off toggle only -- no grouping, schedule, or
# dashboard options.
#
# This harness machine-checks the delivered contract on a clean tree
# (13 checks): the all-ecosystems v1 set registry, sole-updater shape
# (no third-party updater config, bump owns the single-requirement
# rewrite), the resolver prerequisite
# delivered, the widen command plus library-first planning, the native
# bump-PR verification loop docs, the ADR 0006 narrow exception, the ADR
# 0008 exact-pin policy, the scheduled loop runner (one-dep-per-PR,
# toggle-only automerge, concurrency, fork-safety), and the upstream scope
# pins.
#
# Versioned here, run by CI via `bazel run //tools/ci:widen_update_loop`,
# following //tools/ci:ghcr_hygiene.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

contract="docs/cli/commands/audit-update-bazel.md"
semantics="cli/update/src/semantics.rs"
sets="cli/bump/src/sets.rs"
automation="docs/contributing/automation.md"
workflows_doc="docs/contributing/local-workflows.md"
adr_surface="docs/decisions/0006-cli-command-surface.md"
adr_currency="docs/decisions/0008-dependency-currency.md"
bump_request="cli/bump/src/request.rs"
bump_workflow=".github/workflows/bump.yml"

# All ecosystems in v1, no phasing: Bazel modules + .bazelversion, Cargo,
# npm/pnpm, Go, GitHub Actions, Maven, NuGet -- pinned in the native set
# registry.
if grep -q -F -e 'BumpSet::Bazel' "$sets" &&
  grep -q -F -e 'BumpSet::Cargo' "$sets" &&
  grep -q -F -e 'BumpSet::GithubActions' "$sets" &&
  grep -q -F -e 'BumpSet::Go' "$sets" &&
  grep -q -F -e 'BumpSet::Maven' "$sets" &&
  grep -q -F -e 'BumpSet::Npm' "$sets" &&
  grep -q -F -e 'BumpSet::NuGet' "$sets" &&
  grep -q -F -e 'BumpSet::ALL' "$sets"; then
  ok
else
  bad "native set registry lost the all-ecosystems v1 set (bazel/cargo/github-actions/go/maven/npm/nuget)"
fi

# Sole-updater shape: the native loop owns discovery plus widen plus
# verify with one dep per PR; the scaffold plans eight files with no
# updater config and the policy names the sole updater.
if grep -q -F -e 'files.len(), 8' cli/adopt/src/scaffold.rs &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'one dep per PR' "$automation" &&
  grep -q -F -e 'native-only' "$automation" &&
  grep -q -F -e 'issue #461' "$automation"; then
  ok
else
  bad "native loop lost its sole-updater shape (eight-file scaffold plus sole updater plus one-dep-per-PR plus native-only, issue #461)"
fi

# Never-rewrites invariant pinned in code: both update requirement shapes
# refuse rewriting, with the never-widen unit test present.
if grep -q -F -e 'pub fn may_be_rewritten' "$semantics" &&
  grep -q -F -e 'declared_requirements_are_never_rewritten' "$semantics" &&
  grep -q -F -e 'must constrain, never widen' "$semantics"; then
  ok
else
  bad "semantics.rs lost the never-rewrites pin (may_be_rewritten + never-widen test)"
fi

# Never-rewrites contract in docs: update refreshes locks without
# widening or replacing declared requirements; exact pins stay constraints.
if grep -q -F -e 'without widening or replacing' "$contract" &&
  grep -q -F -e 'Exact requirement pins remain constraints' "$contract"; then
  ok
else
  bad "update contract lost the without-widening clause or the exact-pin constraint"
fi

# Widen command delivered: `bump` subcommand in CLI code, the explicit
# single-requirement rewrite owner, and the bump_failed operational code.
# NOTE: no `| grep -q .` pipe here: under `set -o pipefail` the early-exit
# `grep -q` closes the pipe and the producer dies with SIGPIPE (141),
# failing the check even when matches exist (issue #641 fix).
if grep -rn -F -e '"bump"' --include='*.rs' cli/ >/dev/null 2>&1 &&
  grep -q -F -e 'may_be_rewritten' "$bump_request" &&
  grep -q -F -e 'CODE_BUMP_FAILED' cli/cli/src/exec/common.rs &&
  grep -q -F -e 'dx_bump' cli/cli/src/exec/bump.rs; then
  ok
else
  bad "widen bump command missing in cli/ (want bump subcommand + may_be_rewritten + CODE_BUMP_FAILED + dx_bump)"
fi

# Resolver backends delivered in (selector syntax, Git mappings, and
# upstream operation/report mappings decided in dx_update); audit live
# execution delivered in.
if grep -q -F -e 'dx_update::selector' "$contract" &&
  grep -q -F -e 'dx_update::backend' "$contract" &&
  grep -q -F -e 'dx_update::semantics' "$contract"; then
  ok
else
  bad "update contract lost its delivered resolver records"
fi

# Automation policy owns the native-only loop: absent-only
# scaffold, update-only automerge, plus the delivered widen-one loop (one
# dep per PR, toggle-only automerge, scheduled runner, sole updater).
if grep -q -F -e 'absent-only' "$automation" &&
  grep -q -F -e 'update-only' "$automation" &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'native-only' "$automation" &&
  grep -q -F -e 'dx bump' "$automation" &&
  grep -q -F -e 'one dep per PR' "$automation" &&
  grep -q -F -e 'bump.yml' "$automation"; then
  ok
else
  bad "automation.md lost the native-loop ownership (sole updater + native-only + dx bump + one-dep-per-PR + bump.yml)"
fi

# Native bump-PR verification loop documented: regen evidence, flag-diff
# review, full verification per the automation policy, plus the explicit
# widen-then-update steps with a clean-tree reset.
if grep -q -F -e 'regen' "$workflows_doc" &&
  grep -q -F -e 'flag-diff' "$workflows_doc" &&
  grep -q -F -e 'full verification' "$workflows_doc" &&
  grep -q -F -e 'automation.md' "$workflows_doc" &&
  grep -q -F -e 'dx bump' "$workflows_doc" &&
  grep -q -F -e 'dx update' "$workflows_doc" &&
  grep -q -F -e 'clean tree' "$workflows_doc"; then
  ok
else
  bad "local-workflows.md lost the native bump-PR loop (regen/flag-diff/verification/policy + dx bump/update + clean tree)"
fi

# ADR 0006 owns the narrow exception: `dx update` continues independent
# sets without a repository-wide rollback or a private scheduler.
if grep -q -F -e 'narrow exception' "$adr_surface" &&
  grep -q -F -e 'audit-update-bazel.md' "$adr_surface"; then
  ok
else
  bad "ADR 0006 lost the update narrow-exception pin or its contract link"
fi

# ADR 0008 owns the library-first pinning policy: latest stable,
# pinned exactly, checksummed where applicable.
if grep -q -F -e 'pinned exactly' "$adr_currency" &&
  grep -q -F -e 'latest stable' "$adr_currency"; then
  ok
else
  bad "ADR 0008 lost the exact-pin policy (pinned exactly + latest stable)"
fi

# Scheduled widen-then-update runner delivered: `dx bump` plus `widen-one`
# orchestration, one-dep-per-PR, toggle-only automerge, concurrency
# control, GITHUB_TOKEN PR creation, and fork-safety.
if [[ -f "$bump_workflow" ]] &&
  grep -q -F -e 'dx bump' "$bump_workflow" &&
  grep -q -F -e 'widen-one' "$bump_workflow" &&
  grep -q -F -e 'concurrency' "$bump_workflow" &&
  grep -q -F -e 'github.token' "$bump_workflow" &&
  grep -q -F -e 'Fork' "$bump_workflow" &&
  grep -q -F -e 'one dep per PR' "$bump_workflow"; then
  ok
else
  bad "bump.yml runner missing the widen-loop contract (dx bump + widen-one + concurrency + github.token + Fork + one-dep-per-PR)"
fi

# Bump-PR verify parity with docs (issue #641): regen + flag-diff + build +
# test (CI flaky/timeout flags) + coverage seed gate + dogfood gates; subset
# forever stays rejected as a dishonest gate.
if grep -q -F -e 'Bump-PR verification (regen, flag-diff, build, test, coverage, dogfood)' "$bump_workflow" &&
  grep -q -F -e 'coverage --min-coverage 97' "$bump_workflow" &&
  grep -q -F -e 'lint --check' "$bump_workflow" &&
  grep -q -F -e 'typecheck --check' "$bump_workflow" &&
  grep -q -F -e 'format --check' "$bump_workflow" &&
  grep -q -F -e 'audit security' "$bump_workflow" &&
  grep -q -F -e 'audit license' "$bump_workflow" &&
  grep -q -F -e '--flaky_test_attempts=3 --test_timeout=300' "$bump_workflow"; then
  ok
else
  bad "bump.yml verification lost parity with docs (want regen + flag-diff + build + test with flaky flags + coverage seed gate + lint/typecheck/format/audit dogfood, issue #641)"
fi

# Upstream scope pins intact: prerelease follows upstream, transitives
# stay resolver-governed, outside-requirements is not a failure.
if grep -q -F -e 'prerelease_follows_upstream' "$semantics" &&
  grep -q -F -e 'forces_transitive_newest' "$semantics" &&
  grep -q -F -e 'outside_requirements_is_failure' "$semantics" &&
  grep -q -F -e 'prerelease_follows_upstream' cli/bump/src/version.rs; then
  ok
else
  bad "upstream-scope pins lost (prerelease/transitives/scope in update + bump)"
fi

# Stale third-party-disposition hedge stays gone: the
# `until decides` hedge was resolved as native-only under
# , so no such hedge may reappear in the runner or the policy.
if ! grep -rn -F -e 'until issue #3' "$bump_workflow" "$automation" 2>/dev/null | grep -q . &&
  ! grep -rn -F -e 'until issue #3 decides' .github/workflows/bump.yml docs/contributing/automation.md 2>/dev/null | grep -q . &&
  grep -q -F -e 'sole updater' "$automation" &&
  grep -q -F -e 'issue #461' "$bump_workflow"; then
  ok
else
  bad "stale third-party hedge reappeared (want no until-issue-#3 hedge plus native-only under issue #461, issue #424)"
fi

dx_test_summary "widen update loop harness"
