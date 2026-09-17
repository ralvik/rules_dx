#!/usr/bin/env bash
# Seed-cell coverage gate harness (issue #89 item 1).
#
# Every required configuration/platform cell gates its own combined LCOV
# report: no cross-platform union, no averaged percentages, no rounding
# up. Only the seed host qualifies today (issue #5); this harness wires
# the seed cell end to end through the versioned inventory at
# tools/coverage/seed-inventory.txt and the `coverage_bin` gate CLI:
# - the real scoped `bazel coverage` report passes the real gate,
# - mutated inputs fail closed (missing report, uninventoried source,
#   undeclared eligible source, uncovered line with location),
# - the //... rate gate step is still present in CI.
#
# Versioned here, run by CI via `bazel run //tools/ci:coverage_cell`,
# following //tools/ci:target_tags.
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

check_bin="bazel-bin/tools/coverage/coverage_bin"
inventory="tools/coverage/seed-inventory.txt"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# The gate binary must exist (built by the build job / on demand).
if [[ ! -x "$check_bin" ]]; then
  bazel build --noshow_progress //tools/coverage:coverage_bin >/dev/null 2>&1
fi

# Real scoped coverage for the seed inventory scope, then the real gate.
# The gate library lives at //cli/lcov:dx_lcov since the #74 promotion;
# the thin binary shim stays at //tools/coverage:coverage_bin. Both scopes
# are covered together so the inventory (library + shim) matches Bazel.
bazel coverage --noshow_progress //cli/lcov/... //tools/coverage/... \
  --combined_report=lcov \
  --instrumentation_filter='^//(cli/lcov|tools/coverage)' >/dev/null 2>&1
bazel query 'kind("source file", deps(//cli/lcov/... + //tools/coverage/...))' 2>/dev/null \
  | grep -E '^//(cli/lcov|tools/coverage)[:/].*\.rs$' \
  | sed -e 's|^//||' -e 's|:|/|' \
  | LC_ALL=C sort -u > "$scratch/sources.txt"
gate_rc=0
gate_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$inventory" --sources "$scratch/sources.txt" --root . 2>&1)" || gate_rc=$?
if [[ "$gate_rc" == "0" ]] && echo "$gate_out" | grep -q 'coverage gate: PASS'; then
  ok
else
  bad "seed cell gate did not pass: rc=$gate_rc out=$gate_out"
fi

# Missing report file fails the gate (exit 1), never a usage error.
missing_rc=0
missing_out="$("$check_bin" --report "$scratch/absent.lcov" \
  --inventory "$inventory" --sources "$scratch/sources.txt" --root . 2>&1)" || missing_rc=$?
if [[ "$missing_rc" == "1" ]] && echo "$missing_out" | grep -q 'missing report file'; then
  ok
else
  bad "missing report did not fail closed: rc=$missing_rc out=$missing_out"
fi

# An instrumented source dropped from the inventory fails the gate.
grep -v -F -e 'tools/coverage/src/main.rs' "$inventory" > "$scratch/no-main-inventory.txt"
uninventoried_rc=0
uninventoried_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$scratch/no-main-inventory.txt" --sources "$scratch/sources.txt" \
  --root . 2>&1)" || uninventoried_rc=$?
if [[ "$uninventoried_rc" == "1" ]] && echo "$uninventoried_out" | grep -q 'not in inventory'; then
  ok
else
  bad "uninventoried source did not fail closed: rc=$uninventoried_rc out=$uninventoried_out"
fi

# An eligible inventory entry dropped from the Bazel source list fails.
grep -v -F -e 'cli/lcov/src/lib.rs' "$scratch/sources.txt" > "$scratch/no-lib-sources.txt"
undeclared_rc=0
undeclared_out="$("$check_bin" --report bazel-out/_coverage/_coverage_report.dat \
  --inventory "$inventory" --sources "$scratch/no-lib-sources.txt" --root . 2>&1)" || undeclared_rc=$?
if [[ "$undeclared_rc" == "1" ]] && echo "$undeclared_out" | grep -q 'not declared by Bazel'; then
  ok
else
  bad "undeclared eligible source did not fail closed: rc=$undeclared_rc out=$undeclared_out"
fi

# A zero-hit eligible line fails with its location, not just a rate dip.
printf 'SF:cli/lcov/src/lib.rs\nDA:1,0\nend_of_record\n' > "$scratch/uncovered.lcov"
printf 'eligible cli/lcov/src/lib.rs\n' > "$scratch/uncovered-inventory.txt"
printf 'cli/lcov/src/lib.rs\n' > "$scratch/uncovered-sources.txt"
uncovered_rc=0
uncovered_out="$("$check_bin" --report "$scratch/uncovered.lcov" \
  --inventory "$scratch/uncovered-inventory.txt" --sources "$scratch/uncovered-sources.txt" \
  --root . 2>&1)" || uncovered_rc=$?
if [[ "$uncovered_rc" == "1" ]] && echo "$uncovered_out" | grep -q 'uncovered: cli/lcov/src/lib.rs:1'; then
  ok
else
  bad "uncovered line did not fail with location: rc=$uncovered_rc out=$uncovered_out"
fi

# The //... rate gate still guards the whole tree in CI.
if grep -q -F -e 'coverage --min-coverage' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the dx coverage --min-coverage rate gate"
fi

echo "coverage cell harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
