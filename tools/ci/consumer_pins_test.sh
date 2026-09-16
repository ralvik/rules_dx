#!/usr/bin/env bash
# Consumer CI pin + inputs harness (#99 item 1).
#
# Pins the supply-chain half of the contract statically: the starter
# caller references the reusable workflow by full-length commit SHA,
# every third-party action pins to a SHA with a `# vN` tag comment,
# all Bazel setup flows through the in-repo setup-bazelisk action,
# `rules_dx_version` matches the `module()` version in MODULE.bazel,
# and the coverage threshold fragment maps DX_MIN_COVERAGE to a
# `--min-coverage` flag only for well-formed decimals.
# The closing negative controls prove sensitivity: a mutated workflow
# with a floating action tag, and a malformed threshold value, must fail.
set -euo pipefail

workflow="$1"
caller="$2"
module="$3"

command -v python3 >/dev/null || { echo "python3 is required" >&2; exit 1; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

pass=0
fail=0
ok() { pass=$((pass + 1)); }
bad() { echo "FAIL: $1" >&2; fail=$((fail + 1)); }

# Starter references the reusable workflow by full commit SHA, once.
if [[ "$(grep -c -E -e 'uses: rules_dx/\.github/workflows/reusable-consumer\.yml@[0-9a-f]{40}' "$caller")" == "1" ]]; then
  ok
else
  bad "caller must reference reusable-consumer.yml by full SHA exactly once"
fi

# No floating `uses: owner/repo@tag` anywhere in either file.
if grep -E -e 'uses: [^ ]+@(v[0-9]|main|master|latest)' "$workflow" "$caller" >/dev/null; then
  bad "floating action tag found"
else
  ok
fi

# Every actions/* pin carries a `# vN` tag comment; every pin is a full SHA.
actions_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40}' "$workflow" "$caller" | wc -l)"
commented_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40} # v[0-9]+' "$workflow" "$caller" | wc -l)"
if [[ "$actions_pins" -gt "0" && "$actions_pins" == "$commented_pins" ]]; then
  ok
else
  bad "actions/* pins must be full SHAs with a # vN tag comment ($commented_pins/$actions_pins)"
fi

# Single-source Bazel setup: every non-comment setup-bazelisk mention is
# a use of the in-repo action. (The action itself is the one installer;
# its pinned BAZELISK_VERSION lives in .github/actions/setup-bazelisk/.)
setup_uses="$(grep -c -F -e './.github/actions/setup-bazelisk' "$workflow" || true)"
if [[ "$setup_uses" -ge "9" ]] && ! grep -e 'setup-bazelisk' "$workflow" | grep -v -F -e './.github/actions/setup-bazelisk' | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  ok
else
  bad "setup-bazelisk must be the only Bazel setup path (uses=$setup_uses)"
fi
if grep -i -h -E -e 'curl.*bazelisk|bazelisk.*(download|install)|npm i.*bazelisk' "$workflow" "$caller" | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  bad "inline bazelisk install found outside the setup action"
else
  ok
fi

# `rules_dx_version` matches the module() version.
caller_version="$(grep -o -E -e 'rules_dx_version: "[^"]+"' "$caller" | head -1 | cut -d'"' -f2)"
module_version="$(awk '/^module\(/,/^\)/' "$module" | grep -o -E -e 'version = "[^"]+"' | head -1 | cut -d'"' -f2)"
if [[ -n "$caller_version" && "$caller_version" == "$module_version" ]]; then
  ok
else
  bad "rules_dx_version ($caller_version) must match module() version ($module_version)"
fi

# Coverage threshold fragment: extract the DX_MIN_COVERAGE block from
# the coverage job (anchored to the `run: |` indent so the range cannot
# bleed into the following steps) and execute it over a value matrix.
awk '/^          extra=""$/,/^          fi$/ {sub(/^          /, ""); print}' "$workflow" > "$scratch/threshold.sh"
grep -q 'DX_MIN_COVERAGE' "$scratch/threshold.sh" || { echo "threshold extraction missed the block" >&2; exit 1; }
if grep -q -E -e '^ *- (name|uses|run):' -e '^  [a-z-]+:' "$scratch/threshold.sh"; then
  echo "threshold extraction bled outside its block" >&2
  exit 1
fi
threshold() { # value-or-unset, want_extra, want_exit
  local value="$1" want_extra="$2" want_exit="$3"
  local script="$scratch/run_threshold.sh"
  {
    cat "$scratch/threshold.sh"
    echo 'printf "%s:%s" "$extra" "$?"'
  } > "$script"
  local out rc=0
  if [[ "$value" == "UNSET" ]]; then
    out="$(env -u DX_MIN_COVERAGE bash "$script" 2>&1)" || rc=$?
  else
    out="$(DX_MIN_COVERAGE="$value" bash "$script" 2>&1)" || rc=$?
  fi
  # Valid values fall through to the probe (`extra:rc`); rejections exit
  # inside the block, so their output is the bare error message.
  local want
  if [[ "$want_exit" == "0" ]]; then
    want="$want_extra:$want_exit"
  else
    want="$want_extra"
  fi
  if [[ "$out" == "$want" && "$rc" == "$want_exit" ]]; then
    ok
  else
    bad "DX_MIN_COVERAGE=${value} gave '$out' (rc=$rc), want '$want' (rc=$want_exit)"
  fi
}

threshold "UNSET" "" 0
threshold "" "" 0
threshold "80" "--min-coverage 80" 0
threshold "80.5" "--min-coverage 80.5" 0
# Rejections print the block's exact error to stderr and exit before
# the probe, so the output is the message with no `:rc` suffix.
threshold "abc" "dx-ci: invalid min_coverage 'abc': want e.g. 80 or 80.5" 1
threshold "1.2.3" "dx-ci: invalid min_coverage '1.2.3': want e.g. 80 or 80.5" 1
threshold ".5" "dx-ci: invalid min_coverage '.5': want e.g. 80 or 80.5" 1
threshold "5." "dx-ci: invalid min_coverage '5.': want e.g. 80 or 80.5" 1
threshold "80;evil" "dx-ci: invalid min_coverage '80;evil': want e.g. 80 or 80.5" 1

# Negative control: float one action pin to its tag (the realistic
# regression: `uses: actions/checkout@v7`); the pin check must fail.
mutated="$scratch/mutated.yml"
sed -E 's|@[0-9a-f]{40} # v([0-9]+)|@v\1|' "$workflow" > "$mutated"
if grep -E -e 'uses: [^ ]+@(v[0-9]|main|master|latest)' "$mutated" >/dev/null; then
  ok
else
  bad "negative control setup broken (no floating tag after mutation)"
fi

echo "consumer pins harness: $pass passed, $fail failed"
[[ "$fail" == "0" ]]
