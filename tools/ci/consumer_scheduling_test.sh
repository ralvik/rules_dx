#!/usr/bin/env bash
# Consumer CI gate harness, scheduling half (#99 item 1).
#
# Executes the REAL platforms-gate python extracted from
# .github/workflows/reusable-consumer.yml (first `python3 - <<'EOF'`
# heredoc) over a PLATFORMS/DISABLED/SCHEDULING matrix: empty-platforms
# failure, all-disabled passthrough, malformed/unsupported selections,
# sequential fail-closed (qualification-open per docs/github-ci.md),
# and unknown-mode rejection. The closing negative control proves the
# matrix is sensitive: the same cases run against a mutated workflow
# copy with the mode validation removed must stop failing.
set -euo pipefail

workflow="$1"

command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

heredocs="$(grep -c "python3 - <<'EOF'" "$workflow" || true)"
[[ "$heredocs" == "2" ]] || { echo "want 2 embedded python heredocs, found $heredocs" >&2; exit 1; }

# The script body sits indented inside the `run: |` block; strip the
# block indent so the extracted file parses as top-level python.
awk "/python3 - <<'EOF'/{count++; f=(count==1); next} f && /^[[:space:]]*EOF$/{exit} f{sub(/^          /, \"\"); print}" "$workflow" > "$scratch/gate.py"
grep -q "no per-platform check enabled" "$scratch/gate.py" || { echo "gate extraction missed the script" >&2; exit 1; }

pass=0
fail=0
check() { # name, want_exit, want_substring, VAR=val...
  local name="$1" want_exit="$2" want_sub="$3"
  shift 3
  local out rc=0
  if [[ "${SCHEDULING-unset}" == "unset" ]]; then
    out="$(env -u SCHEDULING "$@" python3 "$scratch/gate.py" 2>&1)" || rc=$?
  else
    out="$(env "$@" "SCHEDULING=$SCHEDULING" python3 "$scratch/gate.py" 2>&1)" || rc=$?
  fi
  if [[ "$rc" == "$want_exit" && "$out" == *"$want_sub"* ]]; then
    pass=$((pass + 1))
  else
    echo "FAIL: $name (exit=$rc, want $want_exit; output: $out)" >&2
    fail=$((fail + 1))
  fi
}

SCHEDULING="parallel"
check "empty platforms fails closed" 1 "no implicit default" PLATFORMS="" DISABLED=""
check "all per-platform checks disabled passes" 0 "nothing to validate" PLATFORMS="" DISABLED="test,build,coverage"
check "spaced disabled list still recognized" 0 "nothing to validate" PLATFORMS="" DISABLED=" test, build ,coverage "
check "single linux platform passes" 0 "platforms-gate" PLATFORMS='["linux_x86_64"]' DISABLED=""
check "multi-platform passes" 0 "platforms-gate" PLATFORMS='["linux_x86_64","macos_arm64"]' DISABLED=""
check "no-linux selection passes" 0 "platforms-gate" PLATFORMS='["macos_arm64"]' DISABLED="test"
check "non-JSON platforms fails" 1 "not a JSON array" PLATFORMS='linux_x86_64' DISABLED=""
check "empty array fails" 1 "nonempty JSON array" PLATFORMS='[]' DISABLED=""
check "object fails" 1 "nonempty JSON array" PLATFORMS='{"a":1}' DISABLED=""
check "unsupported platform fails" 1 "unsupported platforms" PLATFORMS='["windows_x64"]' DISABLED=""
check "partially unsupported fails" 1 "unsupported platforms" PLATFORMS='["linux_x86_64","windows_x64"]' DISABLED=""

SCHEDULING="sequential"
check "sequential fails closed as qualification-open" 1 "qualification-open" PLATFORMS='["linux_x86_64"]' DISABLED=""
SCHEDULING="sideways"
check "unknown mode is rejected" 1 "unsupported scheduling_mode" PLATFORMS='["linux_x86_64"]' DISABLED=""
unset SCHEDULING
check "unset mode defaults to parallel" 1 "no implicit default" PLATFORMS="" DISABLED=""

# Negative control: without the mode validation, sequential must pass,
# proving this matrix would catch a removed fail-closed gate.
mutated="$scratch/mutated.yml"
python3 - "$workflow" "$mutated" <<'EOF'
import sys
text = open(sys.argv[1]).read()
block = '''          if mode == "sequential":
              print(
                  "dx-ci: scheduling_mode 'sequential' is qualification-open "
                  "(docs/github-ci.md#execution): the template schedules "
                  "parallel only until sequential ordering is qualified"
              )
              sys.exit(1)
          if mode != "parallel":
              print(f"dx-ci: unsupported scheduling_mode {mode!r}; want 'parallel'")
              sys.exit(1)
'''
assert text.count(block) == 1, "mode block not found exactly once"
open(sys.argv[2], "w").write(text.replace(block, ""))
EOF
awk "/python3 - <<'EOF'/{count++; f=(count==1); next} f && /^[[:space:]]*EOF$/{exit} f{sub(/^          /, \"\"); print}" "$mutated" > "$scratch/mutated_gate.py"
mut_out="$(SCHEDULING="sequential" PLATFORMS='["linux_x86_64"]' DISABLED="" python3 "$scratch/mutated_gate.py" 2>&1)" && mut_rc=0 || mut_rc=$?
if [[ "$mut_rc" == "0" ]]; then
  pass=$((pass + 1))
else
  echo "FAIL: negative control insensitive (mutated gate still fails: $mut_out)" >&2
  fail=$((fail + 1))
fi

echo "consumer scheduling harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
