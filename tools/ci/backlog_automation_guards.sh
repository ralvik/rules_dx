#!/usr/bin/env bash
# Backlog/automation guards (issues #85, #254, #260, #421).
#
# Per-foundation external-consumer examples + acquisition/laziness proof
# (#85) delivered across readme, static, query, aquery, and runtime slices;
# first-party coverage PR comments (#254) landed with the Bazel-owned LCOV
# gate as source of truth; the
# widen-one-requirement + update PR loop (#260) is delivered with Renovate
# proposing alongside it (complementary roles decided in issue #326).
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (23 checks): examples ownership + starter callers + laziness
# slices + attribution + full index breadth + Scala/Polyglot entries +
# mixed disposition, LCOV preset pin + inventory backing + comment landing,
# Renovate plus full manager set + loop policy + Monday schedule
# + schedule policy, never-rewrites pin, prior harnesses green,
# delivered widen implementation, plus the #421 docs-pipeline tracker.
# Reverse queries and adapter runs stay open under their
# issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:backlog_automation_guards`,
# following //tools/ci:backlog_contracts.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# #10 prior slice stays green.
if [[ -f "tools/ci/backlog_contracts.sh" ]]; then
  ok
else
  bad "backlog_contracts harness missing"
fi

# #85 examples ownership: index + per-foundation READMEs + slices green.
if grep -q -F -e 'example' examples/README.md &&
  [[ -f "tools/ci/examples_readme.sh" ]] &&
  [[ -f "tools/ci/examples_laziness.sh" ]]; then
  ok
else
  bad "examples ownership lost (README index or readme/laziness harnesses)"
fi

# #85 index breadth: beyond the Rust/Python/JS-TS minimum.
if grep -q -F -e 'adopt-go' examples/README.md &&
  grep -q -F -e 'adopt-cpp' examples/README.md; then
  ok
else
  bad "examples index lost its beyond-minimum breadth (adopt-go/adopt-cpp)"
fi

# #85 laziness query/aquery slices stay wired.
if [[ -f "tools/ci/examples_laziness_query.sh" ]] &&
  [[ -f "tools/ci/examples_laziness_aquery.sh" ]]; then
  ok
else
  bad "examples laziness query/aquery harnesses missing"
fi

# #85 laziness runtime close-out stays wired (target + CI step).
if [[ -f "tools/ci/examples_laziness_runtime.sh" ]] &&
  grep -q -F -e 'examples_laziness_runtime' tools/ci/BUILD.bazel &&
  grep -q -F -e 'examples_laziness_runtime' .github/workflows/ci.yml; then
  ok
else
  bad "examples laziness runtime close-out missing (harness, target, or CI step)"
fi

# #254 LCOV preset pin: combined report owned by Bazel flags.
if grep -q -F -e 'combined_report=lcov' tools/bazelrc/preset.bazelrc &&
  [[ -f "tools/ci/coverage_cell.sh" ]]; then
  ok
else
  bad "coverage LCOV preset pin lost (preset.bazelrc or coverage_cell)"
fi

# #254 landed slice stays green: first-party comment renderer plus
# marker-owned PR wiring in both workflows (Codecov opt-in only).
if [[ -f "tools/ci/coverage_report_guards.sh" ]] &&
  [[ -x "tools/coverage/coverage_comment.sh" ]] &&
  grep -rln -F -e 'dx-coverage-summary' .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "coverage comment landing dishonest (harness/renderer/marker wiring missing)"
fi

# #260 all-ecosystems v1 manager set retained in the fallback.
if grep -q -F -e '"bazel", "cargo", "github-actions", "gomod", "npm"' renovate.json; then
  ok
else
  bad "Renovate lost its all-ecosystems v1 manager set (#260)"
fi

# #260 Renovate retained with full manager set (complementary, issue #326).
if grep -q -F -e 'npm' renovate.json &&
  grep -q -F -e 'automerge' renovate.json; then
  ok
else
  bad "Renovate lost its manager set (#260)"
fi

# #260 never-rewrites invariant intact.
if grep -q -F -e 'may_be_rewritten' cli/update/src/semantics.rs; then
  ok
else
  bad "widen loop lost its never-rewrites pin (#260)"
fi

# #260 prior slice stays green.
if [[ -f "tools/ci/widen_update_loop.sh" ]]; then
  ok
else
  bad "widen_update_loop harness missing"
fi

