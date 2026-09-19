#!/usr/bin/env bash
# Consumer CI aggregate harness (#99 item 1).
#
# Executes the REAL dx-ci aggregation python extracted from
# .github/workflows/reusable-consumer.yml (second `python3 - <<'EOF'`
# heredoc) over NEEDS-result matrices: all-success, single failure,
# disabled-as-skipped, cancelled, and empty. A static check pins the
# dx-ci `needs` list to the gate plus all nine checks with
# `if: always()`, so a dropped job breaks the aggregate loudly.
# The closing negative control proves sensitivity: a mutated NEEDS map
# missing the failing job must stop failing.
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

workflow="$1"

command -v python3 >/dev/null || {
  echo "python3 is required" >&2
  exit 1
}

dx_mkscratch scratch

heredocs="$(grep -c "python3 - <<'EOF'" "$workflow" || true)"
[[ "$heredocs" == "2" ]] || {
  echo "want 2 embedded python heredocs, found $heredocs" >&2
  exit 1
}

# Strip the `run: |` block indent so the extract parses as top-level python.
awk "/python3 - <<'EOF'/{count++; f=(count==2); next} f && /^[[:space:]]*EOF$/{exit} f{sub(/^          /, \"\"); print}" "$workflow" >"$scratch/aggregate.py"
grep -q "failing checks" "$scratch/aggregate.py" || {
  echo "aggregate extraction missed the script" >&2
  exit 1
}

dx_test_init
check() { # name, want_exit, want_substring, NEEDS-json
  local name="$1" want_exit="$2" want_sub="$3" needs="$4"
  local out rc=0
  out="$(NEEDS="$needs" python3 "$scratch/aggregate.py" 2>&1)" || rc=$?
  if [[ "$rc" == "$want_exit" && "$out" == *"$want_sub"* ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name (exit=$rc, want $want_exit; output: $out)" >&2
    fail=$((fail + 1))
  fi
}

all_ok='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"success"},"build":{"result":"success"},"coverage":{"result":"success"}}'
one_fail='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"failure"},"build":{"result":"success"},"coverage":{"result":"success"}}'
disabled_skip='{"platforms-gate":{"result":"success"},"lint":{"result":"skipped"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"success"},"build":{"result":"success"},"coverage":{"result":"success"}}'
cancelled='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"test":{"result":"cancelled"},"build":{"result":"success"},"coverage":{"result":"success"}}'

check "all success aggregates green" 0 '"aggregate": "dx-ci"' "$all_ok"
check "single failure names the check" 1 "failing checks: test" "$one_fail"
check "disabled-as-skipped stays green" 0 '"aggregate": "dx-ci"' "$disabled_skip"
check "cancelled fails the aggregate" 1 "failing checks: test" "$cancelled"
check "empty needs map stays green" 0 '"aggregate": "dx-ci"' '{}'

# Static: dx-ci needs the gate plus all nine checks, always().
needs_line="$(grep -A 2 '^  dx-ci:' "$workflow" | grep 'needs:' || true)"
for job in platforms-gate lint typecheck format generate security-audit license-audit test build coverage; do
  if [[ "$needs_line" == *"$job"* ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: dx-ci needs list drops $job" >&2
    fail=$((fail + 1))
  fi
done
if grep -A 3 '^  dx-ci:' "$workflow" | grep -q 'if: ${{ always() }}'; then
  pass=$((pass + 1))
else
  echo "FAIL: dx-ci lost its always() guard" >&2
  fail=$((fail + 1))
fi

# Negative control: dropping the failing job from NEEDS must flip the
# verdict to green, proving the matrix observes the failing cell.
no_test='{"platforms-gate":{"result":"success"},"lint":{"result":"success"},"typecheck":{"result":"success"},"format":{"result":"success"},"generate":{"result":"success"},"security-audit":{"result":"success"},"license-audit":{"result":"success"},"build":{"result":"success"},"coverage":{"result":"success"}}'
mut_out="$(NEEDS="$no_test" python3 "$scratch/aggregate.py" 2>&1)" && mut_rc=0 || mut_rc=$?
if [[ "$mut_rc" == "0" ]]; then
  pass=$((pass + 1))
else
  echo "FAIL: negative control insensitive (output: $mut_out)" >&2
  fail=$((fail + 1))
fi

dx_test_summary "consumer aggregate harness"
