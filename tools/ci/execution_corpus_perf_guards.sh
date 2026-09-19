#!/usr/bin/env bash
# Execution/corpus/perf guards (issues #18, #19, #22, #15, #86).
#
# Live `dx audit` executes qualified auditors per family with SARIF/SPDX mapping;
# live `dx update` executes
# resolver-owned backends per set with continuation and per-set reporting
# (issues #18 and #19 delivered). Family selection plus update selector planning,
# aggregate exit-code mapping, and --dry-run planning execute for both. Required-core depcheck (issue #22) is delivered
# in tools/depcheck/; admitted expansion stays open. Corpus stays single `corpus` per directory until `dx generate` emits
# the per-type split; perf tracks the frozen aspect_rules_lint v2.8.0
# baseline as a report, never a gate.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (21 checks): audit plus update live execution, fail-closed/live unit pins, dry-run
# planning paths, audit families + policy modules, selector planning
# + update API, aggregate verdict pins, exit mappings,
# prior harnesses green, corpus single-name rule + Gazelle ownership +
# generate --check wiring + carve-out record, perf report-not-gate
# shape + v2.8.0 fairness pin + bench harness + comparison-test
# presence. Update plus audit live execution
# is delivered (#19 plus #18); per-type generation and comparison numbers stay open under
# their issues.
#
# Versioned here, run by CI via `bazel run //tools/ci:execution_corpus_perf_guards`,
# following //tools/ci:verify_perf_corpus.
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

# #18 live audit code pinned in CLI.
if grep -q -F -e 'CODE_AUDIT_FAILED' cli/cli/src/exec/common.rs \
  && grep -q -F -e 'dx_audit::backend::plan_secrets' cli/cli/src/exec/audit.rs \
  && ! grep -q -F -e 'audit_deferred' cli/cli/src/exec/audit.rs \
  && ! grep -q -F -e 'CODE_AUDIT_DEFERRED' cli/cli/src/exec/common.rs; then
  ok
else
  bad "live audit lost its live-execution code (want CODE_AUDIT_FAILED, no audit_deferred)"
fi

# #19 live update execution pinned in CLI (no deferred code).
if grep -q -F -e 'CODE_UPDATE_FAILED' cli/cli/src/exec/common.rs \
  && grep -q -F -e 'dx_update::backend::plan' cli/cli/src/exec/update.rs \
  && ! grep -q -F -e 'CODE_UPDATE_DEFERRED' cli/cli/src/exec/common.rs \
  && ! grep -q -F -e 'update_deferred' cli/cli/src/exec/update.rs; then
  ok
else
  bad "live update lost its live-execution code (want CODE_UPDATE_FAILED, no update_deferred)"
fi

# #18 live plus #19 live behavior pinned by unit tests, not just codes.
if grep -q -F -e 'assert!(err.contains("audit_failed")' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'assert!(err.contains("update_failed")' cli/cli/src/exec/update.rs; then
  ok
else
  bad "audit/update lost their live unit-test pins"
fi

# #18/#19 dry-run planning paths execute without launching.
if grep -q -F -e 'dry_run' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'dry_run' cli/cli/src/exec/update.rs; then
  ok
else
  bad "audit/update lost their dry-run planning paths"
fi

# Aggregate exit-code mappings stay unit-pinned.
if grep -q -F -e 'pub fn exit_code' cli/audit/src/outcome.rs \
  && grep -q -F -e 'overall_failure' cli/update/src/report.rs; then
  ok
else
  bad "audit/update lost their aggregate exit-code mappings"
fi

# #22 delivered slice stays green with fixtures landed.
if [[ -f "tools/ci/depcheck_contract.sh" ]] \
  && [[ -f "tools/ci/audit_update_guards.sh" ]] \
  && [[ -f "tools/depcheck/depcheck.py" ]]; then
  ok
else
  bad "prior execution harnesses missing (depcheck_contract/audit_update_guards/depcheck)"
fi

if [[ -d "tools/depcheck/testdata/rust/ok_used" ]] \
  && [[ -d "tools/depcheck/testdata/python/ok_used" ]]; then
  ok
else
  bad "#22 fixtures missing after delivery"
fi

# #15 corpus rule: single `corpus` per directory until generation owns split.
if [[ -f "tools/ci/corpus_audit.sh" ]] \
  && grep -q -F -e 'attr(name,' .github/workflows/ci.yml; then
  ok
