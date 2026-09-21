#!/usr/bin/env bash
# Dependency-check contract harness (delivered; opens under).
#
# Required-core (Rust/Python/JavaScript/TypeScript) plus admitted (Go,
# Java/Kotlin/Scala, C#/F#, C/C++) lockfile-consistency and
# declared-dependency usage fixtures are implemented in
# tools/depcheck/ (hermetic checker plus per-language truth-table,
# transitive/shared, exception, obsolete, platform, and category
# fixtures as normal test targets). Remaining admitted quality-adapter
# implementation stays owned by ADR 0019 (qualified under
# ; foundation mappings under -.
#
# This harness machine-checks the delivered half on a clean tree:
# the truth table, offline, non-mutating, independently-runnable,
# non-import/exception, category, and obsolete clauses are present in
# docs/quality/quality-testing.md and recorded as accepted (opens under),
# the checker exists with no network imports, per-language fixtures and
# targets exist with no `manual` exclusion, and docs describe only what
# runs. Run by CI via `bazel run //tools/ci:depcheck_contract`,
# following //tools/ci:quality_cache_aquery.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

contract="docs/quality/quality-testing.md"
checker="tools/depcheck/src/lib.rs"
binary="tools/depcheck/src/main.rs"
build="tools/depcheck/BUILD.bazel"

if [[ -f "$contract" ]]; then
  ok
else
  bad "dependency-check contract missing: $contract"
fi

# Truth table stays in the contract.
if grep -q -F -e 'A stale lockfile must fail consistency even when every declaration is used' "$contract" &&
  grep -q -F -e 'a consistent lockfile' "$contract" &&
  grep -q -F -e 'must pass both, even when newer compatible releases exist' "$contract"; then
  ok
else
  bad "contract lost the stale-vs-consistent truth table"
fi

# Offline routes qualified (no open-work placeholder).
if grep -q -F -e 'with network access denied' "$contract" &&
  grep -q -F -e 'does not query live registries' "$contract" &&
  grep -q -F -e 'Accepted (issue #22' "$contract" &&
  grep -q -F -e 'bazel test //tools/depcheck/...' "$contract"; then
  ok
else
  bad "contract lost the qualified offline-routes record for #22"
fi

# Non-mutating requirement.
if grep -q -F -e 'neither test mutates manifests or locks' "$contract"; then
  ok
else
  bad "contract lost the non-mutating requirement"
fi

# Independently runnable in both bazel test and bare dx test.
if grep -q -F -e 'bazel test //...' "$contract" &&
  grep -q -F -e 'bare `dx test`' "$contract" &&
  grep -q -F -e 'independently runnable by label' "$contract" &&
  grep -q -F -e 'manual' "$contract"; then
  ok
else
  bad "contract lost the independently-runnable clause"
fi

# Non-import/exception clause.
if grep -q -F -e 'non-import use can pass through an explicit dependency-scoped' "$contract" &&
  grep -q -F -e 'Missing reasons must fail validation' "$contract" &&
  grep -q -F -e 'does not waive lockfile consistency' "$contract"; then
  ok
else
  bad "contract lost the non-import/exception-reason clause"
fi

# Category validation clause.
if grep -q -F -e 'must fail the usage test with a category error' "$contract" &&
  grep -q -F -e 'Correctly categorized and legitimate multi-category usage must pass' "$contract"; then
  ok
else
  bad "contract lost the category-validation clause"
fi

# Obsolete-exception clause.
if grep -q -F -e 'as obsolete' "$contract" &&
  grep -q -F -e 'without deleting them' "$contract"; then
  ok
else
  bad "contract lost the obsolete-exception clause"
fi

# Checker exists as Rust, hermetic (no network/process deps), non-mutating.
if [[ -f "$checker" ]] && [[ -f "$binary" ]] &&
  ! grep -rn -E -e 'reqwest|hyper|tokio::net|std::net::Tcp' "$checker" "$binary" >/dev/null 2>&1 &&
  ! grep -rn -E -e 'std::process::Command|tokio::process' "$checker" >/dev/null 2>&1; then
  ok
else
  bad "checker missing or not hermetic: $checker plus $binary"
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

# Portable rust_test targets exist, independently runnable, no manual,
# no Linux-only pins (Rust runs on all platforms).
if grep -q -F -e 'name = "depcheck_test"' "$build" &&
  grep -q -F -e 'name = "depcheck"' "$build" &&
  grep -q -F -e 'rust_binary(' "$build" &&
  grep -q -F -e 'rust_test(' "$build" &&
  ! grep -q -F -e '"manual"' "$build" &&
  ! grep -q -F -e 'target_compatible_with' "$build"; then
  ok
else
  bad "depcheck Rust targets missing, manual, or carry Linux-only pins"
fi

# Docs describe only what runs: required-core plus admitted accepted for
# (opens under), qualified adapter work under plus foundation under -.
if grep -q -F -e 'Accepted (issue #22' "$contract" &&
  grep -q -F -e 'issue #307' "$contract" &&
  grep -q -F -e '#476-#484' "$contract"; then
  ok
else
  bad "contract lost its accepted-vs-open record for #22 plus #510"
fi

dx_test_summary "depcheck contract harness"
