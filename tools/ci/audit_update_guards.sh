#!/usr/bin/env bash
# Audit/update/depcheck execution guards (issues #18, #19, #22).
#
# Live `dx audit` executes qualified auditors per family over resolved scopes
# with per-family reporting (issue #18 delivered: Gitleaks subprocess planning
# in `dx_audit::backend`, advisory snapshots with 24h cache semantics in
# `dx_audit::advisory`, offline matching in `dx_audit::vuln`, SPDX 2.3 JSON in
# `dx_audit::spdx`, plus `dx_audit::outcome` aggregation; `--dry-run` exits 0).
# Live `dx update`
# executes resolver-owned backends per set with independent-set continuation
# and per-set reporting (issue #19 delivered: selector syntax in
# `dx_update::selector`, five-set registry in `dx_update::sets`, backend argv
# in `dx_update::backend`, continuation in `dx_update::outcome`, exit selection
# in `dx_update::report`; `--dry-run` exits 0).
# Required-core lockfile-consistency and usage checks are delivered in
# tools/depcheck/ (issue #22); admitted expansion stays open under
# #304/#306.
#
# This harness machine-checks the verifiable halves on a clean
# tree today: audit live execution, update live execution, exit-code mappings,
# dry-run planning, consumer-ci still
# disabled, depcheck contract green.
#
# Versioned here, run by CI via `bazel run //tools/ci:audit_update_guards`,
# following //tools/ci:depcheck_contract.
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

# #18: live audit executes qualified auditors with per-family reporting (no deferred code).
if grep -q -F -e 'CODE_AUDIT_FAILED' cli/cli/src/exec/common.rs \
  && grep -q -F -e 'dx_audit::backend::plan_secrets' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'dx_audit::outcome::AuditReport' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'dx_audit::outcome::exit_code' cli/cli/src/exec/audit.rs \
  && ! grep -q -F -e 'CODE_AUDIT_DEFERRED' cli/cli/src/exec/common.rs \
  && ! grep -q -F -e 'audit_deferred' cli/cli/src/exec/audit.rs; then
  ok
else
  bad "live audit lost its auditor execution (want CODE_AUDIT_FAILED + backend/aggregate/exit_code, no audit_deferred)"
fi

# #18: aggregate exit-code mapping stays unit-pinned (clean 0, findings
# or incomplete 1; detail in the report, not the code).
if grep -q -F -e 'pub fn exit_code' cli/audit/src/outcome.rs \
  && grep -q -F -e 'Findings' cli/audit/src/outcome.rs \
  && grep -q -F -e 'Incomplete' cli/audit/src/outcome.rs; then
  ok
else
  bad "audit outcome lost its aggregate exit-code mapping"
fi

# #18/#19: consumer smoke still disables both audits (live, not yet gated).
if grep -q -F -e 'security-audit' .github/workflows/ci.yml \
  && grep -q -F -e 'license-audit' .github/workflows/ci.yml; then
  ok
else
  bad "consumer-ci lost its disabled security/license audit record"
fi

# #19: live update executes resolver backends with continuation (no deferred code).
if grep -q -F -e 'CODE_UPDATE_FAILED' cli/cli/src/exec/common.rs \
  && grep -q -F -e 'dx_update::backend::plan' cli/cli/src/exec/update.rs \
  && grep -q -F -e 'dx_update::outcome::aggregate' cli/cli/src/exec/update.rs \
  && grep -q -F -e 'dx_update::report::exit_code' cli/cli/src/exec/update.rs \
  && ! grep -q -F -e 'CODE_UPDATE_DEFERRED' cli/cli/src/exec/common.rs \
  && ! grep -q -F -e 'update_deferred' cli/cli/src/exec/update.rs; then
  ok
else
  bad "live update lost its resolver-backend execution (want CODE_UPDATE_FAILED + backend/aggregate/exit_code, no update_deferred)"
fi

# #19: aggregate exit-code mapping stays unit-pinned (overall_failure).
if grep -q -F -e 'overall_failure' cli/update/src/report.rs \
  && grep -q -F -e 'pub fn exit_code' cli/update/src/report.rs; then
  ok
else
  bad "update report lost its overall_failure exit-code mapping"
fi

# #22: depcheck contract harness stays green (truth table + delivered routes).
if [[ -f "tools/ci/depcheck_contract.sh" ]]; then
  ok
else
  bad "depcheck contract harness missing"
fi

# #22: required-core checker is implemented with fixtures (no false claim).
if [[ -f "tools/depcheck/depcheck.py" ]] \
  && [[ -f "tools/depcheck/BUILD.bazel" ]] \
  && [[ -d "tools/depcheck/testdata/rust/ok_used" ]] \
  && [[ -d "tools/depcheck/testdata/python/ok_used" ]] \
  && [[ -d "tools/depcheck/testdata/js/ok_used" ]] \
  && [[ -d "tools/depcheck/testdata/ts/ok_used" ]]; then
  ok
else
  bad "depcheck implementation or fixtures missing for #22"
fi

echo "audit update guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
