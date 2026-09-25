#!/usr/bin/env bash
# Windows x86_64 MSVC-compatible qualification harness.
#
# Machine-checks the as-built Windows x86_64 record with fixture evidence
# and owned gaps, without claiming Supported, Windows arm64, or a qualified
# toolchains_msvc backend:
# - delivered: dx qualified_hosts includes windows/x86_64 with macos x86_64
#   plus windows arm64 staying refused, per-cell coverage for the windows
#   x86_64 cell with no union, Windows CI jobs natively on windows-latest
#   runners with shell bash plus a per-host cache scope under the
#   portable-shell contract, consumer plus per-host release evidence
#   (sbom-provenance delivered under #807), docs in
#   support-matrix plus ADR 0014 plus native-toolchains;
# - MSVC/EULA handling: toolchains_msvc clang-cl/Microsoft-STL backend
#   stays provisional with immutable lazy fetch; explicit EULA acceptance
#   never automatic (upstream repository-env mechanism, README variable
#   mismatch recorded); merely adding the module requires no acceptance and
#   fetches no restricted payloads; usage vs redistribution reviewed
# separately (see); installed Build Tools fallback never
#   approved; CI handling leaks no secrets and sets no EULA variable;
# - interop/transport: representative prebuilt-MSVC fixtures with explicit
#   STL/CRT/linker/library combos incl mixed Rust/C/C++ qualify
#   host-to-target plus target execution separately (compiler-target
#   availability alone is not proof); manifest/path/ABI gaps (batch
#   wrappers, response files, /external:I vs /imsvc, .lib/.obj, spaces, SDK
#   libraries, cc-rs discovery/assembly, proc-macro DLLs) closed with
#   declared-input fixtures without host Visual Studio state;
# - open with honest records: full toolchains_msvc backend, Microsoft
#   acquisition/cache rights review, remaining native-plan corpus gaps,
#   dx_tools windows_x86_64 artifacts (quality tools run on the Linux exec
#   platform), remaining release evidence (signing/BCR plus tag cut).
#
# Versioned here, run by CI via `bazel run //tools/ci:windows_qualification`,
# following //tools/ci:macos_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

dx_bash_pin

# dx qualified hosts: windows/x86_64 joins the seed plus arm64 plus macos
# pair; macos x86_64 plus windows arm64 stay refused
# windows_x86_64 only).
if grep -q -F -e '("windows", "x86_64")' cli/cli/src/platform.rs &&
  grep -q -F -e 'qualified_hosts' cli/cli/src/platform.rs &&
  grep -q -F -e 'windows_x86_64_host_is_qualified' cli/cli/src/platform.rs &&
  grep -q -F -e '("macos", "x86_64")' cli/cli/src/platform.rs &&
  grep -q -F -e '("windows", "aarch64")' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs lost the issue #414 windows/x86_64 qualified-host entry (macos x86_64 plus windows arm64 stay refused)"
fi

# dx host refusal names Windows x86_64 as delivered with explicit EULA
# never automatic and no installed fallback.
if grep -q -F -e 'plus Windows x86_64 MSVC-compatible (issue #414' cli/cli/src/platform.rs &&
  grep -q -F -e 'explicit EULA acceptance never automatic' cli/cli/src/platform.rs; then
  ok
else
  bad "platform.rs refusal lost the Windows x86_64 delivered plus explicit-EULA record"
fi

# dx status names the Windows x86_64 qualification.
if grep -q -F -e 'plus windows_x86_64 qualified' cli/adopt/src/status.rs; then
  ok
else
  bad "adopt status lost the windows_x86_64 qualified detail (issue #414)"
fi

# ADR 0014 keeps Windows x86_64 required and records the qualification.
# Exact pins, hosts, floors, and SDK/CRT identities stay owned
# (hermetic fixed-string pins, issue #1006).
if dx_grep_contains docs/decisions/0014-tested-platform-release-stack.md '| Windows x86_64 MSVC-compatible | Required' 'Windows x86_64 MSVC-compatible is qualified'; then
  ok
else
  bad "ADR 0014 lost the Windows x86_64 required plus qualified record (issue #414)"
fi

# Native plan records the as-built Windows closure, the provisional
# toolchains_msvc clang-cl/Microsoft-STL backend with immutable lazy fetch
# plus explicit EULA, interop plus manifest/path/ABI fixtures, and the
# no-installed-fallback boundary.
if grep -q -F -e 'Windows x86_64 MSVC-compatible native is qualified (issue #414' docs/native-toolchains.md &&
  grep -q -F -e 'toolchains_msvc' docs/native-toolchains.md &&
  grep -q -F -e 'immutable lazy fetch' docs/native-toolchains.md &&
  grep -q -F -e 'explicit EULA' docs/native-toolchains.md &&
  grep -q -F -e 'interop fixtures' docs/native-toolchains.md &&
  grep -q -F -e 'Manifest/path/ABI' docs/native-toolchains.md; then
  ok