else
  bad "corpus single-name rule lost (corpus_audit harness or ci corpus query)"
fi

# #15 generation ownership: Gazelle extension present, no hand split claimed.
if [[ -f "gazelle/rust/lang.go" ]] \
  && ! grep -rln -F -e 'corpus_markdown' --include='BUILD.bazel' . 2>/dev/null | grep -q .; then
  ok
else
  bad "corpus per-type split appeared without #15 generation landing"
fi

# #15 generate freshness enforced in CI alongside the audit.
if grep -q -F -e 'generate --check //...' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost its generate --check freshness gate (#15)"
fi

# #86 perf report-not-gate shape: workflow + comparator present.
if [[ -f ".github/workflows/perf.yml" ]] \
  && [[ -f "perf/compare.py" ]]; then
  ok
else
  bad "perf harness missing (perf.yml workflow or compare.py)"
fi

# #86 fairness pin recorded in results (report, never gate).
if grep -q -F -e '"rules_lint_pin": "v2.8.0"' perf/rules_lint_results.json; then
  ok
else
  bad "perf lost its rules_lint v2.8.0 fairness pin (results)"
fi

# #86 prior slice stays green.
if [[ -f "tools/ci/verify_perf_corpus.sh" ]]; then
  ok
else
  bad "verify_perf_corpus harness missing"
fi

# #15 corpus carve-out record stays explicit (integration scenario
# workspaces owned outside the audit, no silent skip).
if grep -q -F -e 'carve-out' tools/ci/corpus_audit.sh; then
  ok
else
  bad "corpus_audit lost its carve-out record (#15)"
fi

# #86 bench regeneration harness stays present alongside the comparator
# (synthetic tree + methodology; comparison numbers still a report).
if [[ -f "perf/bench.sh" ]] \
  && [[ -f "perf/regenerate.py" ]]; then
  ok
else
  bad "perf bench/regenerate harness missing (#86)"
fi

# #18/#19 planning-entry evidence stays pinned: audit family
# selection plus update selector resolution (update executes live).
if grep -q -F -e 'family selection' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'dx_update::selector::resolve' cli/cli/src/exec/update.rs; then
  ok
else
  bad "audit/update lost their family-selection/selector-planning entry points"
fi

# #19 aggregate verdict stays unit-pinned in the update reporter
# (per-set reporting delivered with backends).
if grep -q -F -e 'overall_failure' cli/update/src/report.rs; then
  ok
else
  bad "update reporter lost its overall_failure aggregate verdict (#19)"
fi

# #86 comparison-test stays present (numbers open).
if [[ -f "perf/rules_lint_comparison_test.sh" ]]; then
  ok
else
  bad "perf lost its comparison-test harness (#86)"
fi

# #18 audit family surface stays pinned: frozen security/license
# spellings plus the exception/license-policy/secrets plus advisory/vuln/spdx/backend/locks
# modules with live execution.
if grep -q -F -e 'SECURITY_FAMILY' cli/audit/src/lib.rs \
  && grep -q -F -e 'LICENSE_FAMILY' cli/audit/src/lib.rs \
  && [[ -f "cli/audit/src/exception.rs" ]] \
  && [[ -f "cli/audit/src/license_policy.rs" ]] \
  && [[ -f "cli/audit/src/secrets.rs" ]] \
  && [[ -f "cli/audit/src/advisory.rs" ]] \
  && [[ -f "cli/audit/src/vuln.rs" ]] \
  && [[ -f "cli/audit/src/spdx.rs" ]] \
  && [[ -f "cli/audit/src/backend.rs" ]] \
  && [[ -f "cli/audit/src/locks.rs" ]]; then
  ok
else
  bad "audit crate lost its family constants or policy modules (#18)"
fi

# #19 update planning API stays pinned: selector planning plus the
# explicit confirmation/mutation markers (resolver backends delivered).
if grep -q -F -e 'requires_confirmation' cli/update/src/lib.rs \
  && grep -q -F -e 'is_mutating' cli/update/src/lib.rs; then
  ok
else
  bad "update crate lost its confirmation/mutation planning API (#19)"
fi

# Audit live execution delivered in #18 (no green-claim gate needed).

echo "execution corpus perf guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
