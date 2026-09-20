#!/usr/bin/env bash
# Backlog/automation guards.
#
# Per-foundation external-consumer examples + acquisition/laziness proof
# delivered across readme, static, query, aquery, and runtime slices;
# first-party coverage PR comments landed with the Bazel-owned LCOV
# gate as source of truth; the
# widen-one-requirement + update PR loop is delivered as the sole
# updater (native-only,).
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (25 checks): examples ownership + starter callers + laziness
# slices + attribution + full index breadth + Scala/Polyglot entries +
# mixed disposition, LCOV preset pin + inventory backing + comment landing,
# native set registry + sole-updater policy + Monday schedule
# + native-only policy, never-rewrites pin, prior harnesses green,
# delivered widen implementation, plus the docs-pipeline tracker.
# Reverse queries and adapter runs stay open under their
# issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:backlog_automation_guards`,
# following //tools/ci:backlog_contracts.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

# prior slice stays green.
if [[ -f "tools/ci/backlog_contracts.sh" ]]; then
  ok
else
  bad "backlog_contracts harness missing"
fi

# examples ownership: index + per-foundation READMEs + slices green.
if grep -q -F -e 'example' examples/README.md &&
  [[ -f "tools/ci/examples_readme.sh" ]] &&
  [[ -f "tools/ci/examples_laziness.sh" ]]; then
  ok
else
  bad "examples ownership lost (README index or readme/laziness harnesses)"
fi

# index breadth: beyond the Rust/Python/JS-TS minimum.
if grep -q -F -e 'adopt-go' examples/README.md &&
  grep -q -F -e 'adopt-cpp' examples/README.md; then
  ok
else
  bad "examples index lost its beyond-minimum breadth (adopt-go/adopt-cpp)"
fi

# laziness query/aquery slices stay wired.
if [[ -f "tools/ci/examples_laziness_query.sh" ]] &&
  [[ -f "tools/ci/examples_laziness_aquery.sh" ]]; then
  ok
else
  bad "examples laziness query/aquery harnesses missing"
fi

# laziness runtime close-out stays wired (target + CI step).
if [[ -f "tools/ci/examples_laziness_runtime.sh" ]] &&
  grep -q -F -e 'examples_laziness_runtime' tools/ci/BUILD.bazel &&
  grep -q -F -e 'examples_laziness_runtime' .github/workflows/ci.yml; then
  ok
else
  bad "examples laziness runtime close-out missing (harness, target, or CI step)"
fi

# LCOV preset pin: combined report owned by Bazel flags.
if grep -q -F -e 'combined_report=lcov' tools/bazelrc/preset.bazelrc &&
  [[ -f "tools/ci/coverage_cell.sh" ]]; then
  ok
else
  bad "coverage LCOV preset pin lost (preset.bazelrc or coverage_cell)"
fi

# landed slice stays green: first-party comment renderer plus
# marker-owned PR wiring in both workflows (Codecov opt-in only).
if [[ -f "tools/ci/coverage_report_guards.sh" ]] &&
  [[ -x "tools/coverage/coverage_comment.sh" ]] &&
  grep -rln -F -e 'dx-coverage-summary' .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "coverage comment landing dishonest (harness/renderer/marker wiring missing)"
fi

# all-ecosystems v1 set pinned in the native registry.
if grep -q -F -e 'BumpSet::Bazel' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::Cargo' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::GithubActions' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::Go' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::Maven' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::Npm' cli/bump/src/sets.rs &&
  grep -q -F -e 'BumpSet::NuGet' cli/bump/src/sets.rs; then
  ok
else
  bad "native set registry lost its all-ecosystems v1 set (#260)"
fi

# sole updater ships an eight-file scaffold with no updater config.
if grep -q -F -e 'files.len(), 8' cli/adopt/src/scaffold.rs &&
  grep -q -F -e 'ships no updater config' docs/contributing/automation.md; then
  ok
else
  bad "native scaffold lost its sole-updater shape (#260/#461)"
fi

# never-rewrites invariant intact.
if grep -q -F -e 'may_be_rewritten' cli/update/src/semantics.rs; then
  ok
else
  bad "widen loop lost its never-rewrites pin (#260)"
fi

# prior slice stays green.
if [[ -f "tools/ci/widen_update_loop.sh" ]]; then
  ok
else
  bad "widen_update_loop harness missing"
fi

# full index breadth: Java/Kotlin/Scala/C#/F# alongside the
# go/cpp beyond-minimum slice.
if grep -q -F -e 'adopt-java' examples/README.md &&
  grep -q -F -e 'adopt-kotlin' examples/README.md &&
  grep -q -F -e 'adopt-csharp' examples/README.md &&
  grep -q -F -e 'adopt-fsharp' examples/README.md; then
  ok
else
  bad "examples index lost its full Java/Kotlin/C#/F# breadth (#85)"
fi

# inventory + spill backing stays present alongside the gate
# (seed inventory + profraw containment, presentation still open).
if [[ -f "tools/coverage/seed-inventory.txt" ]] &&
  [[ -f "tools/ci/coverage_spill.sh" ]]; then
  ok
else
  bad "coverage inventory/spill backing missing (seed-inventory/coverage_spill)"
fi

