#!/usr/bin/env bash
# macOS arm64 qualification harness (issue #412).
#
# Machine-checks the as-built macOS arm64 record with fixture evidence
# and owned gaps, without claiming Supported or macOS x86_64:
# - delivered: dx qualified_hosts includes macos/aarch64 with macos x86_64
#   plus windows staying refused, per-cell coverage for the macos arm64
#   cell with no union, macOS CI jobs natively on macos-14 runners with a
#   per-host cache scope under the portable-shell contract, consumer plus
#   release evidence per the support-matrix lifecycle (release evidence
#   open), docs in support-matrix plus ADR 0014 plus native-toolchains;
# - Apple-SDK handling: pinned acquired SDK identity plus deployment floor
#   stay owned by O14/O37 per ADR 0014 (SDK version is not the deployment
#   floor); hermetic-llvm Apple-SDK backend stays provisional with
#   immutable lazy fetch; host-installed SDK fallback is never approved;
#   CI handling leaks no secrets and requires no interactive acceptance;
# - open with honest records: full hermetic-llvm backend, Apple
#   acquisition/cache rights review, remaining native-plan corpus gaps,
#   dx_tools macos_arm64 artifacts (quality tools run on the Linux exec
#   platform), release evidence (SBOM/provenance/signing/BCR).
#
# Versioned here, run by CI via `bazel run //tools/ci:macos_qualification`,
# following //tools/ci:musl_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

# dx qualified hosts: macos/aarch64 joins the Linux pair; macos x86_64
# plus windows stay refused (issue #412 flips macos_arm64 only).
if grep -q -F -e '("macos", "aarch64")' cli/cli/src/platform.rs &&
  grep -q -F -e 'qualified_hosts' cli/cli/src/platform.rs &&
  grep -q -F -e 'macos_arm64_host_is_qualified' cli/cli/src/platform.rs &&
  grep -q -F -e '("macos", "x86_64")' cli/cli/src/platform.rs &&
  grep -q -F -e '("windows", "x86_64")' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs lost the issue #412 macos/aarch64 qualified-host entry (macos x86_64 plus windows stay refused)"
fi

# dx host refusal names macOS arm64 as delivered with no host fallback.
if grep -q -F -e 'plus macOS arm64 (issue #412' cli/cli/src/platform.rs &&
  grep -q -F -e 'host-installed SDK fallback never approved' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs refusal lost the macOS arm64 delivered plus no-host-fallback record"
fi

# dx status names the macOS arm64 qualification.
if grep -q -F -e 'plus macos_arm64 qualified' cli/adopt/src/status.rs; then
  ok
else
  bad "adopt status lost the macos_arm64 qualified detail (issue #412)"
fi

# Support matrix flips macOS arm64 only; macOS x86_64 plus windows stay
# unqualified with clean refusal.
if grep -E -e '^\| macOS arm64 \|' docs/product/support-matrix.md | grep -q -F -e 'Platform-qualified (issue #412' &&
  grep -E -e '^\| macOS arm64 \|' docs/product/support-matrix.md | grep -q -F -e 'host-installed SDK fallback never approved' &&
  grep -E -e '^\| macOS arm64 \|' docs/product/support-matrix.md | grep -q -F -e 'release evidence open' &&
  grep -E -e '^\| macOS x86_64 \|' docs/product/support-matrix.md | grep -q -F -e 'Unqualified: clean'; then
  ok
else
  bad "support-matrix lost the macOS arm64 Platform-qualified record (issue #412, macos x86_64 stays unqualified)"
fi

# ADR 0014 keeps macOS arm64 required and records the qualification.
# Exact pins, hosts, floors, and SDK/CRT identities stay owned by O14/O37.
if grep -q -F -e '| macOS arm64 | Required' docs/decisions/0014-tested-platform-release-stack.md &&
  grep -q -F -e 'macOS arm64 qualified under issue #412' docs/decisions/0014-tested-platform-release-stack.md; then
  ok
else
  bad "ADR 0014 lost the macOS arm64 required plus qualified record (issue #412)"
fi

