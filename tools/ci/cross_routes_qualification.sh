#!/usr/bin/env bash
# Cross routes qualification harness.
#
# Defines plus proves the cross-routes slice of the native baseline with
# fixture evidence, without claiming a qualified hermetic-llvm backend,
# qualified floors beyond, qualified coverage beyond issue
# , or Supported:
# - first cohort: Linux x86_64 plus Linux arm64 execution each building
#   Linux x86_64 and arm64 glibc plus static musl; Linux cross is the
#   first priority, not a mandate to build every target from every host.
# - native rows: four native workflows qualified under issues
# #410/#411/#412/#414 with per-host runners plus the shared BuildBuddy
#   remote cache (per-host scopes deleted) on
#   the pinned upstream toolchains; backends stay provisional.
# - musl closures: Linux same-arch static musl qualified under issue
# with exec-platform tools for scripts and target musl libs for
#   apps, cross-built from Linux runners with the shared BuildBuddy cache
#   plus per-cell coverage with no union.
# - cross-arch: x86_64-to-arm64 plus arm64-to-x86_64 glibc plus musl
#   stay in the first cohort with matching native target execution plus
#   separate cache and remote evidence; cross-building alone is
#   insufficient.
# - optional expansion: macOS arm64 to Linux profiles
#   only if bounded upstream configuration suffices (macOS x86_64 Not
#   planned per #976); expand only after the initial cohort passes.
# - excluded: Linux/macOS-to-Windows, Linux/Windows-to-macOS,
#   Windows-to-Linux, Windows arm64 stay outside the initial cohort,
#   not impossibility claims; all-cross mandate rejected.
# - execution: actual action execution platform recorded, not only the
#   runner; artifacts run on matching native workers; emulation,
#   Rosetta, remote execution and floor testing are distinct; LLVM CI
#   remote plus macOS smoke and rules_rs GNULVM vs MSVC distinctions
#   preserved; compiler-target availability alone is not proof.
# - cache plus remote: separate scope plus remote evidence for every
#   claimed row; required native workflows never weakened for a larger
#   table; cross-host Windows inference rejected.
# - open with honest records: backends stay provisional, floors
# qualified seed-only, coverage qualified seed-only
# , corpus qualified seed-only.
#
# Versioned here, run by CI via `bazel run //tools/ci:cross_routes_qualification`,
# following //tools/ci:strict_generation_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

pins="cc/tests/fixtures/cross_routes/pins.bzl"
pins_build="cc/tests/fixtures/cross_routes/BUILD.bazel"
routes="cc/tests/fixtures/cross_routes/routes.expected"
execution="cc/tests/fixtures/cross_routes/execution.txt"
cache_remote="cc/tests/fixtures/cross_routes/cache_remote.txt"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$routes" && -f "$execution" && -f "$cache_remote" ]]; then
  ok
else
  bad "cross routes fixture missing (want $pins plus $pins_build plus routes.expected plus execution.txt plus cache_remote.txt)"
fi

# Pins record the first Linux cross-build cohort, not an all-cross mandate.
if grep -q -F -e 'LINUX_X86_64_EXEC = "Linux x86_64"' "$pins" &&
  grep -q -F -e 'LINUX_ARM64_EXEC = "Linux arm64"' "$pins" &&
  grep -q -F -e 'First Linux cross-build cohort' "$pins" &&
  grep -q -F -e 'not a mandate to build every target from every host' "$pins" &&
  grep -q -F -e '"Linux x86_64 static musl"' "$pins" &&
  grep -q -F -e '"Linux arm64 static musl"' "$pins"; then
  ok
else
  bad "pins.bzl lost its first Linux cross-build cohort plus not-every-target mandate under issue #504"
fi

# Pins record the four qualified native rows with runners (shared cache
# only; macOS x86_64 removed per #976).
if grep -q -F -e 'Native x86_64 glibc qualified under issue #410 on ubuntu-latest seed' "$pins" &&
  grep -q -F -e 'Native arm64 glibc qualified under issue #410 on ubuntu-24.04-arm' "$pins" &&
  grep -q -F -e 'Native macOS arm64 qualified under issue #412 on macos-14' "$pins" &&
  ! grep -q -F -e 'Native macOS x86_64' "$pins" &&
  grep -q -F -e 'Native Windows x86_64 qualified under issue #414 on windows-latest' "$pins" &&
  grep -q -F -e 'pinned upstream toolchains with provisional backends' "$pins"; then
  ok
