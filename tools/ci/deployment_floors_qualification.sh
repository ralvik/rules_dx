#!/usr/bin/env bash
# Deployment plus execution floors qualification harness.
#
# Defines plus proves the floors slice of the native baseline with fixture
# evidence, without claiming a qualified hermetic-llvm backend, qualified
# cross routes, qualified coverage, or Supported:
# - Linux glibc: upstream glibc 2.28 symbol floor with libc++ plus
#   ordinary dynamic glibc linkage; application implementations come from
#   deployment systems, not the link stubs.
# - Linux musl: upstream musl 1.2.6 static native closure, non-PIE first
#   for Rust compatibility; dynamic/shared musl stays excluded with no
#   cell and no coverage.
# - macOS: pinned acquired Apple SDK MacOSX26.5 via hermetic-llvm v0.8.19
#   plus upstream deployment default 14.0 as the starting point; SDK
#   version is not deployment floor; oldest-OS execution plus framework
#   completeness plus licensing remain gates; best-effort gaps never
#   block required-host release.
# - Windows: retail dynamic CRT /MD with Microsoft STL plus UCRT plus
#   VCRuntime; MSVC 14.50.35717 plus redist 14.50.35710 plus SDK package
#   10.0.26100.7705 tracked separately; /MT plus debug CRT never
#   interchangeable.
# - separation: oldest-target and current-host fixtures run separately,
#   never as one merged proof; cross-building alone is insufficient
#   without matching native target execution.
# - loaders: compiler, clangd and bindgen loader dependencies inspected
#   separately; the minimal compiler archive alone is not proof that a
#   loadable libclang exists; Apple extracted SDK framework subset
#   checked for completeness.
# - unpinned floors rejected. Backends stay provisional; linux corpus
# qualified seed-only, coverage qualified seed-only
#
#
# Versioned here, run by CI via `bazel run //tools/ci:deployment_floors_qualification`,
# following //tools/ci:linux_corpus_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/deployment_floors/pins.bzl"
pins_build="cc/tests/fixtures/deployment_floors/BUILD.bazel"
floors_expected="cc/tests/fixtures/deployment_floors/floors.expected"
oldest="cc/tests/fixtures/deployment_floors/oldest_target.txt"
current="cc/tests/fixtures/deployment_floors/current_host.txt"
loaders="cc/tests/fixtures/deployment_floors/loader_deps.txt"
frameworks="cc/tests/fixtures/deployment_floors/apple_frameworks.txt"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$floors_expected" && -f "$oldest" && -f "$current" && -f "$loaders" && -f "$frameworks" ]]; then
  ok
else
  bad "deployment floors fixture missing (want $pins plus $pins_build plus floors.expected plus oldest_target plus current_host plus loader_deps plus apple_frameworks)"
fi

# Pins record the Linux glibc 2.28 symbol floor with libc++ plus dynamic linkage.
if grep -q -F -e 'GLIBC_FLOOR = "2.28"' "$pins" &&
  grep -q -F -e 'Upstream glibc 2.28 symbol floor' "$pins" &&
  grep -q -F -e 'ordinary dynamic glibc linkage' "$pins" &&
  grep -q -F -e 'Application glibc implementations come from deployment systems, not the link stubs' "$pins"; then
  ok
else
  bad "pins.bzl lost its glibc 2.28 symbol floor plus libc++ plus dynamic linkage under issue #500"
fi

# Pins record the Linux musl 1.2.6 static closure with non-PIE first plus dynamic exclusion.
if grep -q -F -e 'MUSL_VERSION = "1.2.6"' "$pins" &&
  grep -q -F -e 'static native closure' "$pins" &&
  grep -q -F -e 'non-PIE first for Rust compatibility' "$pins" &&
  grep -q -F -e 'Dynamic/shared musl stays excluded with no cell and no coverage' "$pins"; then
  ok
else
  bad "pins.bzl lost its musl 1.2.6 static closure plus non-PIE plus dynamic exclusion under issue #500"
fi

# Pins record the macOS deployment 14.0 starting point with SDK != floor plus MacOSX26.5.
if grep -q -F -e 'MACOS_DEPLOYMENT_DEFAULT = "14.0"' "$pins" &&
  grep -q -F -e 'SDK version is not deployment floor' "$pins" &&
  grep -q -F -e 'APPLE_SDK_IDENTITY = "MacOSX26.5"' "$pins" &&
  grep -q -F -e 'hermetic-llvm v0.8.19 pinned extraction' "$pins" &&
  grep -q -F -e '6314688712edf3a95f78642d80393868256b4ef2' "$pins"; then
  ok
else
  bad "pins.bzl lost its macOS 14.0 plus SDK-not-floor plus MacOSX26.5 hermetic-llvm v0.8.19 identity under issue #500"
fi

# Pins record the Windows /MD retail CRT plus STL plus UCRT/VCRuntime plus package identities.
if grep -q -F -e 'WINDOWS_CRT_START = "/MD"' "$pins" &&
  grep -q -F -e 'WINDOWS_STL = "Microsoft STL"' "$pins" &&
  grep -q -F -e '"UCRT"' "$pins" &&
  grep -q -F -e '"VCRuntime"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_MSVC = "14.50.35717"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_REDIST = "14.50.35710"' "$pins" &&
  grep -q -F -e 'WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"' "$pins" &&
  grep -q -F -e '/MT plus debug CRT are not assumed interchangeable' "$pins"; then
  ok
