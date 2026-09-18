#!/usr/bin/env bash
# Perf comparison self-test (issue #215).
#
# Exercises the REAL perf/compare.py against the REAL perf/baseline.json
# with synthetic JSON-lines inputs: at-baseline medians pass clean, a
# relative regression warns but still exits 0 (warn-only by design), an
# unknown benchmark warns but exits 0, and a gated absolute-budget breach
# (dx_startup, the only `gate: true` budget) exits 1. A closing static
# check pins the strictness ladder: exactly one gated budget exists, so a
# newly gated benchmark breaks this test loudly and forces a reviewed
# policy change.
set -euo pipefail

compare_py="$1"
baseline="$2"

command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0

emit() { # file benchmark median_ms count
  local file="$1" name="$2" med="$3" count="$4"
  python3 -c '
import json, sys
name, med, count = sys.argv[1], float(sys.argv[2]), int(sys.argv[3])
for i in range(1, count + 1):
    print(json.dumps({"benchmark": name, "duration_ms": med, "iteration": i}))
' "$name" "$med" "$count" >> "$file"
}

check() { # name, want_exit, want_substring, results-file...
  local name="$1" want_exit="$2" want_sub="$3"
  shift 3
  local out rc=0
  out="$(python3 "$compare_py" --baseline "$baseline" "$@" 2>&1)" || rc=$?
  if [[ "$rc" == "$want_exit" && "$out" == *"$want_sub"* ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name (exit=$rc, want $want_exit; output: $out)" >&2
    fail=$((fail + 1))
  fi
}

at_baseline="$scratch/at.jsonl"
emit "$at_baseline" dx_startup 2.5 7
emit "$at_baseline" dx_status 2.2 7
emit "$at_baseline" scope_owners 252.2 5
emit "$at_baseline" generate_check_warm 462.8 5
check "at-baseline passes clean" 0 "No regressions" "$at_baseline"

regressed="$scratch/regressed.jsonl"
emit "$regressed" scope_owners 500.0 5
check "relative regression warns" 0 "REGRESSION scope_owners" "$regressed"
# Warn-only proof: the same regressed input must NOT fail the gate.
out="$(python3 "$compare_py" --baseline "$baseline" "$regressed" 2>&1)"; rc=$?
[[ "$rc" == "0" ]] || { echo "FAIL: relative regression must exit 0, got $rc" >&2; fail=$((fail + 1)); }

unknown="$scratch/unknown.jsonl"
emit "$unknown" dx_startup 2.5 7
emit "$unknown" future_benchmark 1.0 3
check "unknown benchmark warns, passes" 0 "UNKNOWN" "$unknown"

breach="$scratch/breach.jsonl"
emit "$breach" dx_startup 200.0 7
check "gated absolute breach fails" 1 "GATE dx_startup" "$breach"

# Advisory-budget proof: scope_owners has a 2000ms budget but gate=false,
# so a 2500ms median warns without failing.
advisory="$scratch/advisory.jsonl"
emit "$advisory" scope_owners 2500.0 5
check "advisory budget breach warns only" 0 "BUDGET-ADVISORY scope_owners" "$advisory"

gated="$(python3 -c '
import json, sys
doc = json.load(open(sys.argv[1]))
print(sum(1 for spec in doc["benchmarks"].values() if spec.get("gate")))
' "$baseline")"
if [[ "$gated" == "1" ]]; then
  pass=$((pass + 1))
else
  echo "FAIL: want exactly 1 gated budget (dx_startup), found $gated" >&2
  fail=$((fail + 1))
fi

# Workspace-relative baseline proof (issue #86): under `bazel run` the
# process starts in runfiles, so a relative perf/baseline.json must
# resolve against BUILD_WORKSPACE_DIRECTORY (same convention as
# bench.sh/regenerate.py). Mirror a workspace, resolve from elsewhere,
# and prove the relative path still loads. The compare script path is
# made absolute first: the check cds away from the test runfiles cwd.
ws_root="$scratch/ws"
mkdir -p "$ws_root/perf"
cp "$baseline" "$ws_root/perf/baseline.json"
compare_abs="$(CDPATH= cd -- "$(dirname "$compare_py")" && pwd)/$(basename "$compare_py")"
ws_out=""; rc=0
ws_out="$(cd /tmp && BUILD_WORKSPACE_DIRECTORY="$ws_root" python3 "$compare_abs" --baseline perf/baseline.json "$at_baseline" 2>&1)" || rc=$?
if [[ "$rc" == "0" && "$ws_out" == *"No regressions"* ]]; then
  pass=$((pass + 1))
else
  echo "FAIL: workspace-relative baseline (rc=$rc, output: $ws_out)" >&2
  fail=$((fail + 1))
fi

echo "perf_self_test: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