else
  bad "pins.bzl lost its four qualified native rows with runners under issue #504 plus #976"
fi

# Pins record the qualified Linux same-arch musl closures plus cross-arch cohort gates.
if grep -q -F -e 'x86_64 static musl qualified under issue #411' "$pins" &&
  grep -q -F -e 'ubuntu-latest with shared BuildBuddy cache' "$pins" &&
  grep -q -F -e 'ubuntu-24.04-arm with shared BuildBuddy cache' "$pins" &&
  grep -q -F -e 'exec-platform tools for build scripts and proc macros with target musl libs for apps' "$pins" &&
  grep -q -F -e 'arm64-to-x86_64 cross stays in the first Linux cross-build cohort' "$pins" &&
  grep -q -F -e 'matching native target execution with separate cache and remote evidence' "$pins"; then
  ok
else
  bad "pins.bzl lost its qualified musl closures plus cross-arch cohort gates under issue #504"
fi

# Pins record the optional macOS expansion with its gate.
if grep -q -F -e 'OPTIONAL_EXPANSION_EXEC = "macOS arm64"' "$pins" &&
  grep -q -F -e '"Linux x86_64 and arm64 profiles"' "$pins" &&
  grep -q -F -e 'Optional first expansion if bounded upstream configuration suffices' "$pins" &&
  grep -q -F -e 'Expand only after the initial cohort passes' "$pins"; then
  ok
else
  bad "pins.bzl lost its optional macOS expansion plus expand-after-cohort gate under issue #504"
fi

# Pins record the excluded routes plus the all-cross rejection.
if grep -q -F -e '"Linux-to-Windows"' "$pins" &&
  grep -q -F -e '"macOS-to-Windows"' "$pins" &&
  grep -q -F -e '"Linux-to-macOS"' "$pins" &&
  grep -q -F -e '"Windows-to-macOS"' "$pins" &&
  grep -q -F -e '"Windows-to-Linux"' "$pins" &&
  grep -q -F -e '"Windows arm64"' "$pins" &&
  grep -q -F -e 'do not require Linux/macOS-to-Windows, Linux/Windows-to-macOS, Windows-to-Linux, or Windows arm64 to complete this cohort' "$pins" &&
  grep -q -F -e '"all-cross mandate"' "$pins" &&
  grep -q -F -e '"every target from every host"' "$pins"; then
  ok
else
  bad "pins.bzl lost its excluded routes plus all-cross mandate rejection under issue #504"
fi

# Pins record the execution evidence contract (platform plus target plus distinct).
if grep -q -F -e 'Record the actual action execution platform, not only the Bazel client or CI runner OS' "$pins" &&
  grep -q -F -e 'Run resulting artifacts on matching native target workers; cross-building alone is insufficient' "$pins" &&
  grep -q -F -e 'Emulation, Rosetta, remote execution and deployment-floor testing are distinct evidence' "$pins" &&
  grep -q -F -e 'hermetic-llvm CI uses remote execution in Linux jobs and narrower macOS smoke and coverage tests' "$pins" &&
  grep -q -F -e 'rules_rs Windows-labelled lane builds remote GNULVM targets, not native MSVC tests or coverage' "$pins"; then
  ok
else
  bad "pins.bzl lost its execution platform plus target plus distinct-evidence contract under issue #504"
fi

# Pins record cache plus remote separation with no weakened natives.
if grep -q -F -e 'BuildBuddy shared remote cache, no per-host scopes' "$pins" &&
  grep -q -F -e 'shared cache evidence for every claimed row' "$pins" &&
  grep -q -F -e 'separate cache and remote evidence for every claimed row' "$pins" &&
  grep -q -F -e 'Do not weaken required native workflows to obtain a larger cross-build table' "$pins"; then
  ok
else
  bad "pins.bzl lost its shared-cache plus remote separation plus no-weakened-natives record under issue #504"
fi

