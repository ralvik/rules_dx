#!/usr/bin/env bash
# Pin single-source consistency.
#
# Canonical sources:
#   Bazel version: `.bazelversion` (Bazelisk reads it; every other Bazel pin
#     tracks it).
#   Bazelisk version + per-OS sha256: `.github/actions/setup-bazelisk/action.yml`
# defaults (the single portable installer,; Dockerfile tracks
#     the linux-amd64 pair and docs bootstrap tracks all five hosts).
#
# Every other pin below must equal its canonical source or this fails, so a
# version bump means: bump the canonical file once, then update the tracked
# copies in the same reviewed change.
#
# Usage: pin_consistency.sh <bazelversion> <module> <preset_py>
#   <dockerfile> <tested_stack> <action_yml> <local_workflows>
set -euo pipefail

# Shared workspace + runfiles helpers.
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

# Nested E2E removed, so MODULE.bazel carries no second-Bazel
# `bazel_binaries.download` pin. CI uses the single canonical `.bazelversion`
# Bazel via Bazelisk only; the pin must stay absent.
if grep -q -F -e 'bazel_binaries.download' "$module"; then
  bad "MODULE.bazel must not carry a bazel_binaries.download pin (issue #407: single Bazel only)"
else
  ok
fi

preset_py_pin="$(grep -o -E -e 'PRESET_BAZEL_VERSION = "[^"]+"' "$preset_py" | head -1 | cut -d'"' -f2 || true)"
check_bazel_pin "tools/bazelrc/preset.py pin" "$preset_py_pin"

docker_bazel_pin="$(grep -o -E -e 'USE_BAZEL_VERSION=[0-9.]+' "$dockerfile" | head -1 | cut -d= -f2 || true)"
check_bazel_pin "Dockerfile USE_BAZEL_VERSION" "$docker_bazel_pin"

stack_pin="$(grep -A2 -F -e '"bazel_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
check_bazel_pin "tested_stack.bzl bazel_version default" "$stack_pin"

# --- Bazelisk canonical (action.yml defaults, portable) ---
# Canonical: version plus per-OS sha256 inputs in setup-bazelisk/action.yml.
# Dockerfile tracks the linux-amd64 pair; local-workflows.md documents all
# five qualified hosts.
action_version="$(grep -A3 -F -e 'version:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
action_sha_linux_amd64="$(grep -A3 -F -e 'sha256_linux_amd64:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
action_sha_linux_arm64="$(grep -A3 -F -e 'sha256_linux_arm64:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
action_sha_darwin_amd64="$(grep -A3 -F -e 'sha256_darwin_amd64:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
action_sha_darwin_arm64="$(grep -A3 -F -e 'sha256_darwin_arm64:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
action_sha_windows_amd64="$(grep -A3 -F -e 'sha256_windows_amd64:' "$action_yml" | grep -o -E -e 'default: "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -z "$action_version" || -z "$action_sha_linux_amd64" || -z "$action_sha_linux_arm64" || -z "$action_sha_darwin_amd64" || -z "$action_sha_darwin_arm64" || -z "$action_sha_windows_amd64" ]]; then
  bad "setup-bazelisk action.yml missing version/per-OS sha defaults (issue #617)"
else
  ok
fi
# Legacy single-sha input must stay absent: per-OS pins replace it.
if grep -A3 -E -e '^  sha256:' "$action_yml" | grep -q -F -e 'default:'; then
  bad "setup-bazelisk action.yml still carries legacy single sha256 input (want per-OS sha256_* only, issue #617)"
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
if [[ "$docker_bazelisk_sha" == "$action_sha_linux_amd64" ]]; then
  ok
else
  bad "Dockerfile Bazelisk sha drifts from canonical action.yml linux-amd64 sha"
fi

# Docs bootstrap tracks the canonical version plus all five per-OS shas
# (portable bootstrap).
for sha in "$action_sha_linux_amd64" "$action_sha_linux_arm64" "$action_sha_darwin_amd64" "$action_sha_darwin_arm64" "$action_sha_windows_amd64"; do
  if grep -q -F -e "$sha" "$local_workflows"; then
    ok
  else
    bad "local-workflows.md missing canonical Bazelisk sha $sha (issue #617)"
  fi
done
if grep -q -F -e "v$action_version" "$local_workflows"; then
  ok
else
  bad "local-workflows.md Bazelisk v drifts from canonical action.yml v$action_version"
fi
for asset in bazelisk-linux-amd64 bazelisk-linux-arm64 bazelisk-darwin-amd64 bazelisk-darwin-arm64 bazelisk-windows-amd64.exe; do
  if grep -q -F -e "$asset" "$local_workflows"; then
    ok
  else
    bad "local-workflows.md missing canonical Bazelisk asset $asset (issue #617)"
  fi
done

dx_test_summary "pin consistency"
