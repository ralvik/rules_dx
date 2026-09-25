#!/usr/bin/env bash
# Pin single-source consistency.
#
# Canonical sources:
#   Bazel version: `.bazelversion` (Bazelisk reads it; every other Bazel pin
#     tracks it).
#   Bazelisk version + per-OS sha256: `.github/actions/setup-bazelisk/action.yml`
# defaults (the single portable installer,; Dockerfile tracks
#     the linux-amd64 pair and docs bootstrap tracks all five hosts).
#   Module pins: `modules/*.bzl` wrappers own per-ecosystem pins
#     (rust, python, js, java-scala-kotlin, dotnet, toolchains) plus the
#     crate-manifest groups; `MODULE.bazel` keeps only `bazel_dep` plus
#     extension use plus `use_repo` re-exports and must match every wrapper.
#   Tested stack: `libs/testing/tested_stack.bzl` carries the full MODULE
#     dep map (generated from `bazel mod deps --depth=1 --format=json`);
#     the preset pins plus `.bazelversion` plus the support matrix track it.
#   Preset fragment: `tools/bazelrc/src/lib.rs` inventory owns
#     `tools/bazelrc/preset.bazelrc`; this test enforces the same invariant
#     as `preset.update --verify-only` (flag lines plus counts plus pins).
#   Tool repos: `quality/artifacts/repos.bzl` owns the `dx_tools`
#     `use_repo` inventory; `//quality/artifacts:metadata` proves it
#     against metadata and this test proves MODULE.bazel against it.
#   JVM tool repos: `quality/tools/jvm/repos.bzl` owns the `jvm_tools`
#     `use_repo` inventory plus artifact metadata (issue #1038); this test
#     proves MODULE.bazel against it and proves the versions against
#     `modules/java-scala-kotlin.bzl` `JVM_TOOL_VERSIONS`.
#   Go toolchain: `modules/toolchains.bzl` `GO_SDK_VERSION` owns the
#     toolchain floor (MODULE.bazel `go_sdk.download` mirrors it);
#     `GO_LANGUAGE_FLOOR` owns the language floor
#     (`third_party/go/go.mod` `go` directive mirrors it and tracks
#     gazelle 0.52.2). SDK minor must stay >= go.mod minor
#     (issues #912, #1003).
#   pnpm version: root `package.json` `packageManager` owns the pnpm pin;
#     `quality/tools/javascript/package.json` must match and `MODULE.bazel`
#     resolves the toolchain from the root pin (issue #912).
#   Python foundation: `MODULE.bazel` `aspect_rules_py` prerelease stays an
#     ADR 0008 exception with an explicit bump selector (issue #912).
#   Single version (issue #931): `MODULE.bazel` `version` owns the
#     delivered `dx` == module pin; `cli/adopt/src/version.rs`
#     (`DX_VERSION`, `MODULE_VERSION`, `PREVIOUS_VERSION`),
#     `tools/bazelrc/src/lib.rs` (`PRESET_DX_VERSION`), and
#     `.github/workflows/ghcr.yml` tag prefixes (`-ci-` plus `-sha-`)
#     must equal it in the same reviewed PR.
#
# Every other pin below must equal its canonical source or this fails, so a
# version bump means: bump the canonical file once, then update the tracked
# copies in the same reviewed change.
#
# Usage: pin_consistency.sh <bazelversion> <module> <preset_rs>
#   <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod>
#   <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js>
#   <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains>
#   <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>
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
js_pkg="${10:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_rust="${11:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_python="${12:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_js="${13:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_jvm="${14:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_dotnet="${15:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_hubs="${16:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
modules_toolchains="${17:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc>}"
repos_bzl="${18:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>}"
preset_fragment="${19:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>}"
root_bazelrc="${20:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>}"
version_rs="${21:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>}"
ghcr_yml="${22:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml>}"
jvm_repos_bzl="${23:?usage: pin_consistency.sh <bazelversion> <module> <preset_rs> <dockerfile> <tested_stack> <action_yml> <local_workflows> <go_mod> <root_pkg> <js_pkg> <modules_rust> <modules_python> <modules_js> <modules_jvm> <modules_dotnet> <modules_hubs> <modules_toolchains> <repos_bzl> <preset_fragment> <root_bazelrc> <version_rs> <ghcr_yml> <jvm_repos_bzl>}"

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
# Narrow to the go_sdk block: the first version pin inside go_sdk.download.
sdk_version="$(sed -n '/go_sdk.download(/,/)/p' "$module" | grep -o -E -e 'version = "[^"]+"' | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
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

# --- modules/ split: wrapper pins must equal MODULE.bazel pins ---
mod_pin() { # file, CONSTANT -> version on stdout
  grep -o -E -e "^${2} = \"[^\"]+\"" "$1" | head -1 | cut -d'"' -f2 || true
}

check_dep_pin() { # wrapper-file, CONSTANT, module-name
  local want
  want="$(mod_pin "$1" "$2")"
  if [[ -z "$want" ]]; then
    bad "$1 lost $2 (want a top-level string pin)"
  elif grep -q -F -e "bazel_dep(name = \"$3\", version = \"$want\")" "$module"; then
    ok
  else
    bad "MODULE.bazel drifts from $1 $2=$want (want bazel_dep $3 at $want)"
  fi
}

check_dep_pin "$modules_rust" RULES_RUST_VERSION rules_rust
check_dep_pin "$modules_rust" RULES_RUST_PROST_VERSION rules_rust_prost
check_dep_pin "$modules_toolchains" RULES_CC_VERSION rules_cc
check_dep_pin "$modules_toolchains" GOOGLETEST_VERSION googletest
check_dep_pin "$modules_python" RULES_PYTHON_VERSION rules_python
check_dep_pin "$modules_toolchains" RULES_GO_VERSION rules_go
check_dep_pin "$modules_toolchains" GAZELLE_VERSION gazelle
check_dep_pin "$modules_toolchains" RULES_SHELL_VERSION rules_shell
check_dep_pin "$modules_toolchains" PLATFORMS_VERSION platforms
check_dep_pin "$modules_toolchains" BAZEL_SKYLIB_VERSION bazel_skylib
check_dep_pin "$modules_toolchains" RULES_PROTO_VERSION rules_proto
check_dep_pin "$modules_jvm" RULES_JAVA_VERSION rules_java
check_dep_pin "$modules_jvm" RULES_KOTLIN_VERSION rules_kotlin
check_dep_pin "$modules_jvm" RULES_SCALA_VERSION rules_scala
check_dep_pin "$modules_dotnet" RULES_DOTNET_VERSION rules_dotnet
check_dep_pin "$modules_dotnet" BAZEL_LIB_VERSION bazel_lib
check_dep_pin "$modules_jvm" RULES_JVM_EXTERNAL_VERSION rules_jvm_external
check_dep_pin "$modules_python" ASPECT_RULES_PY_VERSION aspect_rules_py
check_dep_pin "$modules_js" ASPECT_RULES_JS_VERSION aspect_rules_js
check_dep_pin "$modules_js" ASPECT_RULES_TS_VERSION aspect_rules_ts
check_dep_pin "$modules_js" ASPECT_RULES_JEST_VERSION aspect_rules_jest

# Extension-level pins (non-bazel_dep call sites).
rust_ver="$(mod_pin "$modules_rust" RUST_VERSION)"
if grep -q -F -e "versions = [\"$rust_ver\"]" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/rust.bzl RUST_VERSION=$rust_ver (want rust.toolchain versions)"
fi
rustfmt_ver="$(mod_pin "$modules_rust" RUSTFMT_VERSION)"
if grep -q -F -e "rustfmt_version = \"$rustfmt_ver\"" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/rust.bzl RUSTFMT_VERSION=$rustfmt_ver"
fi
for triple in x86_64-unknown-linux-musl aarch64-unknown-linux-musl; do
  if grep -q -F -e "\"$triple\"" "$modules_rust" && grep -q -F -e "\"$triple\"" "$module"; then
    ok
  else
    bad "musl triple $triple lost from modules/rust.bzl or MODULE.bazel rust.toolchain"
  fi
done
scala_ver="$(mod_pin "$modules_jvm" SCALA_VERSION)"
if grep -q -F -e "scala_config.settings(scala_version = \"$scala_ver\")" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/java-scala-kotlin.bzl SCALA_VERSION=$scala_ver"
fi
dotnet_ver="$(mod_pin "$modules_dotnet" DOTNET_VERSION)"
if grep -q -F -e "dotnet.toolchain(dotnet_version = \"$dotnet_ver\")" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/dotnet.bzl DOTNET_VERSION=$dotnet_ver"
fi
ts_ver="$(mod_pin "$modules_js" TYPESCRIPT_VERSION)"
ts_integrity="$(mod_pin "$modules_js" TYPESCRIPT_INTEGRITY)"
if grep -q -F -e "version = \"$ts_ver\"" "$module" && grep -q -F -e "$ts_integrity" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/js.bzl TYPESCRIPT_VERSION=$ts_ver plus TYPESCRIPT_INTEGRITY (want typescript.deps version plus integrity)"
fi
if grep -q -F -e 'integrity = "sha512-' "$module"; then
  ok
else
  bad "MODULE.bazel lost the TypeScript SRI pin (want typescript.deps integrity sha512, issue #954)"
fi
py_interp="$(mod_pin "$modules_python" PYTHON_VERSION)"
if grep -q -F -e "python.toolchain(python_version = \"$py_interp\")" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/python.bzl PYTHON_VERSION=$py_interp"
fi
sdk_ver="$(mod_pin "$modules_toolchains" GO_SDK_VERSION)"
if grep -q -F -e "version = \"$sdk_ver\"" "$module" && grep -q -F -e "go_sdk.download(" "$module"; then
  ok
else
  bad "MODULE.bazel drifts from modules/toolchains.bzl GO_SDK_VERSION=$sdk_ver"
fi
# Go SDK archive pins (issue #954): every GO_SDK_SDKS entry must mirror
# MODULE.bazel sdks so the sha trust anchor never drifts from the wrapper.
for platform in darwin_amd64 darwin_arm64 linux_amd64 linux_arm64 windows_amd64 windows_arm64; do
  want_sha="$(grep -A30 -F -e 'GO_SDK_SDKS = {' "$modules_toolchains" | grep -F -e "\"$platform\"" | grep -o -E -e '[0-9a-f]{64}' | head -1 || true)"
  if [[ -z "$want_sha" ]]; then
    bad "modules/toolchains.bzl GO_SDK_SDKS lost $platform (want six pinned archives, issue #954)"
  elif grep -q -F -e "$want_sha" "$module"; then
    ok
  else
    bad "MODULE.bazel sdks drifts from modules/toolchains.bzl GO_SDK_SDKS $platform=$want_sha (want mirrored sha, issue #954)"
  fi
done
if grep -q -F -e "go1.26.6.linux-amd64.tar.gz" "$module" && grep -q -F -e "go1.26.6.windows-amd64.zip" "$module"; then
  ok
else
  bad "MODULE.bazel lost the Go SDK archive filenames (want pinned sdks filenames, issue #954)"
fi
go_floor="$(mod_pin "$modules_toolchains" GO_LANGUAGE_FLOOR)"
go_mod_ver="$(grep -o -E -e '^go [0-9]+\.[0-9]+(\.[0-9]+)?' "$go_mod" | head -1 | cut -d' ' -f2 || true)"
if [[ "$go_floor" == "$go_mod_ver" ]]; then
  ok
else
  bad "modules/toolchains.bzl GO_LANGUAGE_FLOOR=$go_floor drifts from third_party/go/go.mod $go_mod_ver"
fi
pnpm_mod="$(mod_pin "$modules_js" PNPM_VERSION)"
if [[ -n "$root_pm" && "$pnpm_mod" == "${root_pm#pnpm@}" ]]; then
  ok
else
  bad "modules/js.bzl PNPM_VERSION=$pnpm_mod drifts from root packageManager $root_pm"
fi

# --- Maven availability plus fail-closed lock (issue #954): dual origins
# pin availability while lock hashes pin identity. MODULE must keep both
# Central origins plus fail_if_repin_required, so a single-origin drift
# fails here instead of silently losing redundancy.
if grep -q -F -e "https://maven-central.storage-download.googleapis.com/maven2" "$module" && grep -q -F -e "https://repo1.maven.org/maven2" "$module"; then
  ok
else
  bad "MODULE.bazel lost Maven dual origins (want GCS mirror plus repo1, issue #954)"
fi
if grep -q -F -e "fail_if_repin_required = True" "$module" && grep -q -F -e 'lock_file = "//third_party/jvm:maven_install.json"' "$module"; then
  ok
else
  bad "MODULE.bazel lost the Maven fail-closed lock (want lock_file plus fail_if_repin_required, issue #954)"
fi

# Crate-manifest groups: every wrapper manifest must mirror MODULE.bazel
# exactly (MODULE files cannot load wrappers). Label existence is enforced
# by Bazel itself at MODULE evaluation (a missing label fails the build).
crate_labels="$(grep -o -E -e '"//[^"]*:Cargo.toml"' "$modules_rust" | tr -d '"' || true)"
if [[ -z "$crate_labels" ]]; then
  bad "modules/rust.bzl carries no crate manifests"
else
  ok
fi
module_manifests="$(sed -n '/crate.from_cargo(/,/^)/p' "$module" | grep -o -E -e '"//[^"]*:Cargo.toml"' | tr -d '"' | LC_ALL=C sort -u || true)"
wrapper_manifests="$(echo "$crate_labels" | LC_ALL=C sort -u || true)"
if [[ -n "$module_manifests" && "$module_manifests" == "$wrapper_manifests" ]]; then
  ok
else
  bad "MODULE.bazel crate manifests drift from modules/rust.bzl groups (mirror both ways)"
fi
dupes="$(echo "$crate_labels" | LC_ALL=C sort | uniq -d | wc -l | tr -d ' ')"
if [[ "$dupes" == "0" ]]; then
  ok
else
  bad "modules/rust.bzl lists a crate manifest twice"
fi
for group in CRATE_FIXTURE_MANIFESTS CRATE_CLI_MANIFESTS CRATE_DEPLOY_MANIFESTS CRATE_SHARD_WRITER_MANIFESTS CRATE_SHARED_MANIFESTS; do
  if grep -q -F -e "$group = [" "$modules_rust"; then
    ok
  else
    bad "modules/rust.bzl lost manifest group $group"
  fi
done

# --- Tool repos: MODULE.bazel use_repo must equal repos.bzl inventory ---
want_repos="$(grep -o -E -e '"dx_[a-z0-9_]+"' "$repos_bzl" | tr -d '"' | LC_ALL=C sort -u || true)"
have_repos="$(sed -n '/^use_repo($/,/^)/p' "$module" | grep -o -E -e '"dx_[a-z0-9_]+"' | tr -d '"' | LC_ALL=C sort -u || true)"
if [[ -n "$want_repos" && "$want_repos" == "$have_repos" ]]; then
  ok
else
  bad "MODULE.bazel dx_tools use_repo drifts from quality/artifacts/repos.bzl DX_TOOL_REPOS"
fi

# --- JVM tool repos (issue #1038): MODULE jvm_tools use_repo must equal repos.bzl ---
want_jvm_repos="$(grep -o -E -e '"jvm_[a-z0-9_]+"' "$jvm_repos_bzl" | tr -d '"' | LC_ALL=C sort -u || true)"
have_jvm_repos="$(sed -n '/^use_repo($/,/^)/p' "$module" | grep -o -E -e '"jvm_[a-z0-9_]+"' | tr -d '"' | LC_ALL=C sort -u || true)"
if [[ -n "$want_jvm_repos" && "$want_jvm_repos" == "$have_jvm_repos" ]]; then
  ok
else
  bad "MODULE.bazel jvm_tools use_repo drifts from quality/tools/jvm/repos.bzl JVM_TOOL_REPOS"
fi

# JVM acquisition stays unified under the lazy extension (issue #1038):
# no inline http_file/http_archive for jvm_ repos in MODULE.bazel.
if grep -E -e '^(http_file|http_archive)\(' "$module" | grep -q .; then
  bad "MODULE.bazel carries inline http_file/http_archive (want JVM tools via jvm_tools extension only, issue #1038)"
else
  ok
fi
if grep -q -F -e 'use_repo_rule("@bazel_tools//tools/build_defs/repo:http.bzl"' "$module"; then
  bad "MODULE.bazel carries use_repo_rule http (want JVM tools via jvm_tools extension only, issue #1038)"
else
  ok
fi
if grep -q -F -e 'jvm_tools = use_extension("//quality/tools/jvm:extension.bzl", "jvm_tools")' "$module"; then
  ok
else
  bad "MODULE.bazel lost the jvm_tools extension (want use_extension //quality/tools/jvm:extension.bzl, issue #1038)"
fi

# JVM versions: every modules/java-scala-kotlin.bzl JVM_TOOL_VERSIONS entry
# must appear in quality/tools/jvm/repos.bzl artifact URLs.
for ver in 1.35.0 14.1.0 7.27.0 4.10.4 0.63 1.8.0; do
  if grep -q -F -e "$ver" "$jvm_repos_bzl"; then
    ok
  else
    bad "quality/tools/jvm/repos.bzl lost JVM version $ver (want mirror of modules/java-scala-kotlin.bzl JVM_TOOL_VERSIONS, issue #1038)"
  fi
done
# JVM digests stay single-sourced in repos.bzl, never duplicated in MODULE.
for sha in bfb7f9ead6cd328389bc2da53860443bc0e805dfd08cc889bfdf43b26cb2a6e8 51e2bc7fed1bb56808aa39045f655a316194997acd24bac5195253dcf342b380 4ae396ffaf2b0d3ef0b73a10b2925e77066f73d57a4ce9078c60e7302bcddec9 72bc0d4edd686e462c0f71f42a049b27bf4da6708797ff7b2b56dd202714b4e5 a015521ddb1c7a80c41edb56b91b4a231439592ffd2e85ac866ff8134c37c112 369ad2b789f95a011f807e1fcb690ccef80bd7cd014fd139e73ae82dcc0baeab; do
  if grep -q -F -e "$sha" "$jvm_repos_bzl"; then
    ok
  else
    bad "quality/tools/jvm/repos.bzl lost JVM digest $sha (want single-sourced digests, issue #1038)"
  fi
  if grep -q -F -e "$sha" "$module"; then
    bad "MODULE.bazel duplicates JVM digest $sha (want digests single-sourced in quality/tools/jvm/repos.bzl, issue #1038)"
  else
    ok
  fi
done

# --- Shell-env policy: third-party pin stays False with zero opt-ins ---
if grep -q -F -e 'build --@rules_rust//cargo/settings:use_default_shell_env=False' "$root_bazelrc"; then
  ok
else
  bad ".bazelrc lost the hermetic build-script pin (want use_default_shell_env=False; first-party default lives in gazelle/rust/lang_generate.go)"
fi
if grep -v -E -e '^\s*#' "$module" | grep -q -F -e 'build_script_use_default_shell_env = "on"'; then
  bad "MODULE.bazel gained a shell-env opt-in (narrowly allowed only with a reviewed policy update in modules/rust.bzl)"
else
  ok
fi

# --- Preset verify-only: fragment must equal the lib.rs inventory ---
preset_flags="$(grep -o -E -e '^    "[a-z_:]+ [^"]+"' "$preset_py" | sed -e 's/^    "//' -e 's/"$//' || true)"
preset_flag_count="$(echo "$preset_flags" | grep -c . || true)"
fragment_flags="$(grep -v -E -e '^#|^$' "$preset_fragment" || true)"
fragment_flag_count="$(echo "$fragment_flags" | grep -c . || true)"
if [[ "$preset_flag_count" == "$fragment_flag_count" ]] && [[ "$preset_flag_count" == "16" ]]; then
  ok
else
  bad "preset flag drift (lib.rs has $preset_flag_count, preset.bazelrc has $fragment_flag_count; want 16 each: run preset.update)"
fi
stale_preset=0
while IFS= read -r flag; do
  [[ -n "$flag" ]] || continue
  if ! grep -q -F -e "$flag" "$preset_fragment"; then
    bad "preset.bazelrc stale: missing inventory flag $flag (run preset.update)"
    stale_preset=1
  fi
done <<<"$preset_flags"
if [[ "$stale_preset" == "0" ]]; then
  ok
fi
dx_ver="$(grep -o -E -e 'PRESET_DX_VERSION[^"]*"[^"]+"' "$preset_py" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
module_ver="$(grep -o -E -e '^    version = "[^"]+"' "$module" | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$dx_ver" && "$dx_ver" == "$module_ver" ]]; then
  ok
else
  bad "preset PRESET_DX_VERSION=$dx_ver drifts from MODULE.bazel version=$module_ver"
fi

# --- Single-version atomic (issue #931): MODULE == adopt DX/MODULE/PREVIOUS == preset == GHCR prefix ---
adopt_dx="$(grep -o -E -e 'pub const DX_VERSION[^"]*"[^"]+"' "$version_rs" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
adopt_module="$(grep -o -E -e 'pub const MODULE_VERSION[^"]*"[^"]+"' "$version_rs" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
adopt_prev="$(grep -o -E -e 'pub const PREVIOUS_VERSION[^"]*"[^"]+"' "$version_rs" | head -1 | grep -o -E -e '"[^"]+"$' | tr -d '"' || true)"
if [[ -n "$adopt_dx" && "$adopt_dx" == "$module_ver" ]]; then
  ok
else
  bad "adopt DX_VERSION=$adopt_dx drifts from MODULE.bazel version=$module_ver (want single-version atomic, issue #931)"
fi
if [[ -n "$adopt_module" && "$adopt_module" == "$module_ver" ]]; then
  ok
else
  bad "adopt MODULE_VERSION=$adopt_module drifts from MODULE.bazel version=$module_ver (want single-version atomic, issue #931)"
fi
# PREVIOUS tracks the prior release: at 0.0.0 (no releases cut) it equals
# the module pin so rollback correctly refuses; after the first SemVer flip
# it must differ (previous release) while staying valid semver. Either way
# it must be present and parse as semver, and a drift to non-semver fails.
if [[ -z "$adopt_prev" ]]; then
  bad "adopt PREVIOUS_VERSION is empty (want single-version atomic, issue #931)"
elif [[ "$module_ver" == "0.0.0" ]]; then
  if [[ "$adopt_prev" == "$module_ver" ]]; then
    ok
  else
    bad "adopt PREVIOUS_VERSION=$adopt_prev drifts from MODULE.bazel version=$module_ver at 0.0.0 (want equal until first release, issue #931)"
  fi
else
  if [[ "$adopt_prev" != "$module_ver" ]]; then
    ok
  else
    bad "adopt PREVIOUS_VERSION=$adopt_prev equals MODULE.bazel version=$module_ver after first release (want previous release, issue #931)"
  fi
fi
ghcr_ci_ver="$(grep -o -E -e 'devcontainer:[0-9]+\.[0-9]+\.[0-9]+-ci-' "$ghcr_yml" | head -1 | sed -E 's/.*devcontainer:([0-9]+\.[0-9]+\.[0-9]+)-ci-.*/\1/' || true)"
ghcr_sha_ver="$(grep -o -E -e 'devcontainer:[0-9]+\.[0-9]+\.[0-9]+-sha-' "$ghcr_yml" | head -1 | sed -E 's/.*devcontainer:([0-9]+\.[0-9]+\.[0-9]+)-sha-.*/\1/' || true)"
if [[ -n "$ghcr_ci_ver" && "$ghcr_ci_ver" == "$module_ver" ]]; then
  ok
else
  bad "ghcr.yml ci tag prefix $ghcr_ci_ver drifts from MODULE.bazel version=$module_ver (want single-version atomic, issue #931)"
fi
if [[ -n "$ghcr_sha_ver" && "$ghcr_sha_ver" == "$module_ver" ]]; then
  ok
else
  bad "ghcr.yml sha tag prefix $ghcr_sha_ver drifts from MODULE.bazel version=$module_ver (want single-version atomic, issue #931)"
fi

# --- Tested stack: full MODULE dep map must match ---
while IFS= read -r dep_line; do
  [[ -n "$dep_line" ]] || continue
  dep_name="$(echo "$dep_line" | cut -d'"' -f2)"
  dep_ver="$(echo "$dep_line" | cut -d'"' -f4)"
  if grep -q -F -e "bazel_dep(name = \"$dep_name\", version = \"$dep_ver\")" "$module"; then
    ok
  else
    bad "tested_stack.bzl $dep_name=$dep_ver drifts from MODULE.bazel"
  fi
done <<<"$(grep -o -E -e '^    "[a-z_0-9]+": "[^"]+"' "$tested_stack" || true)"
stack_dotnet="$(grep -A2 -F -e '"dotnet_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_dotnet" && "$stack_dotnet" == "$dotnet_ver" ]]; then
  ok
else
  bad "tested_stack.bzl dotnet_version=$stack_dotnet drifts from modules/dotnet.bzl DOTNET_VERSION=$dotnet_ver"
fi
stack_go_sdk="$(grep -A2 -F -e '"go_sdk_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_go_sdk" && "$stack_go_sdk" == "$sdk_ver" ]]; then
  ok
else
  bad "tested_stack.bzl go_sdk_version=$stack_go_sdk drifts from modules/toolchains.bzl GO_SDK_VERSION=$sdk_ver"
fi
stack_pnpm="$(grep -A2 -F -e '"pnpm_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_pnpm" && "$stack_pnpm" == "$pnpm_mod" ]]; then
  ok
else
  bad "tested_stack.bzl pnpm_version=$stack_pnpm drifts from modules/js.bzl PNPM_VERSION=$pnpm_mod"
fi
stack_python="$(grep -A2 -F -e '"python_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_python" && "$stack_python" == "$py_interp" ]]; then
  ok
else
  bad "tested_stack.bzl python_version=$stack_python drifts from modules/python.bzl PYTHON_VERSION=$py_interp"
fi
stack_scala="$(grep -A2 -F -e '"scala_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_scala" && "$stack_scala" == "$scala_ver" ]]; then
  ok
else
  bad "tested_stack.bzl scala_version=$stack_scala drifts from modules/java-scala-kotlin.bzl SCALA_VERSION=$scala_ver"
fi
stack_ts="$(grep -A2 -F -e '"typescript_version": attr.string(' "$tested_stack" | grep -o -E -e 'default = "[^"]+"' | head -1 | cut -d'"' -f2 || true)"
if [[ -n "$stack_ts" && "$stack_ts" == "$ts_ver" ]]; then
  ok
else
  bad "tested_stack.bzl typescript_version=$stack_ts drifts from modules/js.bzl TYPESCRIPT_VERSION=$ts_ver"
fi

dx_test_summary "pin consistency"