else
  bad "pins.bzl lost its Windows /MD plus Microsoft STL plus UCRT/VCRuntime plus MSVC/redist/SDK plus /MT rejection under issue #500"
fi

# Pins record oldest-target vs current-host separation (merged proof rejected).
if grep -q -F -e 'oldest-target and current-host fixtures run separately' "$pins" &&
  grep -q -F -e 'matching native target execution, cross-building alone is insufficient' "$pins" &&
  grep -q -F -e 'merged oldest-target plus current-host proof' "$pins"; then
  ok
else
  bad "pins.bzl lost its oldest-target vs current-host separation plus merged-proof rejection under issue #500"
fi

# Pins record compiler plus clangd plus bindgen loader inspection (minimal archive not proof).
if grep -q -F -e 'inspect compiler, clangd and bindgen loader dependencies' "$pins" &&
  grep -q -F -e 'compiler loader dependencies' "$pins" &&
  grep -q -F -e 'clangd loader dependencies' "$pins" &&
  grep -q -F -e 'bindgen loader dependencies' "$pins" &&
  grep -q -F -e 'the minimal compiler archive alone is not proof that a loadable libclang exists' "$pins"; then
  ok
else
  bad "pins.bzl lost its compiler/clangd/bindgen loader inspection plus minimal-archive rejection under issue #500"
fi

# Pins record the Apple framework subset plus oldest-OS plus completeness plus licensing gates.
if grep -q -F -e "Check Apple's extracted SDK framework subset" "$pins" &&
  grep -q -F -e 'oldest-OS execution' "$pins" &&
  grep -q -F -e 'framework completeness' "$pins" &&
  grep -q -F -e 'licensing' "$pins" &&
  grep -q -F -e 'best-effort gaps never block required-host release' "$pins"; then
  ok
else
  bad "pins.bzl lost its Apple framework subset plus oldest-OS/completeness/licensing gates under issue #500"
fi

# Pins record the rejected substitutes (unpinned floors plus SDK-as-floor).
if grep -q -F -e '"unpinned floors"' "$pins" &&
  grep -q -F -e '"SDK version as deployment floor"' "$pins"; then
  ok
else
  bad "pins.bzl lost its unpinned-floors plus SDK-as-floor rejection under issue #500"
fi

# Native plan owns the qualified floors record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #500' "$native" &&
  grep -q -F -e 'deployment_floors_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/deployment_floors/pins.bzl' "$native" &&
  grep -q -F -e 'Which deployment and execution floors are supportable?' "$native" &&
  grep -q -F -e 'unpinned floors rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified floors record with fixtures under issue #500"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "deployment_floors_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:deployment_floors_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the deployment_floors_qualification wiring (want target plus dogfood-freshness)"
fi

# Fixture floors.expected plus oldest/current separation covers every floor with split runs.
if grep -q -F -e 'glibc 2.28 symbol floor' "$floors_expected" &&
  grep -q -F -e 'musl 1.2.6' "$floors_expected" &&
  grep -q -F -e 'deployment default 14.0' "$floors_expected" &&
  grep -q -F -e 'MacOSX26.5' "$floors_expected" &&
  grep -q -F -e '/MD' "$floors_expected" &&
  grep -q -F -e 'oldest-target and current-host fixtures run separately' "$floors_expected" &&
  grep -q -F -e 'glibc 2.28 oldest-target symbols' "$oldest" &&
  grep -q -F -e 'macOS 14.0 oldest-target deployment' "$oldest" &&
  grep -q -F -e 'seed-host plus CI-runner execution floors' "$current" &&
  grep -q -F -e 'cross-building alone is insufficient' "$current"; then
  ok
else
  bad "floors.expected plus oldest_target plus current_host lost floor coverage with split runs (want glibc/musl/macOS/SDK/CRT plus separation, issue #500)"
fi

# Fixture loader plus framework inspection covers compilers plus tools plus Apple subset.
if grep -q -F -e 'compiler loader dependencies' "$loaders" &&
  grep -q -F -e 'clangd loader dependencies' "$loaders" &&
  grep -q -F -e 'bindgen loader dependencies' "$loaders" &&
  grep -q -F -e 'the minimal compiler archive alone is not proof that a loadable libclang exists' "$loaders" &&
  grep -q -F -e "Check Apple's extracted SDK framework subset" "$frameworks" &&
  grep -q -F -e 'MacOSX26.5' "$frameworks" &&
  grep -q -F -e 'SDK version is not deployment floor' "$frameworks"; then
  ok
else
  bad "loader_deps plus apple_frameworks lost loader plus framework inspection (want compiler/clangd/bindgen plus MacOSX26.5 subset, issue #500)"
fi

# Live proof: floors are docs-only pins, so the seed hello plus the fixture corpus build green.
if bazel build //cc/tests/fixtures/hello:hello //cc/tests/fixtures/deployment_floors/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello plus deployment floors fixture build failed (want green on the seed host, issue #500)"
fi

dx_test_summary "deployment floors qualification harness"
