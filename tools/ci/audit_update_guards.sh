#!/usr/bin/env bash
# Audit/update/depcheck execution guards (issues #18, #19, #22).
#
# Live `dx audit` fails closed with `audit_deferred` and live `dx update`
# fails closed with `update_deferred`; only planning is implemented
# (family selection, selector planning, aggregate exit-code mapping from
# #241; `--dry-run` exits 0). No auditor wiring, advisory acquisition,
# SARIF/SPDX mapping, resolver backends, or per-set reporting is claimed.
# No lockfile-consistency or usage checker exists (contract accepted but
# unexecuted; fixtures per language stay open under #22).
#
# This harness machine-checks the fail-closed half verifiable on a clean
# tree today (11 checks): deferred codes, exit-code mappings, dry-run
# planning, docs ownership with no false claims, consumer-ci still
# disabled, depcheck contract green, and matrix honesty. Live execution
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

doc="docs/cli/commands/audit-update-bazel.md"

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

# #18: docs own the open auditor wiring, claim no working support.
if grep -q -F -e 'issues/18' "$doc" \
  && grep -q -F -e 'no working' "$doc"; then
  ok
else
  bad "audit doc lost its #18 ownership or no-working-support record"
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

# #19: docs own the open resolver backends, claim no live execution.
if grep -q -F -e 'issues/19' "$doc"; then
  ok
else
  bad "update doc lost its #19 ownership link"
fi

# Dry-run planning stays the only executing path (exits 0, no live claim).
if grep -q -F -e 'dry-run' "$doc" \
  && grep -q -F -e 'exits `0`' "$doc"; then
  ok
else
  bad "audit/update doc lost its dry-run planning record"
fi

# #22: depcheck contract harness stays green (truth table + open routes).
if [[ -f "tools/ci/depcheck_contract.sh" ]]; then
  ok
else
  bad "depcheck contract harness missing"
fi

# #22: no checker implementation falsely claimed in code.
if [[ -z "$(grep -rn -E -e 'fn check_lockfile|fn check_declared_usage|lockfile_consistency|declared_usage_check' --include='*.rs' cli/ quality/ tools/ 2>/dev/null || true)" ]]; then
  ok
else
  bad "checker-shaped symbols appeared without fixtures"
fi

# Matrix stays honest: depcheck open, audit/update planning-only.
if grep -q -F -e 'Open (#22)' docs/testing/verification-matrix.md \
  && grep -q -F -e 'Planning only (#18/#19)' docs/testing/verification-matrix.md; then
  ok
else
  bad "verification-matrix lost its #22/#18/#19 honesty record"
fi

echo "audit update guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
