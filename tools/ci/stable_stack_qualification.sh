#!/usr/bin/env bash
# Stable-stack compose qualification harness.
#
# Freezes the exact current stable stack that composes on the seed host,
# with fixture evidence recorded explicitly here and in the owning docs,
# never silently composed (ad-hoc compose rejected per the issue
# alternatives):
# - as-built freeze: Bazel 9.2.0 plus rules_rust 0.74.0 plus rules_cc
#   0.2.22 plus Rust 1.98.0 (edition 2021, rustfmt coupled 1.98.0) from
#   `.bazelversion`, `MODULE.bazel`, `MODULE.bazel.lock`
#   (`registryFileHashes` integrity), and
#   `libs/testing/tested_stack.bzl`; living at head plus floating versions
#   rejected.
# - candidate comparison: rules_rs v0.0.109 (b55b132...) declares LLVM
#   rules 0.8.18/LLVM 22.1.8 as its baseline, not the newer candidate
#   hermetic-llvm v0.8.19 (631468..., LLVM 23.1.0); its pinned patched
#   rules_rust (e9dd49f..., module 0.74.0) does not make it stock
#   bazelbuild/rules_rust 0.74.0. Upgrading across that line is a real
#   stack change. C/C++ rules stay rules_cc 0.2.22; Windows backend stays
#   toolchains_msvc at 8e2aa46... (module 0.0.0 is not a release).
# - checksums plus patches plus compiler/profile compat: freeze compiler
#   archives, runtime sources, upstream patches, SDK manifests and package
#   hashes separately from ruleset hashes (LLVM archive index plus source
#   acquisition are the starting points). Bazel 9 is a provisional initial
#   coverage baseline because the inspected hermetic-llvm coverage fixture
#   requires it, not a release pin: the release default follows ADR 0008
#   and the exact seed pin is tracked here. Do not silently inherit
#   rules_rs's older compiler default or turn a research version into a
#   release pin. No second Rust graph, no llvm.version override, no MODULE
#   llvm dep, no second Bazel.
# - live proof: the as-built stack composes on the seed host (Rust plus
#   C++ hello fixtures build and test green); the hermetic-llvm candidate
# backend plus corpus wiring stays provisional.
#   Platform plus consumer plus release evidence stays open; no Supported
#   claim. Compatibility is compose only.
#
# Versioned here, run by CI via `bazel run //tools/ci:stable_stack_qualification`,
# following //tools/ci:roslyn_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="rust/tests/fixtures/stable_stack/pins.bzl"
fixture_build="rust/tests/fixtures/stable_stack/BUILD.bazel"
module="MODULE.bazel"
lock="MODULE.bazel.lock"
bazelversion=".bazelversion"
preset="tools/bazelrc/preset.py"
stack="libs/testing/tested_stack.bzl"
native="docs/native-toolchains.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture pair stays present.
if [[ -f "$pins" && -f "$fixture_build" ]]; then
  ok
else
  bad "stable-stack fixture missing (want $pins plus $fixture_build)"
fi

# Pins record the as-built Bzlmod freeze (living at head rejected).
if grep -q -F -e 'BAZEL_VERSION = "9.2.0"' "$pins" &&
  grep -q -F -e 'RULES_RUST_VERSION = "0.74.0"' "$pins" &&
  grep -q -F -e 'RULES_CC_VERSION = "0.2.22"' "$pins" &&
  grep -q -F -e 'RUST_VERSION = "1.98.0"' "$pins" &&
  grep -q -F -e 'RUST_EDITION = "2021"' "$pins" &&
  grep -q -F -e 'RUSTFMT_VERSION = "1.98.0"' "$pins" &&
  grep -q -F -e 'STABLE_LOCK_FILE = "//:MODULE.bazel.lock"' "$pins" &&
  grep -q -F -e 'STABLE_LOCK_INTEGRITY = "registryFileHashes"' "$pins"; then
  ok
else
  bad "pins.bzl lost its as-built Bzlmod freeze under issue #494"
fi

# Pins record the LLVM baseline-vs-target comparison (upgrading is a real stack change).
if grep -q -F -e 'RULES_RS_VERSION = "v0.0.109"' "$pins" &&
  grep -q -F -e 'b55b132af0c9951807c926768e40222330348632' "$pins" &&
  grep -q -F -e 'LLVM_BASELINE_MODULE = "0.8.18"' "$pins" &&
  grep -q -F -e 'LLVM_BASELINE_LLVM = "22.1.8"' "$pins" &&
  grep -q -F -e 'LLVM_TARGET_MODULE = "0.8.19"' "$pins" &&
  grep -q -F -e 'LLVM_TARGET_LLVM = "23.1.0"' "$pins" &&
  grep -q -F -e '6314688712edf3a95f78642d80393868256b4ef2' "$pins" &&
  grep -q -F -e 'PATCHED_RULES_RUST_COMMIT = "e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde"' "$pins" &&
  grep -q -F -e 'TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"' "$pins"; then
  ok
else
  bad "pins.bzl lost its LLVM baseline-vs-target comparison under issue #494"
fi

# Pins record checksums plus patches plus compiler/profile compat plus ad-hoc rejection.
if grep -q -F -e 'COMPILER_ARCHIVE_INDEX = "extensions/llvm_toolchain_minimal_index.json"' "$pins" &&
  grep -q -F -e 'SOURCE_ACQUISITION = "extensions/llvm.bzl"' "$pins" &&
  grep -q -F -e 'Freeze compiler archives, runtime sources, upstream patches' "$pins" &&
  grep -q -F -e 'Bazel 9 is a provisional initial coverage baseline' "$pins" &&
  grep -q -F -e 'the release default follows ADR 0008' "$pins" &&
  grep -q -F -e 'Do not silently inherit rules_rs' "$pins" &&
  grep -q -F -e 'ad-hoc compose rejected' "$pins" &&
  grep -q -F -e 'STABLE_REJECTED' "$pins"; then
  ok
