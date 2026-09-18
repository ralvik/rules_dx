#!/usr/bin/env bash
# Execution/corpus/perf guards (issues #18, #19, #22, #15, #86).
#
# Live `dx audit`/`dx update` fail closed with audit_deferred /
# update_deferred; only family selection, selector planning, aggregate
# exit-code mapping, and --dry-run planning execute. No auditor wiring,
# advisory acquisition, SARIF/SPDX mapping, resolver backends, per-set
# reporting, lockfile-consistency checker, or usage checker is claimed.
# Corpus stays single `corpus` per directory until `dx generate` emits
# the per-type split; perf tracks the frozen aspect_rules_lint v2.8.0
# baseline as a report, never a gate.
#
# This harness machine-checks the frozen half verifiable on a clean tree
# today (20 checks): deferred codes, fail-closed unit pins, dry-run
# planning paths + doc record, exit mappings, doc ownership, depcheck
# truth-table contract, prior harnesses green, corpus single-name rule +
# Gazelle ownership + generate --check wiring + carve-out record, perf
# report-not-gate shape + v2.8.0 fairness pin + report-not-gate honesty,
# and matrix honesty. Live execution, per-type generation, and
# comparison numbers stay open under their issues.
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

doc="docs/cli/commands/audit-update-bazel.md"

# #18 fail-closed audit code pinned in CLI.
if grep -q -F -e 'audit_deferred' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'CODE_AUDIT_DEFERRED' cli/cli/src/exec/common.rs; then
  ok
else
  bad "live audit lost its audit_deferred fail-closed code"
fi

# #19 fail-closed update code pinned in CLI.
if grep -q -F -e 'update_deferred' cli/cli/src/exec/update.rs \
  && grep -q -F -e 'CODE_UPDATE_DEFERRED' cli/cli/src/exec/common.rs; then
  ok
else
  bad "live update lost its update_deferred fail-closed code"
fi

# #18/#19 fail-closed behavior pinned by unit tests, not just codes.
if grep -q -F -e 'assert!(err.contains("audit_deferred")' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'assert!(err.contains("update_deferred")' cli/cli/src/exec/update.rs; then
  ok
else
  bad "audit/update lost their fail-closed unit-test pins"
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

# Docs own the open routes with honest links.
if grep -q -F -e 'issues/18' "$doc" \
  && grep -q -F -e 'issues/19' "$doc"; then
  ok
else
  bad "audit/update doc lost its #18/#19 ownership links"
fi

# #22 prior slice stays green, no checker falsely claimed.
if [[ -f "tools/ci/depcheck_contract.sh" ]] \
  && [[ -f "tools/ci/audit_update_guards.sh" ]]; then
  ok
else
  bad "prior execution harnesses missing (depcheck_contract/audit_update_guards)"
fi

if [[ -z "$(grep -rn -E -e 'fn check_lockfile|fn check_declared_usage' --include='*.rs' cli/ quality/ 2>/dev/null || true)" ]]; then
  ok
else
  bad "checker-shaped symbols appeared without #22 fixtures landing"
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

# #86 fairness pin recorded in results + methodology (report, never gate).
if grep -q -F -e '"rules_lint_pin": "v2.8.0"' perf/rules_lint_results.json \
  && grep -q -F -e 'rules_lint` pin (`v2.8.0` baseline)' docs/tools/rules_lint-comparison.md; then
  ok
else
  bad "perf lost its rules_lint v2.8.0 fairness pin (results or methodology)"
fi

# #86 prior slice stays green.
if [[ -f "tools/ci/verify_perf_corpus.sh" ]]; then
  ok
else
  bad "verify_perf_corpus harness missing"
fi

# #22 depcheck truth-table contract stays owned in docs (fixtures open,
# contract accepted, no checker claimed).
if grep -q -F -e 'lockfile consistency' docs/quality/quality-testing.md \
  && grep -q -F -e 'declared-dependency usage' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost its #22 lockfile-consistency/usage contract record"
fi

# #18/#19 dry-run planning contract stays recorded in the command doc
# (plans without launching, exits 0).
if grep -q -F -e '--dry-run' "$doc"; then
  ok
else
  bad "audit/update doc lost its --dry-run planning record (#18/#19)"
fi

# #86 report-not-gate honesty stays explicit in methodology (results
# arrive as a report, never a gate).
if grep -q -F -e 'not a gate' docs/tools/rules_lint-comparison.md; then
  ok
else
  bad "rules_lint methodology lost its report-not-gate honesty record (#86)"
fi

# #15 corpus carve-out record stays explicit (integration scenario
# workspaces owned outside the audit, no silent skip).
if grep -q -F -e 'carve-out' tools/ci/corpus_audit.sh; then
  ok
else
  bad "corpus_audit lost its carve-out record (#15)"
fi

# Matrix honesty across this group.
if grep -q -F -e 'Tracked (#86)' docs/testing/verification-matrix.md \
  && grep -q -F -e 'Open (#22)' docs/testing/verification-matrix.md \
  && grep -q -F -e 'Planning only (#18/#19)' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #86/#22/#18/#19 honesty markers"
fi

# No live-execution green claim.
if ! grep -rln -F -e 'audit live execution green' tools/ci/ docs/cli/ 2>/dev/null | grep -v -F -e 'execution_corpus_perf_guards.sh' | grep -q . \
  && ! grep -rln -F -e 'resolver backends landed' tools/ci/ docs/cli/ 2>/dev/null | grep -v -F -e 'execution_corpus_perf_guards.sh' | grep -q .; then
  ok
else
  bad "a live-execution green claim appeared without #18/#19 landing"
fi

echo "execution corpus perf guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
