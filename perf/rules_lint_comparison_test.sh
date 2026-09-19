#!/usr/bin/env bash
# Rules-lint comparison harness self-test (issue #86, schema workflow issue #322).
#
# Exercises the REAL perf/rules_lint_comparison.sh: same seed yields the
# same tree_sha256 (deterministic generator), dirty counts match the
# requested mix, fairness pins (bazel_version, rules_lint v2.8.0, host)
# are recorded, and the report stays report-not-gate (gate false, no
# parity claim). Schema validation, not brittle equality: dict-shape checks
# assert types/presence (never exact tool pins); the on-disk `diff -r` below
# is an idempotence assertion between two fresh runs, not a checked-in
# golden, so UPDATE_EXPECT does not apply. Tagged `no-coverage`: timing
# harnesses stay out of the coverage denominator per the repo coverage preset.
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../tools/sh/lib.sh"

harness="$1"
command -v python3 >/dev/null || {
  echo "python3 is required" >&2
  exit 1
}

dx_mkscratch scratch

dx_test_init

check_json() { # name python-expr file
  local name="$1" expr="$2" file="$3"
  if python3 -c "import json,sys; doc=json.load(open(sys.argv[1])); assert ($expr), 'check failed'" "$file"; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name" >&2
    fail=$((fail + 1))
  fi
}

bash "$harness" --files 200 --dirty-pct 10 --seed 86 --out "$scratch/a" >"$scratch/r1.json"
bash "$harness" --files 200 --dirty-pct 10 --seed 86 --out "$scratch/b" >"$scratch/r2.json"

# Determinism: same seed, same digest and counts.
d1="$(python3 -c 'import json; print(json.load(open("'"$scratch"'/r1.json"))["tree_sha256"])')"
d2="$(python3 -c 'import json; print(json.load(open("'"$scratch"'/r2.json"))["tree_sha256"])')"
if [[ "$d1" == "$d2" && -n "$d1" ]]; then pass=$((pass + 1)); else
  echo "FAIL: deterministic digest ($d1 vs $d2)" >&2
  fail=$((fail + 1))
fi

# On-disk trees are identical file-for-file.
if diff -r "$scratch/a" "$scratch/b" >/dev/null; then pass=$((pass + 1)); else
  echo "FAIL: on-disk trees differ" >&2
  fail=$((fail + 1))
fi

check_json "dirty count matches mix" "doc['dirty_files']==20 and doc['files']==200" "$scratch/r1.json"
check_json "rules_lint pin recorded" "doc['rules_lint_pin']=='v2.8.0'" "$scratch/r1.json"
check_json "host recorded" "doc['host']=='linux_x86_64'" "$scratch/r1.json"
check_json "bazel version recorded" "isinstance(doc['bazel_version'],str) and len(doc['bazel_version'])>0" "$scratch/r1.json"
check_json "report-not-gate" "doc['gate'] is False and doc['claim']=='none'" "$scratch/r1.json"

# Dirty triggers are real: dirty markdown has trailing whitespace,
# dirty Starlark has unformatted `=1`.
if grep -r -q -F -e '   ' "$scratch/a/doc_0001.md" && grep -r -q -F -e 'CONST_2=1' "$scratch/a/target_0002.bzl"; then
  pass=$((pass + 1))
else
  echo "FAIL: dirty triggers missing" >&2
  fail=$((fail + 1))
fi
# Clean files lack the triggers.
if ! grep -r -q -F -e '   ' "$scratch/a/doc_0199.md" && grep -r -q -F -e 'CONST_200 = 1' "$scratch/a/target_0200.bzl"; then
  pass=$((pass + 1))
else
  echo "FAIL: clean files carry dirty triggers" >&2
  fail=$((fail + 1))
fi

dx_test_summary "rules_lint_comparison_self_test"
