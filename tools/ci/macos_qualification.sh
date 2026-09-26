#!/usr/bin/env bash
# macOS arm64 qualification harness (x86_64 Not planned).
#
# Machine-checks the as-built macOS arm64 (required) record with fixture
# evidence and owned gaps, without claiming Supported. macOS x86_64 is
# Not planned and never planned for support (issue #976): no CI, coverage,
# or artifact footprint is provisioned and `dx` refuses cleanly:
# - delivered: dx qualified_hosts includes macos/aarch64 with
#   macos/x86_64 refused, per-cell coverage for the macos arm64 cell with
#   no union, macOS CI jobs natively on macos-14 (arm64) with per-host
#   cache scope under the portable-shell contract, consumer plus arm64
#   release evidence (sbom-provenance delivered under #805), docs in
#   support-matrix plus ADR 0014 plus native-toolchains;
# - Apple-SDK handling: pinned acquired SDK identity plus deployment floor
# stay owned per ADR 0014 (SDK version is not the deployment
#   floor); hermetic-llvm Apple-SDK backend stays provisional with
#   immutable lazy fetch; host-installed SDK fallback is never approved;
#   CI handling leaks no secrets and requires no interactive acceptance;
# - open with honest records: full hermetic-llvm backend, Apple
#   acquisition/cache rights review, remaining native-plan corpus gaps,
#   dx_tools macos_arm64 artifacts (quality tools run on
#   the Linux exec platform), remaining release evidence (signing/BCR plus
#   tag cut).
#
# Versioned here, run by CI via `bazel run //tools/ci:macos_qualification`,
# following //tools/ci:musl_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

# dx qualified hosts: macos/aarch64 joins the Linux pair; macOS x86_64 is
# Not planned (issue #976) and stays refused. Windows x86_64 joins under
# , Windows arm64 stays refused
# (hermetic context search, issue #1006).
if dx_context_contains cli/cli/src/platform.rs 'pub fn qualified_hosts' -A 8 '("macos", "aarch64")' &&
  ! dx_context_contains cli/cli/src/platform.rs 'pub fn qualified_hosts' -A 8 '("macos", "x86_64")' &&
  grep -q -F -e 'qualified_hosts' cli/cli/src/platform.rs &&
  grep -q -F -e 'macos_arm64_host_is_qualified' cli/cli/src/platform.rs &&
  grep -q -F -e 'macos_x86_64_is_refused_not_planned' cli/cli/src/platform.rs &&
  dx_context_contains cli/cli/src/platform.rs 'pub fn qualified_hosts' -A 8 '("windows", "x86_64")'; then
  ok
else
  bad "platform.rs lost the macos/aarch64 qualified-host entry with macOS x86_64 refused (issue #412 plus #976; windows x86_64 qualified under #414, windows arm64 stays refused)"
fi

# dx host refusal names macOS arm64 as delivered with no host fallback;
# macOS x86_64 is refused as Not planned.
if grep -q -F -e 'plus macOS arm64 (issue #412' cli/cli/src/platform.rs &&
  grep -q -F -e 'macOS x86_64 is Not planned' cli/cli/src/platform.rs &&
  grep -q -F -e 'host-installed SDK fallback never approved' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs refusal lost the macOS arm64 delivered plus x86_64 Not-planned record"
fi

# dx status names the macOS arm64 qualification (Windows x86_64 may append
# `plus windows_x86_64`; the macos fragment stays; x86_64 stays absent).
if grep -q -F -e 'plus macos_arm64' cli/adopt/src/status.rs &&
  ! grep -q -F -e 'macos_x86_64' cli/adopt/src/status.rs &&
  grep -q -F -e 'qualified' cli/adopt/src/status.rs; then
  ok
else
  bad "adopt status lost the macos_arm64 qualified detail without macos_x86_64 (issue #412 plus #976; windows_x86_64 may append under #414)"
fi

# ADR 0014 keeps macOS arm64 required and records macOS x86_64 Not planned.
# Exact pins, hosts, floors, and SDK/CRT identities stay owned.
if grep -q -F -e '| macOS arm64 | Required' docs/decisions/0014-tested-platform-release-stack.md &&
  grep -q -F -e 'macOS arm64 qualified under issue #412' docs/decisions/0014-tested-platform-release-stack.md &&
  grep -q -F -e '| macOS x86_64 | Not planned' docs/decisions/0014-tested-platform-release-stack.md &&
  grep -q -F -e 'macOS x86_64 is Not planned' docs/decisions/0014-tested-platform-release-stack.md; then
  ok
