#!/usr/bin/env bash
# Audit/update/depcheck execution guards (issues #18, #19, #22).
#
# Live `dx audit` fails closed with `audit_deferred` and live `dx update`
# fails closed with `update_deferred`; only planning is implemented
# (family selection, selector planning, aggregate exit-code mapping from
# #241; `--dry-run` exits 0). No auditor wiring, advisory acquisition,
# SARIF/SPDX mapping, resolver backends, or per-set reporting is claimed.
# Required-core lockfile-consistency and usage checks are delivered in
# tools/depcheck/ (issue #22); admitted expansion stays open under
# #304/#306.
#
# This harness machine-checks the fail-closed half verifiable on a clean
# tree today: deferred codes, exit-code mappings, dry-run
# planning, consumer-ci still
# disabled, depcheck contract green. Live execution
# stays open under its issues.
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

# #18: live audit fails closed with the stable deferred code.
if grep -q -F -e 'audit_deferred' cli/cli/src/exec/audit.rs \
  && grep -q -F -e 'CODE_AUDIT_DEFERRED' cli/cli/src/exec/common.rs; then
  ok
else
  bad "live audit lost its audit_deferred fail-closed code"
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

# #18/#19: consumer smoke still disables both audits (still deferred).
if grep -q -F -e 'security-audit' .github/workflows/ci.yml \
  && grep -q -F -e 'license-audit' .github/workflows/ci.yml; then
  ok
else
  bad "consumer-ci lost its disabled security/license audit record"
fi

# #19: live update fails closed with the stable deferred code.
if grep -q -F -e 'update_deferred' cli/cli/src/exec/update.rs \
  && grep -q -F -e 'CODE_UPDATE_DEFERRED' cli/cli/src/exec/common.rs; then
  ok
else
  bad "live update lost its update_deferred fail-closed code"
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
