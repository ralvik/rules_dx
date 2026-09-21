#!/usr/bin/env bash
# Pin single-source consistency.
#
# Canonical sources:
#   Bazel version: `.bazelversion` (Bazelisk reads it; every other Bazel pin
#     tracks it).
#   Bazelisk version + per-OS sha256: `.github/actions/setup-bazelisk/action.yml`
# defaults (the single portable installer,; Dockerfile tracks
#     the linux-amd64 pair and docs bootstrap tracks all five hosts).
#   Go toolchain: `MODULE.bazel` `go_sdk.download` owns the toolchain floor;
#     `third_party/go/go.mod` carries the language floor (SDK minor must stay
#     >= go.mod minor, issue #912).
#   pnpm version: root `package.json` `packageManager` owns the pnpm pin;
#     `quality/tools/javascript/package.json` must match and `MODULE.bazel`
#     resolves the toolchain from the root pin (issue #912).
#   Python foundation: `MODULE.bazel` `aspect_rules_py` prerelease stays an
#     ADR 0008 exception with an explicit bump selector (issue #912).
#
# Every other pin below must equal its canonical source or this fails, so a
# version bump means: bump the canonical file once, then update the tracked
# copies in the same reviewed change.
#
# Usage: pin_consistency.sh <bazelversion> <module> <preset_rs>
#   <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod>
#   <root_pkg> <js_pkg>
set -euo pipefail

# Shared workspace + runfiles helpers.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_test_init

bazelversion="${1:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
module="${2:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
preset_py="${3:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
dockerfile="${4:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
tested_stack="${5:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
action_yml="${6:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
local_workflows="${7:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
go_mod="${8:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
root_pkg="${9:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"
js_pkg="${10:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg>}"

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

preset_py_pin="$(grep -o -E -e 'PRESET_BAZEL_VERSION[^"]*"[^"]+"' "$preset_py" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
check_bazel_pin "tools/bazelrc/src/lib.rs pin" "$preset_py_pin"

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

# --- Go canonical: MODULE SDK owns the toolchain floor, go.mod the language floor ---
sdk_version="$(grep -o -E -e 'go_sdk\.download\(version = "[^"]+"' "$module" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
go_version="$(grep -o -E -e '^go [0-9]+\.[0-9]+(\.[0-9]+)?' "$go_mod" | head -1 | cut -d' ' -f2 || true)"
if [[ -z "$sdk_version" || -z "$go_version" ]]; then
  bad "Go pin missing (want go_sdk.download version in MODULE.bazel plus go directive in third_party/go/go.mod, issue #912)"
else
  ok
fi
if [[ -n "$sdk_version" && -n "$go_version" ]]; then
  sdk_minor="$(echo "$sdk_version" | cut -d. -f1,2)"
  go_minor="$(echo "$go_version" | cut -d. -f1,2)"
  # Compare minor versions numerically: SDK must stay >= language floor.
  sdk_maj="${sdk_minor%%.*}"
  sdk_min="${sdk_minor#*.}"
  go_maj="${go_minor%%.*}"
  go_min="${go_minor#*.}"
  if [[ "$sdk_maj" -gt "$go_maj" ]] || { [[ "$sdk_maj" == "$go_maj" ]] && [[ "$sdk_min" -ge "$go_min" ]]; }; then
    ok
  else
    bad "Go SDK $sdk_version predates go.mod language floor $go_version (want SDK minor >= go.mod minor, issue #912)"
  fi
  if grep -q -F -e 'third_party/go:go.mod' "$module"; then
    ok
  else
    bad "MODULE.bazel lost the go_deps linkage to third_party/go:go.mod (want gazelle_go_deps.from_file with go_mod, issue #912)"
  fi
fi

# --- pnpm canonical: root packageManager owns the pin, tool graph tracks it ---
root_pm="$(grep -o -E -e '"packageManager": "[^"]+"' "$root_pkg" | head -1 | cut -d'"' -f4 || true)"
js_pm="$(grep -o -E -e '"packageManager": "[^"]+"' "$js_pkg" | head -1 | cut -d'"' -f4 || true)"
if [[ -z "$root_pm" || -z "$js_pm" ]]; then
  bad "packageManager missing (want pnpm pin in both package.json files, issue #912)"
elif [[ "$root_pm" == "$js_pm" ]]; then
  ok
else
  bad "packageManager drifts ($root_pkg is $root_pm, $js_pkg is $js_pm; want identical, issue #912)"
fi
if [[ -n "$root_pm" ]]; then
  pnpm_ver="${root_pm#pnpm@}"
  if grep -q -F -e "pnpm $pnpm_ver" "$module"; then
    ok
  else
    bad "MODULE.bazel lost the pnpm $pnpm_ver linkage (want comment resolving the toolchain from the root packageManager pin, issue #912)"
  fi
fi

# --- Python foundation: aspect_rules_py prerelease needs a bump selector ---
py_version="$(grep -o -E -e 'bazel_dep\(name = "aspect_rules_py", version = "[^"]+"' "$module" | head -1 | grep -o -E -e '"2[^"]*"$' | tr -d '"' || true)"
if [[ -z "$py_version" ]]; then
  bad "MODULE.bazel missing aspect_rules_py pin (want bazel_dep with explicit version, issue #912)"
else
  ok
fi
if grep -q -F -e 'Bump selector: latest stable 2.x' "$module"; then
  ok
else
  bad "MODULE.bazel lost the aspect_rules_py bump selector (want 'Bump selector: latest stable 2.x' for the ADR 0008 prerelease exception, issue #912)"
fi

dx_test_summary "pin consistency"