else
  bad "pins.bzl lost its checksums plus patches plus compat plus ad-hoc rejection under issue #494"
fi

# MODULE.bazel as-built pins match the frozen fixture.
if grep -q -F -e 'bazel_dep(name = "rules_rust", version = "0.74.0")' "$module" &&
  grep -q -F -e 'bazel_dep(name = "rules_cc", version = "0.2.22")' "$module" &&
  grep -q -F -e 'versions = ["1.98.0"]' "$module" &&
  grep -q -F -e 'edition = "2021"' "$module" &&
  grep -q -F -e 'rustfmt_version = "1.98.0"' "$module"; then
  ok
else
  bad "MODULE.bazel lost its as-built stable-stack pins under issue #494"
fi

# MODULE.bazel keeps the single Rust graph (no second graph, no floating LLVM, no second Bazel).
if ! grep -q -F -e 'cxx.rs' "$module" &&
  [[ "$(grep -c -F -e 'use_repo(rust, "rust_toolchains")' "$module")" == "1" ]] &&
  ! grep -q -F -e 'llvm.version(' "$module" &&
  ! grep -q -F -e 'bazel_dep(name = "llvm"' "$module" &&
  ! grep -q -F -e 'bazel_binaries.download' "$module"; then
  ok
else
  bad "MODULE.bazel gained a second Rust graph or floating LLVM or second Bazel (want single rust_toolchains, no cxx.rs, no llvm pin)"
fi

# Canonical Bazel agreement: .bazelversion plus preset plus tested_stack all track 9.2.0.
if [[ "$(tr -d '[:space:]' <"$bazelversion")" == "9.2.0" ]] &&
  grep -q -F -e 'PRESET_BAZEL_VERSION = "9.2.0"' "$preset" &&
  grep -q -F -e 'default = "9.2.0"' "$stack" &&
  grep -q -F -e 'Must match .bazelversion' "$stack"; then
  ok
else
  bad "canonical Bazel pin drifted (want .bazelversion plus preset.py plus tested_stack.bzl at 9.2.0)"
fi

# Committed BCR lock carries integrity for the frozen as-built modules.
if [[ -f "$lock" ]] &&
  grep -q -F -e 'registryFileHashes' "$lock" &&
  grep -q -F -e 'modules/rules_cc/0.2.22/source.json' "$lock" &&
  grep -q -F -e 'modules/rules_rust/0.74.0/source.json' "$lock"; then
  ok
else
  bad "MODULE.bazel.lock lost its frozen as-built integrity (want registryFileHashes plus rules_cc 0.2.22 plus rules_rust 0.74.0 hashes)"
fi

# Committed lock stays fail-closed: lockfile present and tracked.
if [[ -f "$lock" ]] && git ls-files --error-unmatch "$lock" >/dev/null 2>&1; then
  ok
else
  bad "MODULE.bazel.lock lost its committed fail-closed record (want tracked $lock)"
fi

# Unpinned LLVM stays rejected: no floating version override lands outside pins plus harness.
if ! grep -R --include='*.bzl' --include='BUILD.bazel' --exclude='stable_stack_qualification.sh' --exclude='pins.bzl' -F -e 'llvm.version(' -- rust cc tools third_party 2>/dev/null | grep -q .; then
  ok
else
  bad "unpinned LLVM detected (llvm.version override; want pins.bzl pins only)"
fi

# Native plan owns the qualified compose record with fixtures plus harness.
if grep -q -F -e 'qualified seed-only under issue #494' "$native" &&
  grep -q -F -e 'rust/tests/fixtures/stable_stack/pins.bzl' "$native" &&
  grep -q -F -e 'stable_stack_qualification' "$native" &&
  grep -q -F -e 'ad-hoc compose rejected' "$native" &&
  grep -q -F -e 'Does the exact current stable stack compose?' "$native" &&
  grep -q -F -e 'issue #494' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified compose record with fixtures plus harness under issue #494"
fi

# Native plan seed-pin line freezes the exact pin with the provisional-baseline honesty.
if grep -q -F -e 'Bazel 9 is a provisional' "$native" &&
  grep -q -F -e 'the release default follows ADR 0008' "$native" &&
  grep -q -F -e 'rust/tests/fixtures/stable_stack/pins.bzl' "$native" &&
  grep -q -F -e 'Do not silently inherit rules_rs' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its frozen seed-pin line with provisional-baseline honesty under issue #494"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "stable_stack_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:stable_stack_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the stable_stack_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'stable_stack_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under issue #494' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:stable_stack_qualification' "$verify" &&
  grep -q -F -e '`stable_stack_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #494 stable-stack qualified record"
fi

# Live proof: the as-built stack composes (Rust plus C++ hello builds green on the seed host).
if bazel build //rust/tests/fixtures/hello:hello //cc/tests/fixtures/hello:hello --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "stable-stack live build failed (want Rust plus C++ hello green on the as-built stack)"
fi

# Live proof: both sides test green (Rust hello plus C++ hello).
if bazel test //rust/tests/fixtures/hello:hello_test //cc/tests/fixtures/hello:hello_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "stable-stack live tests failed (want Rust plus C++ hello tests green)"
fi

dx_test_summary "stable-stack qualification harness"
