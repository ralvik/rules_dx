#!/usr/bin/env bash
# Backlog/automation guards (issues #85, #254, #260).
#
# Per-foundation external-consumer examples + acquisition/laziness proof
# (#85) grow slice by slice; first-party coverage PR comments (#254)
# stay open with the Bazel-owned LCOV gate as source of truth; the
# widen-one-requirement + update PR loop (#260) stays planned behind
# the #19 resolver prerequisite with Renovate retained as fallback.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (21 checks): examples ownership + starter callers + laziness
# slices + attribution + full index breadth + Scala/Polyglot entries +
# mixed disposition, LCOV preset pin + inventory backing,
# Renovate fallback + full manager set + loop policy + Monday schedule
# + schedule policy, never-rewrites pin, prior harnesses green,
# and no-false-claim gaps. Reverse queries, adapter runs, comment
# presentation, and widen implementation stay open under their
# issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:backlog_automation_guards`,
# following //tools/ci:backlog_contracts.
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

# #10 prior slice stays green.
if [[ -f "tools/ci/backlog_contracts.sh" ]]; then
  ok
else
  bad "backlog_contracts harness missing"
fi

# #85 examples ownership: index + per-foundation READMEs + slices green.
if grep -q -F -e 'example' examples/README.md \
  && [[ -f "tools/ci/examples_readme.sh" ]] \
  && [[ -f "tools/ci/examples_laziness.sh" ]]; then
  ok
else
  bad "examples ownership lost (README index or readme/laziness harnesses)"
fi

# #85 index breadth: beyond the Rust/Python/JS-TS minimum.
if grep -q -F -e 'adopt-go' examples/README.md \
  && grep -q -F -e 'adopt-cpp' examples/README.md; then
  ok
else
  bad "examples index lost its beyond-minimum breadth (adopt-go/adopt-cpp)"
fi

# #85 laziness query/aquery slices stay wired.
if [[ -f "tools/ci/examples_laziness_query.sh" ]] \
  && [[ -f "tools/ci/examples_laziness_aquery.sh" ]]; then
  ok
else
  bad "examples laziness query/aquery harnesses missing"
fi

# #254 LCOV preset pin: combined report owned by Bazel flags.
if grep -q -F -e 'combined_report=lcov' tools/bazelrc/preset.bazelrc \
  && [[ -f "tools/ci/coverage_cell.sh" ]]; then
  ok
else
  bad "coverage LCOV preset pin lost (preset.bazelrc or coverage_cell)"
fi

# #254 prior slice stays green, no comment workflow falsely claimed.
if [[ -f "tools/ci/coverage_report_guards.sh" ]] \
  && ! grep -rln -F -e 'coverage-summary-comment' .github/workflows/ 2>/dev/null | grep -q .; then
  ok
else
  bad "coverage comment gap dishonest (harness missing or comment workflow claimed)"
fi

# #260 all-ecosystems v1 manager set retained in the fallback.
if grep -q -F -e '"bazel", "cargo", "github-actions", "gomod", "npm"' renovate.json; then
  ok
else
  bad "Renovate fallback lost its all-ecosystems v1 manager set (#260)"
fi

# #260 Renovate fallback retained with full manager set.
if grep -q -F -e 'npm' renovate.json \
  && grep -q -F -e 'automerge' renovate.json; then
  ok
else
  bad "Renovate fallback lost its manager set (#260)"
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
if grep -q -F -e 'adopt-java' examples/README.md \
  && grep -q -F -e 'adopt-kotlin' examples/README.md \
  && grep -q -F -e 'adopt-csharp' examples/README.md \
  && grep -q -F -e 'adopt-fsharp' examples/README.md; then
  ok
else
  bad "examples index lost its full Java/Kotlin/C#/F# breadth (#85)"
fi

# #254 inventory + spill backing stays present alongside the gate
# (seed inventory + profraw containment, presentation still open).
if [[ -f "tools/coverage/seed-inventory.txt" ]] \
  && [[ -f "tools/ci/coverage_spill.sh" ]]; then
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
if grep -q -F -e '"schedule"' renovate.json \
  && grep -q -F -e '"automerge": false' renovate.json; then
  ok
else
  bad "automation lost its schedule-policy record (#254/#260)"
fi

# #85 Scala/Polyglot index entries stay pinned alongside the full
# breadth (foreign sbt/polyglot trees via Gazelle extensions).
if grep -q -F -e 'adopt-scala' examples/README.md \
  && grep -q -F -e 'adopt-polyglot' examples/README.md; then
  ok
else
  bad "examples index lost its Scala/Polyglot entries (#85)"
fi

# #260 loop policy stays reviewable (no pending-stampede PRs, no
# automerge, human merge path preserved).
if grep -q -F -e '"prCreation"' renovate.json \
  && grep -q -F -e '"dependencyDashboard"' renovate.json; then
  ok
else
  bad "Renovate fallback lost its reviewable-loop policy (#260)"
fi

# #85 acquisition attribution stays owned: runtime (logs/aquery) and
# static (prohibited-installer) attribution prove the private tool
# graph never shells out to installers (full laziness proof open).
if grep -q -F -e 'Runtime attribution' tools/ci/examples_laziness.sh \
  && grep -q -F -e 'No-install attribution' tools/ci/examples_laziness.sh; then
  ok
else
  bad "examples laziness lost its acquisition-attribution record (#85)"
fi

# #85 starter callers stay indexed: consumer-ci + docs-ci starter
# callers alongside the per-foundation adopt workspaces (full
# laziness proof still open).
if grep -q -F -e '[consumer-ci](consumer-ci/)' examples/README.md \
  && grep -q -F -e '[docs-ci](docs-ci/)' examples/README.md; then
  ok
else
  bad "examples index lost its starter caller entries (#85)"
fi

# #260 Monday schedule stays pinned: Renovate runs before 5am on Monday
# with reviewable PRs and no automerge (widen loop still open).
if grep -q -F -e 'before 5am on Monday' renovate.json; then
  ok
else
  bad "Renovate fallback lost its Monday schedule record (#260)"
fi

# No widen implementation falsely claimed.
if ! grep -rn -F -e '"bump"' --include='*.rs' cli/ 2>/dev/null | grep -q .; then
  ok
else
  bad "a widen bump command appeared in cli/ without #260 landing"
fi

# No first-party comment service falsely claimed.
if ! grep -rn -F -e 'codecov-action' .github/workflows/ci.yml 2>/dev/null | grep -q .; then
  ok
else
  bad "a third-party coverage action appeared in ci.yml against #254 policy"
fi

echo "backlog automation guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
