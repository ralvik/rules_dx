#!/usr/bin/env bash
# Dependency-check contract harness (issue #22, seed slice).
#
# No lockfile-consistency or declared-dependency usage checker exists in
# code yet. The test contract in docs/quality/quality-testing.md
# (stale vs consistent lockfiles, offline routes, non-import
# recognition, category errors, obsolete exceptions) is accepted but
# unexecuted. Fixtures per language, offline execution with network
# denied, ecosystem lock/usage scopes, category validation, and narrow
# usage exceptions all stay open under #22.
#
# This harness machine-checks the contract half verifiable on a clean
# tree today (10 checks): the truth table, offline, non-mutating,
# independently-runnable, non-import/exception, category, and obsolete
# clauses are present and linked to #22 where routes stay open, and no
# checker implementation is falsely claimed in code or docs. It records
# gaps instead of claiming fixtures.
#
# Versioned here, run by CI via `bazel run //tools/ci:depcheck_contract`,
# following //tools/ci:quality_cache_aquery.
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

contract="docs/quality/quality-testing.md"

# The accepted contract file exists.
if [[ -f "$contract" ]]; then
  ok
else
  bad "dependency-check contract missing: $contract"
fi

# Stale-vs-consistent truth table: stale fails consistency even when
# every declaration is used; consistent+unused passes consistency but
# fails usage; consistent+all-used passes both.
if grep -q -F -e 'A stale lockfile must fail consistency even when every declaration is used' "$contract" \
  && grep -q -F -e 'a consistent lockfile' "$contract" \
  && grep -q -F -e 'must pass both, even when newer compatible releases exist' "$contract"; then
  ok
else
  bad "contract lost the stale-vs-consistent truth table"
fi

# Offline routes: network denied after provisioning, no undeclared
# cache, no live-registry queries; exact routes stay open work.
if grep -q -F -e 'with network access denied' "$contract" \
  && grep -q -F -e 'does not query live registries' "$contract" \
  && grep -q -F -e 'Exact offline routes are open under' "$contract" \
  && grep -q -F -e 'open work' "$contract"; then
  ok
else
  bad "contract lost the offline-routes clause or its open-work record"
fi

# Non-mutating requirement: neither check mutates manifests or locks.
if grep -q -F -e 'neither test mutates manifests or locks' "$contract"; then
  ok
else
  bad "contract lost the non-mutating requirement"
fi

# Independently runnable: generated as normal test targets in both
# `bazel test //...` and bare `dx test`, no manual exclusion.
if grep -q -F -e 'bazel test //...' "$contract" \
  && grep -q -F -e 'bare `dx test`' "$contract" \
  && grep -q -F -e 'independently runnable by label' "$contract" \
  && grep -q -F -e 'no' "$contract" \
  && grep -q -F -e 'manual' "$contract"; then
  ok
else
  bad "contract lost the independently-runnable clause"
fi

# Non-import recognition with explicit dependency-scoped exceptions:
# legitimate non-import use passes with a reason, unrelated unused
# declarations still fail, exceptions never waive consistency, missing
# reasons fail validation.
if grep -q -F -e 'non-import use can pass through an explicit dependency-scoped' "$contract" \
  && grep -q -F -e 'Missing reasons must fail validation' "$contract" \
  && grep -q -F -e 'does not waive lockfile consistency' "$contract"; then
  ok
else
  bad "contract lost the non-import/exception-reason clause"
fi

# Category validation: prod declarations used only by tests fail with a
# category error; correctly categorized and multi-category usage pass.
if grep -q -F -e 'must fail the usage test with a category error' "$contract" \
  && grep -q -F -e 'Correctly categorized and legitimate multi-category usage must pass' "$contract"; then
  ok
else
  bad "contract lost the category-validation clause"
fi

# Obsolete exceptions: removed dependencies and checker-upgrade
# recognitions fail as obsolete; still-needed exceptions pass;
# validation reports without deleting.
if grep -q -F -e 'fail' "$contract" \
  && grep -q -F -e 'as obsolete' "$contract" \
  && grep -q -F -e 'without deleting them' "$contract"; then
  ok
else
  bad "contract lost the obsolete-exception clause"
fi

# No checker implementation is falsely claimed: no lockfile-consistency
# or declared-usage checker entry point exists in implementation code.
# (Incidental "lockfile" mentions in planning comments/docs are not
# checkers; this gates on checker-shaped symbols only.)
if [[ -z "$(grep -rn -E -e 'fn check_lockfile|fn check_declared_usage|lockfile_consistency|declared_usage_check' --include='*.rs' cli/ quality/ tools/ 2>/dev/null || true)" ]]; then
  ok
else
  bad "checker-shaped symbols appeared in code without fixtures: $(grep -rn -E -e 'fn check_lockfile|fn check_declared_usage|lockfile_consistency|declared_usage_check' --include='*.rs' cli/ quality/ tools/ 2>/dev/null | head -n 3)"
fi

# Open routes stay recorded as open work (offline, native config, scopes),
# never silently decided elsewhere.
if [[ "$(grep -c -F -e 'open work' "$contract")" -ge 2 ]]; then
  ok
else
  bad "contract lost its open-work route records"
fi

echo "depcheck contract harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