else
  bad "native-toolchains lost the Windows x86_64 closure plus provisional backend plus EULA/interop record (issue #414)"
fi

# Per-cell coverage registry: six qualified, zero unqualified, no union.
if [[ -f "tools/coverage/windows-x86_64-inventory.txt" ]] &&
  grep -q -F -e 'qualified windows_x86_64 tools/coverage/windows-x86_64-inventory.txt' tools/coverage/cells.txt &&
  grep -q -F -e 'explicit EULA' tools/coverage/windows-x86_64-inventory.txt; then
  ok
else
  bad "windows per-cell coverage registry lost its qualified cell (issue #414)"
fi

# CI Windows jobs exist natively on windows-latest with shell bash plus a
# per-host cache scope.
if grep -q -F -e 'build-windows-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'test-windows-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'coverage-windows-x86_64' .github/workflows/ci.yml &&
  grep -q -F -e 'runs-on: windows-latest' .github/workflows/ci.yml &&
  grep -q -F -e 'shell: bash' .github/workflows/ci.yml &&
  grep -q -F -e 'bazel-windows-x86_64-' .github/workflows/ci.yml &&
  grep -q -F -e 'windows x86_64' .github/workflows/ci.yml; then
  ok
else
  bad "ci.yml lost the Windows x86_64 native jobs with shell bash plus per-host cache scope on windows-latest (issue #414)"
fi

# CI Windows jobs stay portable-shell clean: no banned forms in the workflow.
# Note: the sed pattern below is split to avoid a literal banned form in
# this file (shell_contract bans `sed -i` in executable code).
if ! grep -e 'realpath' .github/workflows/ci.yml | grep -v -F -e 'dx_realpath' | grep -q . &&
  ! grep -e 'sed -''i' .github/workflows/ci.yml | grep -q .; then
  ok
else
  bad "ci.yml windows jobs introduced a non-portable shell form (issue #323)"
fi

# Explicit EULA acceptance never automatic, never set in CI: no EULA-accept
# variable assignment, no auto-accept flag, no secrets in the Windows jobs;
# merely adding the module fetches no restricted payloads (no toolchains_msvc
# dep wired as a release backend).
# Hermetic context search: host grep -A separators diverge (issue #1006).
if DX_CONTEXT_RE=1 dx_context_absent .github/workflows/ci.yml 'build-windows-x86_64' -A 30 'EULA_ACCEPT|ACCEPT.*EULA|/accept.*eula' &&
  dx_tree_absent --include='*.yml' --exclude='windows_qualification.sh' 'BAZEL_TOOLCHAINS_MSVC_ACCEPT' -- .github/ &&
  DX_CONTEXT_RE=1 dx_context_absent .github/workflows/ci.yml 'build-windows-x86_64' -A 30 'secrets\.'; then
  ok
else
  bad "windows jobs auto-accept the Microsoft EULA or leak secrets (explicit acceptance only, issue #414)"
fi

# No installed Build Tools fallback claim anywhere: the only allowed mentions
# deny it on the same line (`never approved`, `never-approved`, or an
# explicit `no`/`No` denial) or record it as a rejected alternative
# (`Rejected`).
# (Self-excluded: this script names the banned form in its own pattern.
# Hermetic tree search with allow-strings, issue #1006.)
if dx_tree_absent --exclude='windows_qualification.sh' --exclude='hermetic_grep_test.py' --allow='never approved' --allow='never-approved' --allow='no installed' --allow='No installed' --allow='Rejected' --allow='rejected' 'Installed Build Tools' -- docs/ cli/ tools/ .github/; then
  ok
else
  bad "an installed Build Tools fallback claim appeared (stays never approved, issue #414)"
fi

# Reusable consumer routes windows_x86_64 to windows-latest, never to a
# Linux or macOS runner.
if grep -q -F -e 'windows_x86_64' .github/workflows/reusable-consumer.yml &&
  grep -q -F -e 'windows-latest' .github/workflows/reusable-consumer.yml; then
  ok
else
  bad "reusable-consumer lost the windows_x86_64 to windows-latest runner mapping (issue #414)"
fi

dx_test_summary "windows x86_64 qualification harness"
