#!/usr/bin/env bash
# Windows EULA acknowledgement UX qualification.
#
# Qualifies the consumer-facing acknowledgement slice of the Windows
# baseline with fixture evidence, without claiming a qualified
# toolchains_msvc backend, Windows arm64, rights review, redistribution
# permission, or Supported:
# - acknowledgement: BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1
#   via repository-env for the toolchains_msvc head 8e2aa4624bbb5a53a94f135e90995f307875d1ad
#   (extensions.bzl, README advertises a different variable); deliberate
#   export plus --repo_env after reviewing terms, never automatic via
#   .bazelrc or wrapper defaults, never bypassing upstream controls;
# - fail-closed: missing acknowledgement fails with an actionable error
#   before restricted MSVC payload download (manifest reads stay
#   version-resolution cost); the error names the variable, the required
#   value, the repository-env mechanism, and the native-plan docs link;
# - unrelated-green: merely adding the module requires no acceptance and
#   fetches no restricted payloads; unrelated seed workflows stay green
#   without acceptance; deferred acceptance failure never proves laziness
#   since extension evaluation already fetches manifests;
# - out of scope: rights review itself (issue #496), redistribution
#   permission, the hermetic-llvm SDK EULA variable (also issue #496);
# - release linkage: per-host Windows release evidence stays owned under
#   issue #807; this UX is the EULA cell of that row. Backend stays
#   provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:windows_eula_qualification`,
# following //tools/ci:acquisition_rights_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/windows_eula/pins.bzl"
pins_build="cc/tests/fixtures/windows_eula/BUILD.bazel"
expected="cc/tests/fixtures/windows_eula/windows_eula.expected"
gate="cc/tests/fixtures/windows_eula/eula_gate.sh"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
targets_d="tools/ci/ci_targets_d.bzl"
freshness="tools/ci/dogfood_freshness.sh"
release_pins="tools/ci/tests/fixtures/release_windows/pins.bzl"

# Fixture set stays present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" && -f "$gate" ]]; then
  ok
else
  bad "windows EULA fixture missing (want $pins plus $pins_build plus $expected plus $gate)"
fi

# Pins record the selected upstream route identity plus extension sources.
if grep -q -F -e 'TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"' "$pins" &&
  grep -q -F -e 'TOOLCHAINS_MSVC_MODULE = "0.0.0"' "$pins" &&
  grep -q -F -e 'TOOLCHAINS_MSVC_EXTENSION = "extensions.bzl"' "$pins" &&
  grep -q -F -e 'private/vs_channel_manifest.bzl' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_VERSION = "v0.4.1"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_MSVC = "14.50.35717"' "$pins"; then
  ok
else
  bad "pins.bzl lost its toolchains_msvc plus windows_support selected-route identity under issue #818"
fi

# Pins record the acknowledgement variable plus required value plus mechanism plus README mismatch.
if grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA' "$pins" &&
  grep -q -F -e 'EULA_REQUIRED_VALUE = "1"' "$pins" &&
  grep -q -F -e 'EULA_MECHANISM = "repository-env"' "$pins" &&
  grep -q -F -e 'EULA_README_MISMATCH = True' "$pins"; then
  ok
else
  bad "pins.bzl lost its EULA variable plus required value plus repository-env mechanism plus README mismatch under issue #818"
fi

# Pins record deliberate acknowledgement, never automatic and never bypassing upstream controls.
if grep -q -F -e '--repo_env=BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1' "$pins" &&
  grep -q -F -e 'never automatic via .bazelrc or wrapper defaults' "$pins" &&
  grep -q -F -e 'never bypassing upstream controls' "$pins"; then
  ok
else
  bad "pins.bzl lost its deliberate export plus --repo_env UX with never-automatic plus never-bypass record under issue #818"
fi

# Pins record fail-before-fetch with the actionable error contract.
if grep -q -F -e 'missing acknowledgement fails before restricted MSVC payload download' "$pins" &&
  grep -q -F -e 'See docs/native-toolchains.md#windows-acquisition-and-compatibility' "$pins" &&
  grep -q -F -e 'deferred acceptance failure never proves laziness' "$pins"; then
  ok
else
  bad "pins.bzl lost its fail-before-fetch plus actionable-error plus deferred-failure record under issue #818"
fi

# Pins record unrelated-green laziness (adding the module needs nothing, extension manifests are not laziness proof).
if grep -q -F -e 'adding the module requires no acceptance' "$pins" &&
  grep -q -F -e 'fetches no restricted payloads' "$pins" &&
  grep -q -F -e 'missing acceptance leaves unrelated workflows green' "$pins" &&
  grep -q -F -e 'extension evaluation already fetches manifests' "$pins"; then
  ok
else
  bad "pins.bzl lost its unrelated-green laziness contract with manifest caveat under issue #818"
fi