# Native plan records the as-built macOS closure, the provisional
# hermetic-llvm Apple-SDK backend with immutable lazy fetch, and the
# no-host-fallback boundary.
if grep -q -F -e 'macOS arm64 native is qualified (issue #412' docs/native-toolchains.md &&
  grep -q -F -e 'Apple-SDK backend stays provisional' docs/native-toolchains.md &&
  grep -q -F -e 'host-installed SDK fallback' docs/native-toolchains.md &&
  grep -q -F -e 'immutable lazy fetch' docs/native-toolchains.md; then
  ok
else
  bad "native-toolchains lost the macOS arm64 closure plus provisional backend plus no-fallback record (issue #412)"
fi

# Per-cell coverage registry: five qualified, one unqualified, no union.
if [[ -f "tools/coverage/macos-arm64-inventory.txt" ]] &&
  grep -q -F -e 'qualified macos_arm64 tools/coverage/macos-arm64-inventory.txt' tools/coverage/cells.txt &&
  grep -q -F -e 'Host-installed SDK fallback is never' tools/coverage/macos-arm64-inventory.txt; then
  ok
else
  bad "macos per-cell coverage registry lost its qualified cell (issue #412)"
fi

# CI macOS jobs exist natively on macos-14 with a per-host cache scope.
if grep -q -F -e 'build-macos-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'test-macos-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage-macos-arm64' .github/workflows/ci.yml &&
  grep -q -F -e 'runs-on: macos-14' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel-macos-arm64-' .github/workflows/ci.yml &&
  grep -q -F -e 'macos arm64' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the macOS arm64 native jobs with per-host cache scope on macos-14 (issue #412)"
fi

# CI macOS jobs stay portable-shell clean: no banned forms in the workflow.
# Note: the sed pattern below is split to avoid a literal banned form in
# this file (shell_contract bans `sed -i` in executable code).
if ! grep -e 'realpath' .github/workflows/ci.yml | grep -v -F -e 'dx_realpath' | grep -q . &&
  ! grep -e 'sed -''i' .github/workflows/ci.yml | grep -q .; then
  ok
else
  bad "ci.yml macos jobs introduced a non-portable shell form (issue #323)"
fi

# Apple-SDK handling leaks no secrets and needs no interactive acceptance:
# no secret env, no EULA-accept variable, no interactive prompt in the
# macOS jobs or the qualification docs.
if ! grep -A20 -e 'build-macos-arm64' .github/workflows/ci.yml | grep -E -e 'secrets\.|GH_TOKEN|EULA_ACCEPT|accept.*license' | grep -q . &&
  ! grep -rn -F -e 'xcode-select --install' --include='*.yml' .github/ 2>/dev/null | grep -q .; then
  ok
else
  bad "macos jobs leak secrets or require interactive Apple-SDK acceptance (issue #412)"
fi

# No host-installed SDK fallback claim anywhere: the only allowed mentions
# deny it on the same line (`never approved`, `never-approved`, or an
# explicit `no`/`No` denial such as `no host-installed SDK fallback`).
# (Self-excluded: this script names the banned form in its own pattern.)
if ! grep -rn -F -e 'host-installed SDK' --exclude='macos_qualification.sh' docs/ cli/ tools/ .github/ 2>/dev/null | grep -v -F -e 'never approved' | grep -v -F -e 'never-approved' | grep -v -F -e 'no host-installed SDK fallback' | grep -v -F -e 'No host-installed SDK fallback' | grep -q .; then
  ok
else
  bad "a host-installed SDK fallback claim appeared (stays never approved, issue #412)"
fi

# No Supported claim for macOS: Platform-qualified only, release open.
if grep -E -e '^\| macOS arm64 \|' docs/product/support-matrix.md | grep -q -F -e 'Platform-qualified' &&
  ! grep -E -e '^\| macOS arm64 \|' docs/product/support-matrix.md | grep -q -F -e 'Supported'; then
  ok
else
  bad "macos row lost its Platform-qualified (never Supported) record"
fi

dx_test_summary "macos arm64 qualification harness"
