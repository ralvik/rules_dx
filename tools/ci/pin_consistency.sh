#!/usr/bin/env bash
# Pin single-source consistency (issue #326).
#
# Canonical sources:
#   Bazel version: `.bazelversion` (Bazelisk reads it; every other Bazel pin
#     tracks it).
#   Bazelisk version + sha256: `.github/actions/setup-bazelisk/action.yml`
#     defaults (the single installer; Dockerfile and docs bootstrap track it).
#
# Every other pin below must equal its canonical source or this fails, so a
# version bump means: bump the canonical file once, then update the tracked
# copies in the same reviewed change.
#
# Usage: pin_consistency.sh <bazelversion> <module> <preset_py>
#   <dockerfile> <tested_stack> <action_yml> <local_workflows>
set -euo pipefail

# Shared workspace + runfiles helpers (issues #319, #323).
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_test_init

bazelversion="${1:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
module="${2:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
preset_py="${3:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
dockerfile="${4:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
tested_stack="${5:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
action_yml="${6:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"
local_workflows="${7:?usage: pin_consistency.sh <bazelversion> <module> <preset_py> <dockerfile> <tested_stack> <action_yml> <local_workflows>}"

# --- Bazel canonical ---
bazel_pin="$(tr -d '[:space:]' <"$bazelversion")"
if [[ -z "$bazel_pin" ]]; then
  bad ".bazelversion is empty"
else
  ok
fi

check_bazel_pin() { # description, actual
  if [[ "$2" == "$bazel_pin" ]]; then
    ok
  else
    bad "$1 is $2, want canonical .bazelversion $bazel_pin"
  fi
}

module_pin="$(grep -o -E -e 'bazel_binaries\.download\(version = "[^"]+"' "$module" | head -1 | cut -d'"' -f2 || true)"
check_bazel_pin "MODULE.bazel bazel_binaries pin" "$module_pin"

preset_py_pin="$(grep -o -E -e 'PRESET_BAZEL_VERSION = "[^"]+"' "$preset_py" | head -1 | cut -d'"' -f2 || true)"
check_bazel_pin "tools/bazelrc/preset.py pin" "$preset_py_pin"

docker_bazel_pin="$(grep -o -E -e 'USE_BAZEL_VERSION=[0-9.]+' "$dockerfile" | head -1 | cut -d= -f2 || true)"
check_bazel_pin "Dockerfile USE_BAZEL_VERSION" "$docker_bazel_pin"

stack_pin="$(grep -A2 -F -e '"bazel_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
check_bazel_pin "tested_stack.bzl bazel_version default" "$stack_pin"

# --- Bazelisk canonical (action.yml defaults) ---
# First default is the Bazelisk version, second is the sha256 (file order).
action_version="$(grep -o -E -e 'default: "[^"]+"' "$action_yml" | head -1 | cut -d'"' -f2 || true)"
action_sha="$(grep -o -E -e 'default: "[^"]+"' "$action_yml" | sed -n '2p' | cut -d'"' -f2 || true)"
if [[ -z "$action_version" || -z "$action_sha" ]]; then
  bad "setup-bazelisk action.yml missing version/sha defaults"
else
  ok
fi

docker_bazelisk_version="$(grep -o -E -e 'bazelisk/releases/download/v[0-9.]+/bazelisk-linux-amd64' "$dockerfile" | head -1 | sed -E 's|.*/v([0-9.]+)/.*|\1|' || true)"
docker_bazelisk_sha="$(grep -o -E -e '[0-9a-f]{64}  /tmp/bazelisk' "$dockerfile" | head -1 | cut -d' ' -f1 || true)"
if [[ "$docker_bazelisk_version" == "$action_version" ]]; then
  ok
else
  bad "Dockerfile Bazelisk v$docker_bazelisk_version drifts from canonical action.yml v$action_version"
fi
if [[ "$docker_bazelisk_sha" == "$action_sha" ]]; then
  ok
else
  bad "Dockerfile Bazelisk sha drifts from canonical action.yml sha"
fi

docs_bazelisk_version="$(grep -o -E -e 'bazelisk/releases/download/v[0-9.]+/bazelisk-linux-amd64' "$local_workflows" | head -1 | sed -E 's|.*/v([0-9.]+)/.*|\1|' || true)"
docs_bazelisk_sha="$(grep -o -E -e '[0-9a-f]{64}  /tmp/bazelisk' "$local_workflows" | head -1 | cut -d' ' -f1 || true)"
if [[ "$docs_bazelisk_version" == "$action_version" ]]; then
  ok
else
  bad "local-workflows.md Bazelisk v$docs_bazelisk_version drifts from canonical action.yml v$action_version"
fi
if [[ "$docs_bazelisk_sha" == "$action_sha" ]]; then
  ok
else
  bad "local-workflows.md Bazelisk sha drifts from canonical action.yml sha"
fi

dx_test_summary "pin consistency"