# mixed-framework fixture disposition stays explicit (intentionally
# unindexed composition over one shared helper, not a consumer).
if grep -q -F -e 'mixed/hello' examples/README.md; then
  ok
else
  bad "examples index lost its mixed/hello fixture disposition (#85)"
fi

# / schedule policy stays pinned together
# (weekly Monday schedule, toggle-only automerge, reviewable PRs).
if grep -q -F -e 'before 5am' .github/workflows/bump.yml &&
  grep -q -F -e 'sole updater' .github/workflows/bump.yml &&
  grep -q -F -e 'update-only' docs/contributing/automation.md; then
  ok
else
  bad "automation lost its schedule-policy record (#254/#260)"
fi

# Scala/Polyglot index entries stay pinned alongside the full
# breadth (foreign sbt/polyglot trees via Gazelle extensions).
if grep -q -F -e 'adopt-scala' examples/README.md &&
  grep -q -F -e 'adopt-polyglot' examples/README.md; then
  ok
else
  bad "examples index lost its Scala/Polyglot entries (#85)"
fi

# loop policy stays reviewable (one dep per PR, toggle-only
# automerge, human merge path preserved).
if grep -q -F -e 'one dep per PR' docs/contributing/automation.md &&
  grep -q -F -e 'sole updater' docs/contributing/automation.md &&
  grep -q -F -e 'native-only' docs/contributing/automation.md; then
  ok
else
  bad "native loop lost its reviewable-loop policy (#260)"
fi

# acquisition attribution stays owned: static (prohibited-installer)
# plus runtime (aquery action-command) attribution prove the private tool
# graph never shells out to installers; remote/empty-cache attribution
# stays owned by /.
if grep -q -F -e 'Runtime attribution' tools/ci/examples_laziness.sh &&
  grep -q -F -e 'No-install attribution' tools/ci/examples_laziness.sh &&
  grep -q -F -e 'No-install attribution' tools/ci/examples_laziness_runtime.sh; then
  ok
else
  bad "examples laziness lost its acquisition-attribution record (#85)"
fi

# starter callers stay indexed: consumer-ci + docs-ci starter
# callers alongside the per-foundation adopt workspaces (laziness proof
# delivered on the seed host).
if grep -q -F -e '[consumer-ci](consumer-ci/)' examples/README.md &&
  grep -q -F -e '[docs-ci](docs-ci/)' examples/README.md; then
  ok
else
  bad "examples index lost its starter caller entries (#85)"
fi

# Monday schedule stays pinned: native loop runs before 5am on Monday
# with reviewable PRs and toggle-only automerge (sole updater,).
if grep -q -F -e 'before 5am' .github/workflows/bump.yml &&
  grep -q -F -e 'issue #461' .github/workflows/bump.yml &&
  grep -q -F -e 'issue #461' docs/contributing/automation.md; then
  ok
else
  bad "native loop lost its Monday schedule record (#260/#461)"
fi

# Widen implementation delivered: explicit bump command plus the
# scheduled loop runner and native-loop automation docs.
if grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q . &&
  [[ -f ".github/workflows/bump.yml" ]] &&
  grep -q -F -e 'dx bump' docs/contributing/automation.md &&
  grep -q -F -e 'dx bump' docs/cli/commands/audit-update-bazel.md; then
  ok
else
  bad "widen bump implementation missing for #260 (want bump command + bump.yml + automation/contract docs)"
fi

# No third-party coverage service smuggled in (first-party only).
if ! grep -rn -F -e 'codecov-action' .github/workflows/ci.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "a third-party coverage action appeared in ci.yml against #254 policy"
fi

# docs-pipeline gaps stay tracked in the documentation contract
# (live successor to closed; owns).
if grep -q -F -e 'Docs pipeline gaps stay open under issue #581' docs/documentation/README.md; then
  ok
else
  bad "documentation README lost its #581 docs-pipeline tracker record"
fi

# Mxx/Oxx de-milestoning stays clean plus Stage-N (cross-links hygiene,
# no duplicate guard there): no new milestone specs, register entries, or Stage-N
# close-out labels outside the curated CHANGELOG.md delivery-record note (which lives in
# git history; see `git log --all --oneline` for the Mxx/Oxx entries).
if git grep -n -E -e '\bM[0-9]{2}[a-z]?\b|\bO[0-9]{1,2}\b|\bStage[ -][0-9]' -- ':!CHANGELOG.md' ':!pnpm-lock.yaml' ':!*.lock' 2>/dev/null | grep -q .; then
  bad "new Mxx/Oxx/Stage-N milestone references appeared (use ADR/contract/issue tracker; history lives in CHANGELOG.md plus git log)"
else
  ok
fi

# bare open/planned work stays owned (same family as / guards):
# every `open work` or `planned work` line in docs/ must name its owner on the
# same line via `issue(s) #` or `roadmap` (owner plus ADR/contract/issue link
# or roadmap entry,; no new Mxx/Oxx).
if git grep -n -i -E -e 'open work|planned work' -- docs/ 2>/dev/null | grep -v -E -e 'issues? #' | grep -v -i -e 'roadmap' | grep -q .; then
  bad "bare open/planned work appeared (name owner plus ADR/contract/issue link or roadmap entry on the same line, issue #446)"
else
  ok
fi

dx_test_summary "backlog automation guards harness"
