#!/usr/bin/env bash
# Static-musl profile qualification harness (issue #411).
#
# Machine-checks the as-built static-musl record with fixture evidence
# and owned gaps, without claiming Supported or dynamic musl:
# - delivered: Rust musl std via extra_target_triples, dx
#   qualified_static_musl_profiles plus musl_profile_refusal with dynamic
#   explicitly refused, per-cell coverage for both musl cells with no
#   union, musl CI jobs cross-building from Linux runners with per-profile
#   cache scopes under the portable-shell contract, consumer plus release
#   evidence per the support-matrix lifecycle (release evidence open),
#   docs in support-matrix plus ADR 0014 plus native-toolchains;
# - open with honest records: full hermetic-llvm backend, prebuilt glibc
#   interop (never by linker change alone), remaining native-plan corpus
#   (SQLite/OpenSSL/ring/bindgen/CXX), dx_tools musl artifacts (exec tools
#   stay glibc), release evidence (SBOM/provenance/signing/BCR).
#
# Versioned here, run by CI via `bazel run //tools/ci:musl_qualification`,
# following //tools/ci:coverage_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# Rust musl std: both static triples resolve via extra_target_triples.
# Bounded upstream maintenance: rules_rust ships musl std, no new backend.
if grep -q -F -e 'extra_target_triples' MODULE.bazel &&
  grep -q -F -e '"x86_64-unknown-linux-musl"' MODULE.bazel &&
  grep -q -F -e '"aarch64-unknown-linux-musl"' MODULE.bazel &&
  grep -q -F -e 'Issue #411' MODULE.bazel; then
  ok
else
  bad "MODULE.bazel lost the issue #411 static-musl extra_target_triples"
fi

# dx qualified static-musl profiles: two static entries, dynamic refused.
if grep -q -F -e 'qualified_static_musl_profiles' cli/cli/src/platform.rs &&
  grep -q -F -e '"linux_x86_64_static_musl"' cli/cli/src/platform.rs &&
  grep -q -F -e '"linux_arm64_static_musl"' cli/cli/src/platform.rs &&
  grep -q -F -e 'musl_profile_refusal' cli/cli/src/platform.rs &&
  grep -q -F -e 'dynamic musl' cli/cli/src/platform.rs &&
  grep -q -F -e 'static_musl_profiles_are_qualified' cli/cli/src/platform.rs &&
  grep -q -F -e 'dynamic_musl_is_explicitly_refused' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs lost the issue #411 qualified static-musl profile list plus dynamic refusal"
fi

# dx host refusal names static musl as delivered and dynamic as out of scope.
if grep -q -F -e 'glibc plus static musl, issue #411' cli/cli/src/platform.rs &&
  grep -q -F -e 'dynamic musl explicitly out of scope' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs refusal lost the static-musl delivered plus dynamic-out-of-scope record"
fi

# dx status names the static-musl qualification (macOS arm64 may append
# `plus macos_arm64` under issue #412; the static-musl fragment stays).
if grep -q -F -e 'glibc plus static musl' cli/adopt/src/status.rs &&
  grep -q -F -e 'qualified' cli/adopt/src/status.rs; then
  ok
else
  bad "adopt status lost the static-musl qualified detail (issue #411)"
fi

# Support matrix flips the two static profiles only; dynamic stays out.
if grep -E -e '^\| Linux x86_64/arm64 static musl \|' docs/product/support-matrix.md | grep -q -F -e 'Platform-qualified (issue #411' &&
  grep -E -e '^\| Linux x86_64/arm64 static musl \|' docs/product/support-matrix.md | grep -q -F -e 'dynamic musl explicitly out of scope' &&
  grep -E -e '^\| Linux x86_64/arm64 static musl \|' docs/product/support-matrix.md | grep -q -F -e 'release evidence open'; then
  ok
else
  bad "support-matrix lost the static-musl Platform-qualified record (issue #411)"
fi

