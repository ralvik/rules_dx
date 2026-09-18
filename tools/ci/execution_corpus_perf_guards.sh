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
# today (12 checks): deferred codes, exit mappings, doc ownership, prior
# harnesses green, corpus single-name rule + Gazelle ownership, perf
# report-not-gate shape, and matrix honesty. Live execution, per-type
# generation, and comparison numbers stay open under their issues.
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

# #86 perf report-not-gate shape: workflow + comparator present.
if [[ -f ".github/workflows/perf.yml" ]] \
  && [[ -f "perf/compare.py" ]]; then
  ok
else
  bad "perf harness missing (perf.yml workflow or compare.py)"
fi

# #86 prior slice stays green.
if [[ -f "tools/ci/verify_perf_corpus.sh" ]]; then
  ok
else
  bad "verify_perf_corpus harness missing"
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
