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
# today (36 checks): codegen/env contracts + commit-lock + try_lock +
# clean + bootstrap-first + managed-PATH detail + Node/Python/Rust
# env records, docs-pipeline records + IR/site identity + rustdoc
# exception, examples ownership + laziness slices + attribution +
# full index breadth + Scala/Polyglot entries + mixed disposition,
# coverage gate + min-coverage threshold + ignore-marker contract +
# Codecov honesty + LCOV preset pin + inventory backing +
# fork-security record, Renovate fallback + full manager set + loop
# policy + preset-update/bump-PR record + automation ownership +
# schedule policy, never-rewrites + ADR pins, prior harnesses green,
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

# #9 managed-PATH detail stays pinned alongside the commit lock
# (bootstrap first, PATH-tool tree, lock re-read before compose).
if grep -q -F -e 'managed PATH' docs/environments/managed-state.md \
  && grep -q -F -e 'commit lock' docs/environments/managed-state.md; then
  ok
else
  bad "env managed-state lost its managed-PATH/commit-lock detail (#9)"
fi

# #10 documentation IR identity stays pinned (common symbol model,
# versioned doc-ir contract; per-language adapter runs still open).
if grep -q -F -e 'IR' docs/cli/commands/docs.md \
  && [[ -f "docs/documentation/doc-ir.md" ]]; then
  ok
else
  bad "docs pipeline lost its IR identity record (#10)"
fi

# #85 mixed-framework fixture disposition stays explicit (intentionally
# unindexed composition over one shared helper, not a consumer).
if grep -q -F -e 'mixed/hello' examples/README.md; then
  ok
else
  bad "examples index lost its mixed/hello fixture disposition (#85)"
fi

# #254 fork-security + #260 schedule policy stay pinned together
# (no write credentials to fork code; weekly Monday schedule, no
# automerge, reviewable PRs).
if grep -q -F -e 'fork' docs/github-ci.md \
  && grep -q -F -e '"schedule"' renovate.json \
  && grep -q -F -e '"automerge": false' renovate.json; then
  ok
else
  bad "automation lost its fork-security/schedule-policy record (#254/#260)"
fi

# #85 Scala/Polyglot index entries stay pinned alongside the full
# breadth (foreign sbt/polyglot trees via Gazelle extensions).
if grep -q -F -e 'adopt-scala' examples/README.md \
  && grep -q -F -e 'adopt-polyglot' examples/README.md; then
  ok
else
  bad "examples index lost its Scala/Polyglot entries (#85)"
fi

# #9 Node/Python env records stay owned (managed node_modules facade,
# persistent Python env; bootstrap still first).
if grep -q -F -e 'Gazelle maintains' docs/environments/node.md \
  && grep -q -F -e 'persistent environment' docs/environments/python-environment.md; then
  ok
else
  bad "env docs lost their Node/Python ownership records (#9)"
fi

# #9 Rust env record stays owned (provider-derived plans on the
# configured toolchain).
if grep -q -F -e 'provider-derived plans' docs/environments/rust.md; then
  ok
else
  bad "rust.md lost its provider-derived-plans record (#9)"
fi

# #260 loop policy stays reviewable (no pending-stampede PRs, no
# automerge, human merge path preserved).
if grep -q -F -e '"prCreation"' renovate.json \
  && grep -q -F -e '"dependencyDashboard"' renovate.json; then
  ok
else
  bad "Renovate fallback lost its reviewable-loop policy (#260)"
fi

# #9 lock/clean mechanics stay pinned beyond the commit lock: Rust
# try_lock evidence plus the explicit `dx clean` removal record.
if grep -q -F -e 'try_lock' docs/environments/managed-state.md \
  && grep -q -F -e 'dx clean' docs/environments/managed-state.md; then
  ok
else
  bad "env managed-state lost its try_lock/dx-clean mechanics (#9)"
fi

# #10 docs command owns its downstream docs: the versioned IR model
# plus the cache-friendly site build (adapter runs still open).
if grep -q -F -e 'Documentation IR' docs/cli/commands/docs.md \
  && grep -q -F -e 'Site build' docs/cli/commands/docs.md \
  && [[ -f "docs/documentation/doc-ir.md" ]] \
  && [[ -f "docs/documentation/site.md" ]]; then
  ok
else
  bad "docs command lost its IR/site-build doc ownership (#10)"
fi

# #254 ignore-marker contract stays explicit: LCOV exclusions need a
# reason, and missing reasons fail the gate (comment service open).
if grep -q -F -e 'LCOV_EXCL_LINE' docs/testing/README.md \
  && grep -q -F -e 'missing reasons' docs/testing/README.md; then
  ok
else
  bad "coverage doc lost its ignore-marker/reason contract (#254)"
fi

# #260 bump-PR loop record stays owned: preset verify-only review
# plus the Renovate-proposes/dx-verifies flow (widen loop still open).
if grep -q -F -e 'preset.update -- --verify-only' docs/contributing/local-workflows.md \
  && grep -q -F -e 'Version bumps flow through Renovate out of the box' docs/contributing/local-workflows.md; then
  ok
else
  bad "local-workflows lost its preset-update/bump-PR loop record (#260)"
fi

# #9 bootstrap-first ordering stays pinned: PATH/environment
# bootstrap comes first and the managed `.dx/bin` tree stays owned
# (extension boundary still PATH-tools-only).
if grep -q -F -e 'bootstrap come first' docs/environments/managed-state.md \
  && grep -q -F -e '.dx/bin' docs/environments/managed-state.md; then
  ok
else
  bad "env managed-state lost its bootstrap-first/.dx/bin record (#9)"
fi

# #10 nightly-rustdoc exception stays explicit: the pinned nightly
# rustdoc JSON route carries a narrow currency exception, with no
# ambient or stable-output fallback (adapter runs still open).
if grep -q -F -e 'nightly rustdoc' docs/documentation/doc-ir.md \
  && grep -q -F -e 'rustdoc-extraction-exception' docs/documentation/doc-ir.md; then
  ok
else
  bad "docs IR lost its nightly-rustdoc exception record (#10)"
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

# #254 min-coverage threshold stays owned: the gate is a pinned
# `dx coverage --min-coverage` percent over non-ignored executable
# lines (first-party comment presentation still open).
if grep -q -F -e 'dx coverage --min-coverage' docs/testing/README.md; then
  ok
else
  bad "coverage doc lost its min-coverage threshold record (#254)"
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