else
  bad "ADR 0014 lost the macOS arm64 required plus x86_64 Not-planned records (issue #412 plus #976)"
fi

# Native plan records the as-built macOS arm64 closure, the provisional
# hermetic-llvm Apple-SDK backend with immutable lazy fetch, and the
# no-host-fallback boundary. macOS x86_64 stays Not planned with no
# qualified closure.
if grep -q -F -e 'macOS arm64 native is qualified (issue #412' docs/native-toolchains.md &&
  grep -q -F -e 'Apple-SDK backend stays provisional' docs/native-toolchains.md &&
  grep -q -F -e 'host-installed SDK fallback' docs/native-toolchains.md &&
  grep -q -F -e 'immutable lazy fetch' docs/native-toolchains.md; then
  ok
else
  bad "native-toolchains lost the macOS arm64 closure plus provisional backend plus no-fallback record (issue #412)"
fi

# Per-cell coverage registry: macos arm64 qualified with no union; macOS
# x86_64 carries no cell (six qualified, zero unqualified).
if [[ -f "tools/coverage/macos-arm64-inventory.txt" ]] &&
  [[ ! -f "tools/coverage/macos-x86_64-inventory.txt" ]] &&
  grep -q -F -e 'qualified macos_arm64 tools/coverage/macos-arm64-inventory.txt' tools/coverage/cells.txt &&
  ! grep -q -F -e 'qualified macos_x86_64' tools/coverage/cells.txt &&
  grep -q -F -e 'Host-installed SDK fallback is never' tools/coverage/macos-arm64-inventory.txt; then
  ok
else
  bad "macos per-cell coverage registry lost the arm64 qualified cell or still carries x86_64 (issue #412 plus #976)"
fi

# CI macOS arm64 runs natively through the consumer matrix on macos-14;
# ci.yml only selects the macos_arm64 platform (no macOS runner of its
# own, no macos-15-intel, no x86_64 job, and no per-host cache scope —
# hygiene is setup-bazel plus the shared BuildBuddy remote cache).
if grep -q -F -e 'macos_arm64' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'macos-14' .github/workflows/reusable-consumer.yml &&
  ! grep -q -F -e 'macos-15-intel' .github/workflows/reusable-consumer.yml &&
  ! grep -q -F -e 'macos_x86_64' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'macos_arm64' .github/workflows/ci.yml &&
  ! grep -q -F -e 'runs-on: macos' .github/workflows/ci.yml &&
  ! grep -q -F -e 'build-macos-x86_64' .github/workflows/ci.yml &&
  ! grep -q -F -e 'bazel-macos-' .github/workflows/ci.yml &&
  ! grep -q -F -e 'bazel-macos-' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "ci.yml lost the macOS arm64 native selection or still carries x86_64/cache scopes (issue #412 plus #976)"
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

# Apple-SDK handling needs no EULA variable and no interactive
# acceptance in the macOS route (the consumer matrix jobs may reference
# the BuildBuddy cache secret by design; xcode-select never runs in CI)
# (hermetic context plus tree search, issue #1006).
if DX_CONTEXT_RE=1 dx_context_absent .github/workflows/reusable-consumer.yml 'macos-14' -A 20 'EULA_ACCEPT|ACCEPT.*EULA|accept.*license' &&
  dx_tree_absent --include='*.yml' 'xcode-select --install' -- .github/; then
  ok
else
  bad "macos jobs set an EULA-accept variable or require interactive Apple-SDK acceptance (issue #412)"
fi

# No host-installed SDK fallback claim anywhere: the only allowed mentions
# deny it on the same line (`never approved`, `never-approved`, or an
# explicit `no`/`No` denial such as `no host-installed SDK fallback`).
# (Self-excluded: this script names the banned form in its own pattern.
# Hermetic tree search with allow-strings, issue #1006.)
if dx_tree_absent --exclude='macos_qualification.sh' --allow='never approved' --allow='never-approved' --allow='no host-installed SDK fallback' --allow='No host-installed SDK fallback' 'host-installed SDK' -- docs/ cli/ tools/ .github/; then
  ok
else
  bad "a host-installed SDK fallback claim appeared (stays never approved, issue #412)"
fi

dx_test_summary "macos arm64 qualification harness"
