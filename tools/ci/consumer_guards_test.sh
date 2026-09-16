#!/usr/bin/env bash
# Consumer CI check-selection harness (#99 item 1).
#
# Pins the disabled-checks contract statically: every one of the nine
# check jobs carries the exact opt-out guard naming its own check ID,
# and the starter caller enables all nine (`disabled_checks: ""`).
# The closing negative control proves sensitivity: the same checker
# run against a mutated workflow copy missing one guard must fail.
set -euo pipefail

workflow="$1"
caller="$2"

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0
check_file() { # file, name, want_count, fixed needle
  local file="$1" name="$2" want="$3" needle="$4"
  local got
  got="$(grep -c -F -e "$needle" "$file" || true)"
  if [[ "$got" == "$want" ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name (found $got, want $want)" >&2
    fail=$((fail + 1))
  fi
}

for id in lint typecheck format generate security-audit license-audit test build coverage; do
  check_file "$workflow" "guard for $id" 1 "if: \${{ !contains(format(',{0},', inputs.disabled_checks), ',$id,') }}"
  check_file "$workflow" "job $id exists" 1 "  $id:"
done

check_file "$caller" "starter enables all nine" 1 'disabled_checks: ""'
check_file "$caller" "starter passes explicit platforms" 1 "platforms: '[\"linux_x86_64\"]'"
check_file "$caller" "starter stays parallel" 1 'scheduling_mode: "parallel"'

# Per-platform jobs wait on the gate; Linux-once jobs never do (an
# independent check must not be blocked by platform validation).
for id in test build coverage; do
  if grep -A 3 "^  $id:" "$workflow" | grep -q 'needs: \[platforms-gate\]'; then
    pass=$((pass + 1))
  else
    echo "FAIL: per-platform job $id lost its platforms-gate edge" >&2
    fail=$((fail + 1))
  fi
done
for id in lint typecheck format generate security-audit license-audit; do
  if grep -A 3 "^  $id:" "$workflow" | grep -q 'needs:'; then
    echo "FAIL: Linux-once job $id gained a needs edge" >&2
    fail=$((fail + 1))
  else
    pass=$((pass + 1))
  fi
done

# Negative control: drop the coverage guard; the checker must flag it.
# (pipefail would poison a `grep -c | grep -q` pipeline because grep -c
# exits 1 on zero matches, so capture the count first.)
mutated="$scratch/mutated.yml"
grep -v -F -e "inputs.disabled_checks), ',coverage,') }}" "$workflow" > "$mutated"
remaining="$(grep -c -F -e "inputs.disabled_checks), ',coverage,') }}" "$mutated" || true)"
if [[ "$remaining" == "0" ]]; then
  pass=$((pass + 1))
else
  echo "FAIL: negative control setup broken (guard still present)" >&2
  fail=$((fail + 1))
fi

echo "consumer guards harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
