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
# today (36 checks): deferred codes, fail-closed unit pins, dry-run
# planning paths + doc record, audit families + policy modules +
# SARIF/SPDX mapping record, selector planning + update API +
# per-set report record + aggregate verdict pins, exit mappings, doc
# ownership + auditor/resolver records, depcheck truth-table +
# category + missing-reasons + offline contract, prior harnesses
# green, corpus single-name rule + Gazelle ownership + generate doc +
# contract + generate --check wiring + carve-out record, perf
# report-not-gate shape + v2.8.0 fairness pin + Bazel-version pin +
# report honesty + bench harness + comparison-test record, and matrix
# honesty. Live execution, per-type generation, and comparison
# numbers stay open under their issues.
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

# #18 secrets auditor wiring stays recorded as Gitleaks standalone
# artifact (SARIF output, redaction, findings-vs-error split open).
if grep -q -F -e 'Gitleaks' "$doc"; then
  ok
else
  bad "audit/update doc lost its Gitleaks secrets-auditor record (#18)"
fi

# #19 resolver ownership stays recorded (resolver-owned backends per
# set, no dx lockfile or private resolver; live backends still open).
if grep -q -F -e 'resolver' "$doc"; then
  ok
else
  bad "audit/update doc lost its resolver-ownership record (#19)"
fi

# #22 offline route contract stays explicit (network denied, local
# matching, no lockfile/inventory upload; fixtures still open).
if grep -q -F -e 'offline' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost its offline-route contract record (#22)"
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
# selection plus update selector planning (execution still deferred).
if grep -q -F -e 'family selection' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'UpdateSelection::Selected' cli/cli/src/exec/update.rs; then
  ok
else
  bad "audit/update lost their family-selection/selector-planning entry points"
fi

# #19 aggregate verdict stays unit-pinned in the update reporter
# (per-set reporting still open).
if grep -q -F -e 'overall_failure' cli/update/src/report.rs; then
  ok
else
  bad "update reporter lost its overall_failure aggregate verdict (#19)"
fi

# #15 generate doc owns the canonical Gazelle workflow (per-type
# emission still open).
if grep -q -F -e 'Gazelle' docs/cli/commands/generate.md; then
  ok
else
  bad "generate doc lost its canonical Gazelle workflow record (#15)"
fi

# #86 comparison-test record stays present: deterministic synthetic
# tree + fairness-pinned skeleton, proven by its test (numbers open).
if [[ -f "perf/rules_lint_comparison_test.sh" ]] \
  && grep -q -F -e 'deterministic synthetic tree' docs/tools/rules_lint-comparison.md; then
  ok
else
  bad "perf lost its comparison-test / synthetic-tree record (#86)"
fi

# #18 audit family surface stays pinned: frozen security/license
# spellings plus the exception/license-policy/secrets policy modules
# (tool wiring and acquisition still open).
if grep -q -F -e 'SECURITY_FAMILY' cli/audit/src/lib.rs \
  && grep -q -F -e 'LICENSE_FAMILY' cli/audit/src/lib.rs \
  && [[ -f "cli/audit/src/exception.rs" ]] \
  && [[ -f "cli/audit/src/license_policy.rs" ]] \
  && [[ -f "cli/audit/src/secrets.rs" ]]; then
  ok
else
  bad "audit crate lost its family constants or policy modules (#18)"
fi

# #19 update planning API stays pinned: selector planning plus the
# explicit confirmation/mutation markers (resolver backends open).
if grep -q -F -e 'requires_confirmation' cli/update/src/lib.rs \
  && grep -q -F -e 'is_mutating' cli/update/src/lib.rs; then
  ok
else
  bad "update crate lost its confirmation/mutation planning API (#19)"
fi

# #22 depcheck verdict contract stays explicit: stale lockfiles fail
# consistency while usage failures need category errors (fixtures open).
if grep -q -F -e 'A stale lockfile must fail consistency' docs/quality/quality-testing.md \
  && grep -q -F -e 'must fail the usage test with a category error' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost its depcheck verdict contract (#22)"
fi

# #15 generate command contract stays owned: contract registry plus
# the CLI boundary around the Gazelle workflow (emission still open).
if grep -q -F -e '## Generation Contracts' docs/cli/commands/generate.md \
  && grep -q -F -e '## CLI Boundary' docs/cli/commands/generate.md; then
  ok
else
  bad "generate doc lost its contracts/CLI-boundary record (#15)"
fi

# #18 report-mapping record stays explicit: SARIF severity/report
# mappings plus the SPDX 2.3 JSON shape (tool wiring and advisory
# acquisition still open).
if grep -q -F -e 'SARIF' "$doc" \
  && grep -q -F -e 'SPDX' "$doc"; then
  ok
else
  bad "audit/update doc lost its SARIF/SPDX report-mapping record (#18)"
fi

# #19 per-set report record stays explicit: one aggregate exit code
# with a per-set report, never a per-set code (backends still open).
if grep -q -F -e 'per-set report' "$doc"; then
  ok
else
  bad "audit/update doc lost its per-set report record (#19)"
fi

# #22 exception-reason contract stays explicit: reason-less usage
# exceptions fail validation (fixtures still open).
if grep -q -F -e 'Missing reasons must fail validation' docs/quality/quality-testing.md; then
  ok
else
  bad "quality-testing.md lost its missing-reasons validation contract (#22)"
fi

# #86 Bazel-version fairness pin stays explicit alongside the
# rules_lint pin: same Bazel version from `.bazelversion` on the same
# machine class (comparison numbers still a report).
if grep -q -F -e '.bazelversion' docs/tools/rules_lint-comparison.md \
  && grep -q -F -e '9.2.0' docs/tools/rules_lint-comparison.md; then
  ok
else
  bad "rules_lint methodology lost its Bazel-version fairness pin (#86)"
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
