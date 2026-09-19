#!/usr/bin/env bash
# Dependency-check contract harness (issues #22, #306, delivered).
#
# Required-core (Rust/Python/JavaScript/TypeScript) plus admitted (Go,
# Java/Kotlin/Scala, C#/F#, C/C++) lockfile-consistency and
# declared-dependency usage fixtures are implemented in
# tools/depcheck/ (hermetic checker plus per-language truth-table,
# transitive/shared, exception, obsolete, platform, and category
# fixtures as normal test targets). Remaining admitted quality-adapter
# implementation stays owned by O32/O31 plus ADR 0019 (qualified under
# issue #307); foundation mappings under #304.
#
# This harness machine-checks the delivered half on a clean tree:
# the truth table, offline, non-mutating, independently-runnable,
# non-import/exception, category, and obsolete clauses are present in
# docs/quality/quality-testing.md and recorded as accepted for #22/#306,
# the checker exists with no network imports, per-language fixtures and
# targets exist with no `manual` exclusion, and docs describe only what
# runs. Run by CI via `bazel run //tools/ci:depcheck_contract`,
# following //tools/ci:quality_cache_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workspace="$(dx_workspace_root)"
cd "$workspace"

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

contract="docs/quality/quality-testing.md"
checker="tools/depcheck/depcheck.py"
build="tools/depcheck/BUILD.bazel"

if [[ -f "$contract" ]]; then
  ok
else
  bad "dependency-check contract missing: $contract"
fi

# Truth table stays in the contract.
if grep -q -F -e 'A stale lockfile must fail consistency even when every declaration is used' "$contract" \
  && grep -q -F -e 'a consistent lockfile' "$contract" \
  && grep -q -F -e 'must pass both, even when newer compatible releases exist' "$contract"; then
  ok
else
  bad "contract lost the stale-vs-consistent truth table"
fi

# Offline routes qualified for #22/#306 (no open-work placeholder).
if grep -q -F -e 'with network access denied' "$contract" \
  && grep -q -F -e 'does not query live registries' "$contract" \
  && grep -q -F -e 'Accepted (issues #22, #306)' "$contract" \
  && grep -q -F -e 'bazel test //tools/depcheck/...' "$contract"; then
  ok
else
  bad "contract lost the qualified offline-routes record for #22/#306"
fi

# Non-mutating requirement.
if grep -q -F -e 'neither test mutates manifests or locks' "$contract"; then
  ok
else
  bad "contract lost the non-mutating requirement"
fi

# Independently runnable in both bazel test and bare dx test.
if grep -q -F -e 'bazel test //...' "$contract" \
  && grep -q -F -e 'bare `dx test`' "$contract" \
  && grep -q -F -e 'independently runnable by label' "$contract" \
  && grep -q -F -e 'manual' "$contract"; then
  ok
else
  bad "contract lost the independently-runnable clause"
fi

# Non-import/exception clause.
if grep -q -F -e 'non-import use can pass through an explicit dependency-scoped' "$contract" \
  && grep -q -F -e 'Missing reasons must fail validation' "$contract" \
  && grep -q -F -e 'does not waive lockfile consistency' "$contract"; then
  ok
else
  bad "contract lost the non-import/exception-reason clause"
fi

# Category validation clause.
if grep -q -F -e 'must fail the usage test with a category error' "$contract" \
  && grep -q -F -e 'Correctly categorized and legitimate multi-category usage must pass' "$contract"; then
  ok
else
  bad "contract lost the category-validation clause"
fi

# Obsolete-exception clause.
if grep -q -F -e 'as obsolete' "$contract" \
  && grep -q -F -e 'without deleting them' "$contract"; then
  ok
else
  bad "contract lost the obsolete-exception clause"
fi

# Checker exists, hermetic (no network imports), non-mutating (reads only).
if [[ -f "$checker" ]] \
  && ! grep -rn -E -e 'import urllib|import socket|import http|import requests|from urllib|from socket' "$checker" >/dev/null 2>&1 \
  && ! grep -rn -E -e 'subprocess|os\.system|os\.exec' "$checker" >/dev/null 2>&1; then
  ok
else
  bad "checker missing or not hermetic: $checker"
fi

# Per-language fixtures exist (truth table + edge cases for each core+admitted lang).
fixtures_ok=1
for lang in rust python js ts go java kotlin scala csharp fsharp cc; do
  for case in ok_used stale unused transitive_shared exception obsolete platform_optional platform_optional_unused category category_ok; do
    if [[ ! -d "tools/depcheck/testdata/$lang/$case" ]]; then
      fixtures_ok=0
    fi
  done
done
if [[ "$fixtures_ok" == "1" ]]; then
  ok
else
  bad "per-language depcheck fixtures missing under tools/depcheck/testdata/"
fi

# Twenty-two normal test targets exist, independently runnable, no manual,
# Linux-only per the shell contract.
targets_ok=1
for t in rust_consistency_test rust_usage_test python_consistency_test python_usage_test js_consistency_test js_usage_test ts_consistency_test ts_usage_test go_consistency_test go_usage_test java_consistency_test java_usage_test kotlin_consistency_test kotlin_usage_test scala_consistency_test scala_usage_test csharp_consistency_test csharp_usage_test fsharp_consistency_test fsharp_usage_test cc_consistency_test cc_usage_test; do
  if ! grep -q -F -e "name = \"$t\"" "$build"; then
    targets_ok=0
  fi
done
if [[ "$targets_ok" == "1" ]] \
  && ! grep -A8 -e 'depcheck' "$build" | grep -q -F -e '"manual"'; then
  ok
else
  bad "depcheck test targets missing or carry manual exclusion"
fi
if [[ "$(grep -c -F -e 'target_compatible_with = ["@platforms//os:linux"]' "$build")" -ge 22 ]]; then
  ok
else
  bad "depcheck sh_tests missing Linux-only labels"
fi

# Docs describe only what runs: required-core plus admitted accepted for
# #22/#306, qualified adapter work under #307 plus foundation under #304.
if grep -q -F -e 'Accepted (issues #22, #306)' "$contract" \
  && grep -q -F -e 'issue #307' "$contract" \
  && grep -q -F -e 'issue #304' "$contract"; then
  ok
else
  bad "contract lost its accepted-vs-open record for #22/#306"
fi

echo "depcheck contract harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