# Pins record rejected substitutes plus out-of-scope rights/SDK-EULA ownership.
if grep -q -F -e '"automatic acceptance"' "$pins" &&
  grep -q -F -e '"bypassing upstream controls"' "$pins" &&
  grep -q -F -e '"installed Build Tools fallback"' "$pins" &&
  grep -q -F -e 'rights review itself under issue #496' "$pins" &&
  grep -q -F -e 'redistribution permission' "$pins" &&
  grep -q -F -e 'hermetic-llvm SDK EULA variable under issue #496' "$pins"; then
  ok
else
  bad "pins.bzl lost its rejected substitutes plus #496 out-of-scope record under issue #818"
fi

# Expected fixture pins the UX plus fail-closed plus unrelated-green plus release linkage.
if grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1 via repository-env' "$expected" &&
  grep -q -F -e 'missing acknowledgement fails before restricted MSVC payload download' "$expected" &&
  grep -q -F -e 'missing acceptance leaves unrelated workflows green' "$expected" &&
  grep -q -F -e 'windows_x86_64 sbom-provenance delivered under issue #807' "$expected" &&
  grep -q -F -e 'Qualified under issue #818' "$expected"; then
  ok
else
  bad "windows_eula.expected lost its UX plus fail-closed plus unrelated-green plus #807 linkage lines under #818"
fi

# Gate checks acknowledgement before any restricted fetch with an actionable error.
if grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA' "$gate" &&
  grep -q -F -e 'missing Microsoft EULA acknowledgement before Windows MSVC acquisition' "$gate" &&
  grep -q -F -e 'docs/native-toolchains.md#windows-acquisition-and-compatibility' "$gate" &&
  grep -q -F -e 'exit 1' "$gate"; then
  ok
else
  bad "eula_gate.sh lost its fail-before-fetch acknowledgement check with actionable error under issue #818"
fi

# Laziness proof: MODULE.bazel wires no toolchains_msvc backend, so merely
# adding the module fetches no restricted payloads on the seed host.
if ! grep -q -F -e 'toolchains_msvc' "$module"; then
  ok
else
  bad "MODULE.bazel wires toolchains_msvc (want no backend dep: adding the module must fetch nothing, issue #818)"
fi

# Laziness proof: the committed lock carries no Windows MSVC payload.
if [[ -f "$lock" ]] && ! grep -q -F -e 'toolchains_msvc' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock carries a toolchains_msvc payload (want none: unrelated workflows fetch nothing, issue #818)"
fi

# UX proof: CI sets no EULA variable and leaks no secrets in the Windows
# jobs; missing acceptance leaves unrelated workflows green
# (hermetic tree plus context search, issue #1006).
if dx_tree_absent --include='*.yml' 'BAZEL_TOOLCHAINS_MSVC_ACCEPT' -- .github/ &&
  DX_CONTEXT_RE=1 dx_context_absent .github/workflows/ci.yml 'build-windows-x86_64' -A 30 'secrets\.'; then
  ok
else
  bad "windows jobs set the EULA variable or leak secrets (want deliberate acceptance only, missing acceptance leaves unrelated workflows green, issue #818)"
fi

# Native plan owns the qualified EULA-UX record plus fixture proof with #807 linkage.
if grep -q -F -e 'qualified seed-only under issue #818' "$native" &&
  grep -q -F -e 'windows_eula_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/windows_eula/pins.bzl' "$native" &&
  grep -q -F -e 'How is Windows EULA acknowledgement required' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified EULA-UX record with fixtures under issue #818"
fi

# BUILD owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "windows_eula_qualification"' "$targets_d" &&
  grep -q -F -e 'windows_eula_qualification.sh' "$targets_d" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_eula_qualification' "$freshness"; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl or dogfood_freshness.sh lost the windows_eula_qualification wiring (want target plus freshness)"
fi

# Live proof: missing acknowledgement fails before restricted acquisition
# with an actionable error (the gate exits 1 naming the variable plus docs link).
if [[ -z "${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}" ]]; then
  gate_out="$(bash "$gate" 2>&1 || true)"
  if echo "$gate_out" | grep -q -F -e 'BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA' &&
    echo "$gate_out" | grep -q -F -e 'docs/native-toolchains.md#windows-acquisition-and-compatibility' &&
    ! BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA= bash "$gate" >/dev/null 2>&1; then
    ok
  else
    bad "EULA gate did not fail closed with an actionable error on missing acknowledgement (want var plus docs link before fetch, issue #818)"
  fi
else
  bad "EULA variable is set in this environment (want unset: missing-ack must fail before fetch, issue #818)"
fi

# Live proof: missing acceptance leaves unrelated workflows green (the seed
# hello builds with the EULA variable unset, fetching no Windows payloads).
if [[ -z "${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}" ]] &&
  bazel build //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello failed without EULA acceptance (want unrelated workflows green with missing acceptance, issue #818)"
fi

# Release linkage: the #807 per-host fixture keeps its EULA cell, so this UX
# lands in the release-evidence row rather than as a parallel claim.
if grep -q -F -e 'explicit EULA acceptance required never automatic' "$release_pins"; then
  ok
else
  bad "release-windows pins lost the explicit-EULA cell linked into the #807 row (want EULA linkage, issue #818)"
fi

dx_test_summary "windows EULA acknowledgement harness"