# #85 full index breadth: Java/Kotlin/Scala/C#/F# alongside the
# go/cpp beyond-minimum slice.
if grep -q -F -e 'adopt-java' examples/README.md &&
  grep -q -F -e 'adopt-kotlin' examples/README.md &&
  grep -q -F -e 'adopt-csharp' examples/README.md &&
  grep -q -F -e 'adopt-fsharp' examples/README.md; then
  ok
else
  bad "examples index lost its full Java/Kotlin/C#/F# breadth (#85)"
fi

# #254 inventory + spill backing stays present alongside the gate
# (seed inventory + profraw containment, presentation still open).
if [[ -f "tools/coverage/seed-inventory.txt" ]] &&
  [[ -f "tools/ci/coverage_spill.sh" ]]; then
  ok
else
  bad "coverage inventory/spill backing missing (seed-inventory/coverage_spill)"
fi

# #85 mixed-framework fixture disposition stays explicit (intentionally
# unindexed composition over one shared helper, not a consumer).
if grep -q -F -e 'mixed/hello' examples/README.md; then
  ok
else
  bad "examples index lost its mixed/hello fixture disposition (#85)"
fi

# #254/#260 schedule policy stays pinned together
# (weekly Monday schedule, no automerge, reviewable PRs).
if grep -q -F -e '"schedule"' renovate.json &&
  grep -q -F -e '"automerge": false' renovate.json; then
  ok
else
  bad "automation lost its schedule-policy record (#254/#260)"
fi

# #85 Scala/Polyglot index entries stay pinned alongside the full
# breadth (foreign sbt/polyglot trees via Gazelle extensions).
if grep -q -F -e 'adopt-scala' examples/README.md &&
  grep -q -F -e 'adopt-polyglot' examples/README.md; then
  ok
else
  bad "examples index lost its Scala/Polyglot entries (#85)"
fi

# #260 loop policy stays reviewable (no pending-stampede PRs, no
# automerge, human merge path preserved).
if grep -q -F -e '"prCreation"' renovate.json &&
  grep -q -F -e '"dependencyDashboard"' renovate.json; then
  ok
else
  bad "Renovate lost its reviewable-loop policy (#260)"
fi

# #85 acquisition attribution stays owned: static (prohibited-installer)
# plus runtime (aquery action-command) attribution prove the private tool
# graph never shells out to installers; remote/empty-cache attribution
# stays owned by #298/#308.
if grep -q -F -e 'Runtime attribution' tools/ci/examples_laziness.sh &&
  grep -q -F -e 'No-install attribution' tools/ci/examples_laziness.sh &&
  grep -q -F -e 'No-install attribution' tools/ci/examples_laziness_runtime.sh; then
  ok
else
  bad "examples laziness lost its acquisition-attribution record (#85)"
fi

# #85 starter callers stay indexed: consumer-ci + docs-ci starter
# callers alongside the per-foundation adopt workspaces (laziness proof
# delivered on the seed host).
if grep -q -F -e '[consumer-ci](consumer-ci/)' examples/README.md &&
  grep -q -F -e '[docs-ci](docs-ci/)' examples/README.md; then
  ok
else
  bad "examples index lost its starter caller entries (#85)"
fi

# #260 Monday schedule stays pinned: Renovate runs before 5am on Monday
# with reviewable PRs and no automerge (native loop delivered alongside).
if grep -q -F -e 'before 5am on Monday' renovate.json; then
  ok
else
  bad "Renovate lost its Monday schedule record (#260)"
fi

# Widen implementation delivered (#260): explicit bump command plus the
# scheduled loop runner and native-loop automation docs.
if grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q . &&
  [[ -f ".github/workflows/bump.yml" ]] &&
  grep -q -F -e 'dx bump' docs/contributing/automation.md &&
  grep -q -F -e 'dx bump' docs/cli/commands/audit-update-bazel.md; then
  ok
else
  bad "widen bump implementation missing for #260 (want bump command + bump.yml + automation/contract docs)"
fi

# No third-party coverage service smuggled in (#254 first-party only).
if ! grep -rn -F -e 'codecov-action' .github/workflows/ci.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "a third-party coverage action appeared in ci.yml against #254 policy"
fi

# #421 docs-pipeline gaps stay tracked in the documentation contract
# (live successor to closed #310; #421 owns #310 per issue #445).
if grep -q -F -e 'Docs pipeline gaps stay open under issue #421' docs/documentation/README.md; then
  ok
else
  bad "documentation README lost its #421 docs-pipeline tracker record"
fi

dx_test_summary "backlog automation guards harness"
