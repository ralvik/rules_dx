#!/usr/bin/env bash
# Windows immutable-lazy acquisition qualification.
#
# Qualifies the acquisition-mechanism slice of the Windows baseline with
# fixture evidence, without claiming a qualified toolchains_msvc backend,
# Windows arm64, or Supported:
# - immutable: narrow upstream fixed-manifest plus package-index inputs
#   pinned in `cc/tests/fixtures/windows_acquisition/pins.bzl` (toolchains_msvc
#   head `8e2aa4624bbb5a53a94f135e90995f307875d1ad` with module `0.0.0`, never
#   a release; windows_support `v0.4.1` with MSVC `14.50.35717` plus redist
#   `14.50.35710` plus SDK package `10.0.26100.7705` tracked separately).
#   Mutable fetch stays rejected: live VS channel manifests, minor-version
#   selectors, individual payload checksums only, pinned rules commit only.
# - lazy: merely adding the module requires no acceptance and fetches no
#   restricted payloads; unrelated seed workflows fetch no Windows payloads
#   and stay green without acceptance. Deferred acceptance failure does not
#   prove laziness: extension evaluation already fetches manifests.
# - open with honest records: full toolchains_msvc backend, Microsoft
# acquisition/cache rights, transport plus ABI,
# prebuilt interop, corpus plus floors plus coverage (issues
# //), release evidence. Backend stays provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:windows_acquisition_qualification`,
# following //tools/ci:fsharplint_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/windows_acquisition/pins.bzl"
pins_build="cc/tests/fixtures/windows_acquisition/BUILD.bazel"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture pair stays present.
if [[ -f "$pins" && -f "$pins_build" ]]; then
  ok
else
  bad "windows acquisition fixture missing (want $pins plus $pins_build)"
fi

# Pins record the toolchains_msvc prototype identity plus sources.
if grep -q -F -e 'TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"' "$pins" &&
  grep -q -F -e 'TOOLCHAINS_MSVC_MODULE = "0.0.0"' "$pins" &&
  grep -q -F -e 'private/vs_channel_manifest.bzl' "$pins" &&
  grep -q -F -e 'extensions.bzl' "$pins" &&
  grep -q -F -e 'private/msvc_toolchains_repo.bzl' "$pins" &&
  grep -q -F -e 'overlays/toolchain/clang-cl/BUILD.toolchain.tpl' "$pins"; then
  ok
else
  bad "pins.bzl lost its toolchains_msvc prototype identity plus sources under issue #495"
fi

# Pins record the windows_support building-block identity plus packages.
if grep -q -F -e 'WINDOWS_SUPPORT_VERSION = "v0.4.1"' "$pins" &&
  grep -q -F -e '43dce21da77d8cb4a486709e34f1d887e69058c6' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_MSVC = "14.50.35717"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_REDIST = "14.50.35710"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"' "$pins"; then
  ok
else
  bad "pins.bzl lost its windows_support v0.4.1 plus MSVC/redist/SDK identities under issue #495"
fi

# Pins record the deliberate EULA mechanism plus the README mismatch.
if grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA' "$pins" &&
  grep -q -F -e 'EULA_MECHANISM = "repository-env"' "$pins" &&
  grep -q -F -e 'EULA_README_MISMATCH = True' "$pins" &&
  grep -q -F -e 'never automatic' "$pins"; then
  ok
else
  bad "pins.bzl lost its deliberate EULA repository-env mechanism plus README mismatch under issue #495"
fi

# Pins record immutable inputs with mutable fetch rejected.
if grep -q -F -e '"fixed-manifest"' "$pins" &&
  grep -q -F -e '"package-index"' "$pins" &&
  grep -q -F -e '"mutable fetch"' "$pins" &&
  grep -q -F -e 'live VS channel manifests' "$pins" &&
  grep -q -F -e 'minor-version selectors' "$pins" &&
  grep -q -F -e 'individual payload checksums only' "$pins" &&
  grep -q -F -e 'pinned rules commit only' "$pins"; then
  ok
else
  bad "pins.bzl lost its immutable fixed-manifest/package-index inputs plus mutable-fetch rejection under issue #495"
fi

# Pins record the laziness contract (deferred failure is not laziness proof).
if grep -q -F -e 'adding the module requires no acceptance' "$pins" &&
  grep -q -F -e 'fetches no restricted payloads' "$pins" &&
  grep -q -F -e 'unrelated workflows fetch no Windows payloads' "$pins" &&
  grep -q -F -e 'extension' "$pins" &&
  grep -q -F -e 'already fetches manifests' "$pins" &&
  grep -q -F -e '"project-owned downloader"' "$pins" &&
  grep -q -F -e '"cross-host Windows route"' "$pins"; then
  ok
else
  bad "pins.bzl lost its laziness contract plus downloader/cross-host rejection under issue #495"
fi

# Laziness proof: MODULE.bazel wires no toolchains_msvc backend, so merely
# adding the module fetches no restricted payloads on the seed host.
if ! grep -q -F -e 'toolchains_msvc' "$module"; then
  ok
else
  bad "MODULE.bazel wires toolchains_msvc (want no backend dep: adding the module must fetch nothing, issue #495)"
fi

# Laziness proof: the committed lock carries no Windows MSVC payload.
if [[ -f "$lock" ]] && ! grep -q -F -e 'toolchains_msvc' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock carries a toolchains_msvc payload (want none: unrelated workflows fetch nothing, issue #495)"
fi

# Laziness proof: CI sets no EULA variable and leaks no secrets in the
# Windows jobs; missing acceptance leaves unrelated workflows green.
if ! grep -rn -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT' --include='*.yml' .github/ 2>/dev/null | grep -v -F -e 'windows_acquisition_qualification.sh' | grep -v -F -e 'windows_qualification.sh' | grep -q . &&
  ! grep -A30 -e 'build-windows-x86_64' "$ci" | grep -E -e 'secrets\.' | grep -q .; then
  ok
else
  bad "windows jobs set the EULA variable or leak secrets (want deliberate acceptance only, missing acceptance leaves unrelated workflows green, issue #495)"
fi

# Native plan owns the qualified immutable-lazy record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #495' "$native" &&
  grep -q -F -e 'windows_acquisition_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/windows_acquisition/pins.bzl' "$native" &&
  grep -q -F -e 'Can Windows acquisition be immutable and lazy?' "$native" &&
  grep -q -F -e 'mutable fetch rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified immutable-lazy record with fixtures under issue #495"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "windows_acquisition_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_acquisition_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the windows_acquisition_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: missing acceptance leaves unrelated workflows green (the seed
# hello builds with the EULA variable unset, fetching no Windows payloads).
if [[ -z "${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}" ]] &&
  bazel build //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello failed without EULA acceptance (want unrelated workflows green with missing acceptance, issue #495)"
fi

dx_test_summary "windows acquisition qualification harness"
