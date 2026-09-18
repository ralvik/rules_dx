#!/usr/bin/env bash
# Backlog/automation guards (issues #9, #10, #85, #254, #260).
#
# Environment/codegen (#9) and docs-pipeline (#10) keep frozen
# cross-file contracts with honest gap labels; per-foundation
# external-consumer examples + acquisition/laziness proof (#85) grow
# slice by slice; first-party coverage PR comments (#254) stay open
# with the Bazel-owned LCOV gate as source of truth; the
# widen-one-requirement + update PR loop (#260) stays planned behind
# the #19 resolver prerequisite with Renovate retained as fallback.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (20 checks): codegen/env contracts + commit-lock detail,
# docs-pipeline records, examples ownership + laziness slices + full
# index breadth, coverage gate + Codecov honesty + LCOV preset pin +
# inventory backing, Renovate fallback + full manager set + automation
# ownership, never-rewrites + ADR pins, prior harnesses green, and
# no-false-claim gaps. Reverse queries, adapter runs, comment
# presentation, and widen implementation stay open under their issues.
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

# #9 codegen command ownership: dx codegen builds Bazel-owned artifacts.
if grep -q -F -e 'dx codegen' docs/environments/codegen.md; then
  ok
else
  bad "codegen.md lost its dx codegen ownership (#9)"
fi

# #9 env/codegen contracts owned in docs.
if grep -q -F -e 'codegen' docs/environments/codegen.md \
  && grep -q -F -e 'commit' docs/environments/managed-state.md; then
  ok
else
  bad "env/codegen docs lost their codegen/commit ownership (#9)"
fi

# #10 docs-pipeline record present, no Supported claim smuggled.
if grep -q -F -e 'docs' docs/cli/commands/docs.md \
  && [[ -f "tools/ci/backlog_contracts.sh" ]]; then
  ok
else
  bad "docs-pipeline record lost (docs.md command or backlog_contracts harness)"
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

# #254 gate stays Bazel-owned LCOV, Codecov selection stays honest.
if grep -q -F -e 'LCOV' docs/testing/README.md \
  && grep -q -F -e 'Codecov' docs/testing/README.md; then
  ok
else
  bad "coverage doc lost its LCOV gate or Codecov-selection record (#254)"
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
  && grep -q -F -e 'automerge' renovate.json \
  && grep -q -F -e 'Renovate' docs/contributing/automation.md; then
  ok
else
  bad "Renovate fallback lost its manager set or automation ownership (#260)"
fi

# #260 never-rewrites invariant + ADR pins intact.
if grep -q -F -e 'may_be_rewritten' cli/update/src/semantics.rs \
  && grep -q -F -e 'narrow exception' docs/decisions/0006-cli-command-surface.md \
  && grep -q -F -e 'pinned exactly' docs/decisions/0008-dependency-currency.md; then
  ok
else
  bad "widen loop lost its never-rewrites or ADR 0006/0008 pins (#260)"
fi

# #260 prior slice stays green.
if [[ -f "tools/ci/widen_update_loop.sh" ]]; then
  ok
else
  bad "widen_update_loop harness missing"
fi

# #9 commit-lock detail stays pinned beyond the broad commit marker
# (parallel composition under the lock, atomic codegen commit).
if grep -q -F -e 'commit lock' docs/environments/managed-state.md \
  && grep -q -F -e 'atomic commit' docs/environments/codegen.md; then
  ok
else
  bad "env/codegen docs lost their commit-lock/atomic-commit detail (#9)"
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

# #260 automation ownership stays explicit (Renovate chosen updater,
# bump-PR review path, issue #3 link).
if grep -q -F -e 'Renovate' docs/contributing/automation.md \
  && grep -q -F -e 'issue #3' docs/contributing/automation.md; then
  ok
else
  bad "automation.md lost its Renovate ownership / issue #3 link (#260)"
fi

# Matrix honesty for this group.
if grep -q -F -e 'Open (#10)' docs/testing/verification-matrix.md \
  && grep -q -F -e 'Open (#9)' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #9/#10 honesty markers"
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
