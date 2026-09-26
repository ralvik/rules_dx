#!/usr/bin/env bash
# Consumer CI pin + inputs harness (item 1).
#
# Pins the supply-chain half of the contract statically: the starter
# caller references the reusable workflow by full-length commit SHA,
# every third-party action pins to a SHA with a `# vN` tag comment,
# all Bazel setup flows through the pinned bazel-contrib/setup-bazel
# action,
# `rules_dx_version` matches the `module()` version in MODULE.bazel,
# and the coverage threshold fragment maps DX_MIN_COVERAGE to a
# `--min-coverage` flag only for well-formed decimals.
# The closing negative controls prove sensitivity: a mutated workflow
# with a floating action tag, and a malformed threshold value, must fail.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

workflow="$1"
caller="$2"
module="$3"

command -v python3 >/dev/null || {
  echo "python3 is required" >&2
  exit 1
}

dx_mkscratch scratch

dx_test_init

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
# Checkout pins are inline per job (step 1), so pin counting covers the
# workflow plus caller.
actions_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40}' "$workflow" "$caller" | wc -l)"
commented_pins="$(grep -h -o -E -e 'uses: actions/[^ ]+@[0-9a-f]{40} # v[0-9]+' "$workflow" "$caller" | wc -l)"
if [[ "$actions_pins" -gt "0" && "$actions_pins" == "$commented_pins" ]]; then
  ok
else
  bad "actions/* pins must be full SHAs with a # vN tag comment ($commented_pins/$actions_pins)"
fi

# Single-source Bazel setup (issue #915): every non-comment setup mention
# is a use of the pinned bazel-contrib/setup-bazel action; each job starts
# with inline no-secrets checkout then that action
# (its pinned BAZELISK_VERSION lives in .github/workflows/*.yml).
setup_uses="$(grep -c -F -e 'bazel-contrib/setup-bazel@' "$workflow" || true)"
checkout_uses="$(grep -c -F -e 'persist-credentials: false' "$workflow" || true)"
if [[ "$setup_uses" -ge "9" && "$checkout_uses" -ge "9" ]] && ! grep -e 'setup-bazelisk\|restore-bazel-cache' "$workflow" | grep -v -E -e '^[[:space:]]*#' | grep -q .; then
  ok
else
  bad "setup-bazel must be the only Bazel setup path with no-secrets checkout first (uses=$setup_uses checkouts=$checkout_uses)"
fi
# Bazelisk version is single-sourced through the workflow env pin and the
# remote cache is wired through the BuildBuddy secret.
if grep -q -F -e 'bazelisk-version: ${{ env.BAZELISK_VERSION }}' "$workflow" &&
  grep -q -F -e 'BAZELISK_VERSION: "' "$workflow" &&
  grep -q -F -e 'common --remote_cache=grpcs://remote.buildbuddy.io' "$workflow" &&
  grep -q -F -e 'BUILDBUDDY_API_KEY: ${{ secrets.BUILDBUDDY_API_KEY }}' "$workflow"; then
  ok
else
  bad "reusable-consumer lost the BAZELISK_VERSION env pin or BuildBuddy remote-cache wiring"
fi
# No-secrets checkout is inline per job (step 1).
if [[ "$checkout_uses" -ge "9" ]]; then
  ok
else
  bad "reusable-consumer lost no-secrets checkout record (persist-credentials: false)"
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
# Array form (issue #914): `extra` is an argv array, probed via
# `${extra[*]}` (space-joined) so empty stays zero words.
awk '/^          extra=\(\)$/,/^          fi$/ {sub(/^          /, ""); print}' "$workflow" >"$scratch/threshold.sh"
grep -q 'DX_MIN_COVERAGE' "$scratch/threshold.sh" || {
  echo "threshold extraction missed the block" >&2
  exit 1
}
if grep -q -E -e '^ *- (name|uses|run):' -e '^  [a-z-]+:' "$scratch/threshold.sh"; then
  echo "threshold extraction bled outside its block" >&2
  exit 1
fi
threshold() { # value-or-unset, want_extra, want_exit
  local value="$1" want_extra="$2" want_exit="$3"
  local script="$scratch/run_threshold.sh"
  {
    cat "$scratch/threshold.sh"
    echo 'printf "%s:%s" "${extra[*]}" "$?"'
  } >"$script"
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
# Checkout pins are inline per job (issue #915 follow-up), so the
# control mutates the workflow itself.
mutated="$scratch/mutated.yml"
sed -E 's|@[0-9a-f]{40} # v([0-9]+)|@v\1|' "$workflow" >"$mutated"
if grep -E -e 'uses: [^ ]+@(v[0-9]|main|master|latest)' "$mutated" >/dev/null; then
  ok
else
  bad "negative control setup broken (no floating tag after mutation)"
fi

dx_test_summary "consumer pins harness"