# ADR 0014 keeps static musl required and records the qualification.
if grep -q -F -e 'Dynamic musl is not an initial requirement' docs/decisions/0014-tested-platform-release-stack.md &&
  grep -q -F -e 'static musl qualified under issue #411' docs/decisions/0014-tested-platform-release-stack.md; then
  ok
else
  bad "ADR 0014 lost the static-musl required plus qualified record (issue #411)"
fi

# Native plan records the static-only closure, exec vs target separation,
# glibc-prebuilt incompatibility, and the corpus boundary.
if grep -q -F -e 'static musl qualified (issue #411' docs/native-toolchains.md &&
  grep -q -F -e 'static-only musl' docs/native-toolchains.md &&
  grep -q -F -e 'execution-platform' docs/native-toolchains.md &&
  grep -q -F -e 'prebuilt glibc' docs/native-toolchains.md &&
  grep -q -F -e 'SQLite' docs/native-toolchains.md &&
  grep -q -F -e 'bindgen' docs/native-toolchains.md; then
  ok
else
  bad "native-toolchains lost the static-musl closure plus exec/target plus corpus record (issue #411)"
fi

# Per-cell coverage registry: musl pair qualified (six qualified plus one
# unqualified after issues #412/#413), no union.
if [[ -f "tools/coverage/musl-x86_64-inventory.txt" ]] &&
  [[ -f "tools/coverage/musl-arm64-inventory.txt" ]] &&
  grep -q -F -e 'qualified linux_x86_64_musl tools/coverage/musl-x86_64-inventory.txt' tools/coverage/cells.txt &&
  grep -q -F -e 'qualified linux_arm64_musl tools/coverage/musl-arm64-inventory.txt' tools/coverage/cells.txt &&
  grep -q -F -e 'Static musl only' tools/coverage/musl-x86_64-inventory.txt &&
  grep -q -F -e 'Static musl only' tools/coverage/musl-arm64-inventory.txt; then
  ok
else
  bad "musl per-cell coverage registry lost its two qualified cells (issue #411)"
fi

# CI musl jobs exist on Linux runners with per-profile cache scopes.
if grep -q -F -e 'build-musl-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'build-musl-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage-musl-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage-musl-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel-musl-x86_64-' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel-musl-arm64-' .github/workflows/ci.yml &&
  grep -q -F -e 'static musl' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the static-musl profile jobs with per-profile cache scopes (issue #411)"
fi

# CI musl jobs stay portable-shell clean: no banned forms in the workflow.
# Note: the sed pattern below is split to avoid a literal banned form in
# this file (shell_contract bans `sed -i` in executable code).
if ! grep -e 'realpath' .github/workflows/ci.yml | grep -v -F -e 'dx_realpath' | grep -q . &&
  ! grep -e 'sed -''i' .github/workflows/ci.yml | grep -q .; then
  ok
else
  bad "ci.yml musl jobs introduced a non-portable shell form (issue #323)"
fi

# No dynamic-musl support claim anywhere.
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -F -e 'dynamic musl qualified' --exclude='musl_qualification.sh' docs/ cli/ tools/ .github/ 2>/dev/null | grep -q . &&
  grep -q -F -e 'dynamic musl explicitly out of scope' docs/product/support-matrix.md &&
  grep -q -F -e 'Dynamic musl is not an initial requirement' docs/decisions/0014-tested-platform-release-stack.md; then
  ok
else
  bad "a dynamic-musl support claim appeared (stays explicitly out of scope, issue #411)"
fi

# No Supported claim for musl: Platform-qualified only, release open.
if grep -E -e '^\| Linux x86_64/arm64 static musl \|' docs/product/support-matrix.md | grep -q -F -e 'Platform-qualified' &&
  ! grep -E -e '^\| Linux x86_64/arm64 static musl \|' docs/product/support-matrix.md | grep -q -F -e 'Supported'; then
  ok
else
  bad "musl row lost its Platform-qualified (never Supported) record"
fi

dx_test_summary "static-musl qualification harness"