# Pins record the rejected substitutes plus native-only boundary.
if grep -q -F -e '"cross-building alone as execution proof"' "$pins" &&
  grep -q -F -e '"emulation as native target execution"' "$pins" &&
  grep -q -F -e '"compiler-target availability as route proof"' "$pins" &&
  grep -q -F -e '"cross-host Windows inference"' "$pins" &&
  grep -q -F -e 'Native only' "$pins" &&
  grep -q -F -e 'dynamic musl explicitly out of scope' "$pins" &&
  grep -q -F -e 'no Windows or macOS cross-host claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its rejected substitutes plus native-only boundary under issue #504"
fi

# Fixture cohort texts cover exec plus targets plus natives plus exclusions.
if grep -q -F -e 'First Linux cross-build cohort' "$routes" &&
  grep -q -F -e 'Linux x86_64-to-arm64 glibc plus static musl' "$routes" &&
  grep -q -F -e 'Linux arm64-to-x86_64 glibc plus static musl' "$routes" &&
  grep -q -F -e 'Native Windows x86_64 qualified under issue #414 on windows-latest' "$routes" &&
  grep -q -F -e 'Optional first expansion if bounded upstream configuration suffices' "$routes" &&
  grep -q -F -e 'do not require Linux/macOS-to-Windows, Linux/Windows-to-macOS, Windows-to-Linux, or Windows arm64 to complete this cohort' "$routes" &&
  grep -q -F -e 'no Windows or macOS cross-host claim' "$routes"; then
  ok
else
  bad "routes.expected lost cohort coverage (want first cohort plus cross-arch plus natives plus expansion plus exclusions, issue #504)"
fi

# Fixture execution plus cache texts cover platform plus target plus scopes.
if grep -q -F -e 'Record the actual action execution platform, not only the Bazel client or CI runner OS' "$execution" &&
  grep -q -F -e 'Run resulting artifacts on matching native target workers; cross-building alone is insufficient' "$execution" &&
  grep -q -F -e 'Emulation, Rosetta, remote execution and deployment-floor testing are distinct evidence' "$execution" &&
  grep -q -F -e 'rules_rs Windows-labelled lane builds remote GNULVM targets, not native MSVC tests or coverage' "$execution" &&
  grep -q -F -e 'separate cache and remote evidence for every claimed row' "$cache_remote" &&
  grep -q -F -e 'BuildBuddy shared remote cache, no per-host scopes' "$cache_remote" &&
  grep -q -F -e 'shared cache evidence for every claimed row' "$cache_remote" &&
  grep -q -F -e 'Do not weaken required native workflows to obtain a larger cross-build table' "$cache_remote"; then
  ok
else
  bad "execution.txt plus cache_remote.txt lost evidence coverage (want platform plus target plus distinct plus shared cache, issue #504)"
fi

# Native plan owns the qualified cross-routes record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #504' "$native" &&
  grep -q -F -e 'cross_routes_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/cross_routes/pins.bzl' "$native" &&
  grep -q -F -e 'Which cross routes actually work and execute?' "$native" &&
  grep -q -F -e 'not a mandate to build every target from every host' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified cross-routes record with fixtures under issue #504"
fi

# ci_targets_d.bzl owns the harness target plus dogfood-freshness wires it.
if grep -q -F -e 'name = "cross_routes_qualification"' "tools/ci/ci_targets_d.bzl" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:cross_routes_qualification' tools/ci/dogfood_freshness.sh; then
  ok
else
  bad "tools/ci/ci_targets_d.bzl or dogfood_freshness.sh lost the cross_routes_qualification wiring (want target plus dogfood-freshness)"
fi

# Live proof: the seed hello plus the fixture corpus build green on the
# seed host with no cross-host requirement.
if bazel build //cc/tests/fixtures/hello:hello //cc/tests/fixtures/cross_routes/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello plus cross routes fixture build failed (want green on the seed host, issue #504)"
fi

# Live proof: the Linux profiles stay green plus Rust musl std still
# resolves, proving the first-cohort reference shapes without claiming
# cross-arch execution from the seed host.
if bazel query @rust_toolchains//... 2>/dev/null | grep -E -e 'musl' >/dev/null 2>&1 &&
  bazel build //cc/tests/fixtures/linux_corpus:corpus //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "linux corpus plus musl toolchain proof failed (want musl std plus corpus green, issue #504)"
fi

dx_test_summary "cross routes qualification harness"
