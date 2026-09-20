#!/usr/bin/env bash
# Python source-audit split qualification harness.
#
# Splits Python source-audit tooling out of the quality family taxonomy
# (stays taxonomy-only) with fixture evidence:
# - disposition: no selected audit tool in v1; curated audit stays empty
#   with explicit disablement for the python family, Bandit excluded from
#   v1 by ADR 0019, secrets ride the separate Gitleaks family;
# - unaffected: python lint stays pydoclint plus ruff, format stays ruff,
#   typecheck stays ty, flake8 plus pylint stay baseline opt-ins;
# - adapters: no audit adapter claims python plus python_stub;
# - scope: per-language source audit distinct from ecosystem dx audit plus
#   dx update live execution delivered repo-wide;
# - rejected: leaving under taxonomy rejected with mismatched scope;
# - fixtures: `python/tests/fixtures/python_audit/` (`pins.bzl` plus
#   `python_audit.expected`) pins disposition plus rejected plus honesty;
# - scope: quality-only; no workflow change.
#   Seed only: no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:python_audit_qualification`,
# following //tools/ci:quality_taxonomy_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="python/tests/fixtures/python_audit/pins.bzl"
pins_build="python/tests/fixtures/python_audit/BUILD.bazel"
expected="python/tests/fixtures/python_audit/python_audit.expected"
adapters="quality/adapters.bzl"
curated="quality/curated_defaults.bzl"
support="docs/product/support-matrix.md"
baseline="docs/tools/tool-baseline.md"
action_doc="docs/quality/action-model.md"
python_adr="docs/decisions/0010-python-foundation.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "python audit fixture missing (want $pins plus $pins_build plus python_audit.expected)"
fi

# Pins record the empty audit plus Bandit plus secrets split.
if grep -q -F -e 'curated audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'python family audit stays empty with explicit disablement' "$pins" &&
  grep -q -F -e 'Bandit excluded from v1 by ADR 0019' "$pins" &&
  grep -q -F -e 'secrets family rides Gitleaks detect with redact plus SARIF' "$pins"; then
  ok
else
  bad "pins.bzl lost its empty audit plus Bandit plus secrets split under issue #613"
fi

# Pins record the unaffected quality plus no-adapter plus scope plus rejected plus owned gaps.
if grep -q -F -e 'python family lint pydoclint plus ruff' "$pins" &&
  grep -q -F -e 'python family format ruff' "$pins" &&
  grep -q -F -e 'python family typecheck ty' "$pins" &&
  grep -q -F -e 'flake8 plus pylint stay baseline opt-ins' "$pins" &&
  grep -q -F -e 'no audit adapter claims python plus python_stub' "$pins" &&
  grep -q -F -e 'per-language source audit distinct from ecosystem dx audit plus dx update live execution' "$pins" &&
  grep -q -F -e 'leaving under taxonomy rejected with mismatched scope' "$pins" &&
  grep -q -F -e 'future tool selection stays owned under issue #613' "$pins" &&
  grep -q -F -e 'platform plus consumer plus release evidence stays owned gap' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #613' "$pins"; then
  ok
else
  bad "pins.bzl lost its lint plus format plus typecheck plus no-adapter plus scope plus rejected plus owned gaps under issue #613"
fi

# Curated defaults keep python audit empty with lint plus format plus typecheck.
if grep -q -F -e '"python": {' "$curated" &&
  grep -q -F -e '"audit": []' "$curated" &&
  grep -q -F -e '"python": ["ruff"]' "$curated" &&
  grep -q -F -e 'pydoclint' "$curated" &&
  grep -q -F -e '"typecheck": ["ty"]' "$curated"; then
  ok
else
  bad "curated defaults lost python audit empty plus ruff plus pydoclint plus ty"
fi

# No audit adapter claims python: no audit capability in the real registry.
if ! grep -q -F -e '"audit":' "$adapters"; then
  ok
else
  bad "adapters.bzl must carry no audit capability under issue #613 (python audit has no adapter claim)"
fi

# Support matrix owns the split Python audit record under.
if grep -q -F -e 'qualified seed-only under issue #613' "$support" &&
  grep -q -F -e 'python_audit_qualification' "$support" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$support" &&
  grep -q -F -e 'Bandit excluded' "$support" &&
  grep -q -F -e 'platform plus consumer plus release owned gap' "$support"; then
  ok
else
  bad "docs/product/support-matrix.md lost its #613 Python audit record with fixtures plus qualification"
fi

# Support matrix keeps taxonomy-only with no Python audit ownership.
if grep -q -F -e 'issue #512 stays taxonomy-only' "$support" &&
  grep -q -F -e 'qualified seed-only under issue #512' "$support" &&
  grep -q -F -e 'quality_taxonomy_qualification' "$support"; then
  ok
else
  bad "docs/product/support-matrix.md lost its #512 taxonomy-only split record"
fi

# Tool baseline owns the audit-open record with Bandit honesty.
if grep -q -F -e 'issue #613' "$baseline" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$baseline" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$baseline" &&
  grep -q -F -e 'Bandit excluded from v1' "$baseline"; then
  ok
else
  bad "docs/tools/tool-baseline.md lost its #613 audit-open record with fixtures plus qualification"
fi

# Action model owns the audit-selection record with empty audit plus Bandit.
if grep -q -F -e 'qualified seed-only under issue #613' "$action_doc" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$action_doc" &&
  grep -q -F -e 'python_audit.expected' "$action_doc" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$action_doc" &&
  grep -q -F -e 'Bandit excluded from v1' "$action_doc"; then
  ok
else
  bad "docs/quality/action-model.md lost its #613 audit-selection record with fixtures plus qualification"
fi

# Python foundation ADR owns the source-audit selection.
if grep -q -F -e 'issue #613' "$python_adr" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$python_adr" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$python_adr"; then
  ok
else
  bad "docs/decisions/0010-python-foundation.md lost its #613 source-audit selection record"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'python_audit_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #613' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:python_audit_qualification' "$verify" &&
  grep -q -F -e 'python/tests/fixtures/python_audit/pins.bzl' "$verify" &&
  grep -q -F -e '`python_audit_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #613 python audit qualified record"
fi

# Verification matrix lists the harness in dogfood-freshness.
if grep -q -F -e ':python_audit_qualification' "$verify"; then
  ok
else
  bad "verification-matrix dogfood-freshness lost :python_audit_qualification"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`python_audit_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix Green lost python_audit_qualification 16/16"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "python_audit_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:python_audit_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the python_audit_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture expected covers the split with owned gaps.
if grep -q -F -e 'qualified seed-only under issue #613' "$expected" &&
  grep -q -F -e 'issue #512 stays taxonomy-only' "$expected" &&
  grep -q -F -e 'Bandit excluded' "$expected" &&
  grep -q -F -e 'owned gap' "$expected" &&
  grep -q -F -e 'no' "$expected" &&
  grep -q -F -e 'Supported claim' "$expected"; then
  ok
else
  bad "python_audit.expected lost split coverage (want qualified plus taxonomy-only plus Bandit plus owned gap plus no Supported, issue #613)"
fi

# Live proof: the fixture build stays green on the seed host.
if bazel build //python/tests/fixtures/python_audit/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "python audit live proof failed (want //python/tests/fixtures/python_audit green, issue #613)"
fi

dx_test_summary "python audit qualification harness"
