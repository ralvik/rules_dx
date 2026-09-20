#!/usr/bin/env bash
# Windows transport plus ABI qualification (issue #497).
#
# Qualifies the transport-plus-ABI slice of the Windows baseline with
# fixture evidence, without claiming a qualified toolchains_msvc backend,
# Windows arm64, or Supported:
# - ABI: toolchains_msvc registration constrains Windows/CPU but not the
#   LLVM MSVC ABI constraint used by rules_rs; the explicit constraint fix
#   pinned in `cc/tests/fixtures/windows_transport/pins.bzl` keeps it from
#   accidentally satisfying GNU/GNULVM targets.
# - paths: clang-cl `/external:I` plus `.lib`/`.obj` bare paths are
#   preserved alongside `/imsvc`; the pinned Rust build-script rebasing
#   source handled `/imsvc` only, so the fix covers both spellings plus
#   both suffixes with declared inputs and no host Visual Studio state.
# - transport: batch wrappers, response-file contents, spaces, SDK system
#   libraries, cc-rs discovery, cc-rs assembly tools, and proc-macro DLLs
#   all ride declared-input fixtures (`transport.expected` plus
#   `response.rsp` plus `batch_wrapper.txt`); compiler-target availability alone
#   is not proof of host-to-target routes or target execution.
# - open with honest records: full toolchains_msvc backend, Microsoft
#   rights (issue #496), prebuilt interop (issue #498), corpus plus floors
#   plus coverage (issues #499/#500/#501), release evidence. Backend stays
#   provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:windows_transport_qualification`,
# following //tools/ci:windows_acquisition_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers (issue #319).
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/windows_transport/pins.bzl"
pins_build="cc/tests/fixtures/windows_transport/BUILD.bazel"
expected="cc/tests/fixtures/windows_transport/transport.expected"
rsp="cc/tests/fixtures/windows_transport/response.rsp"
wrapper="cc/tests/fixtures/windows_transport/batch_wrapper.txt"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture set stays present (issue #497).
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" && -f "$rsp" && -f "$wrapper" ]]; then
  ok
else
  bad "windows transport fixture missing (want $pins plus $pins_build plus $expected plus $rsp plus $wrapper)"
fi

# Pins record the toolchains_msvc prototype identity plus registration sources.
if grep -q -F -e 'TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"' "$pins" &&
  grep -q -F -e 'private/msvc_toolchains_repo.bzl' "$pins" &&
  grep -q -F -e 'overlays/toolchain/clang-cl/BUILD.toolchain.tpl' "$pins"; then
  ok
else
  bad "pins.bzl lost its toolchains_msvc prototype identity plus registration sources under issue #497"
fi

# Pins record the explicit ABI constraint fix (GNU/GNULVM rejected).
if grep -q -F -e 'LLVM MSVC ABI constraint' "$pins" &&
  grep -q -F -e 'GNU' "$pins" &&
  grep -q -F -e 'GNULVM' "$pins" &&
  grep -q -F -e 'explicit constraint fix so it cannot accidentally satisfy GNU/GNULVM targets' "$pins"; then
  ok
else
  bad "pins.bzl lost its explicit ABI constraint fix with GNU/GNULVM rejection under issue #497"
fi

# Pins record the rules_rs triples identity behind the ABI constraint.
if grep -q -F -e 'RULES_RS_VERSION = "v0.0.109"' "$pins" &&
  grep -q -F -e 'b55b132af0c9951807c926768e40222330348632' "$pins" &&
  grep -q -F -e 'rs/platforms/triples.bzl' "$pins"; then
  ok
else
  bad "pins.bzl lost its rules_rs triples identity behind the ABI constraint under issue #497"
fi

# Pins record the rebasing source plus /imsvc vs /external:I plus .lib/.obj.
if grep -q -F -e 'e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde' "$pins" &&
  grep -q -F -e 'cargo/private/cargo_build_script.bzl' "$pins" &&
  grep -q -F -e '"/imsvc"' "$pins" &&
  grep -q -F -e '"/external:I"' "$pins" &&
  grep -q -F -e '".lib"' "$pins" &&
  grep -q -F -e '".obj"' "$pins" &&
  grep -q -F -e 'bare-path handling covers .lib plus .obj' "$pins"; then
  ok
else
  bad "pins.bzl lost its rebasing source plus /imsvc vs /external:I plus .lib/.obj handling under issue #497"
fi

# Pins record all seven transport cases with the declared-input contract.
if grep -q -F -e '"batch wrappers"' "$pins" &&
  grep -q -F -e '"response files"' "$pins" &&
  grep -q -F -e '"spaces"' "$pins" &&
  grep -q -F -e '"SDK system libraries"' "$pins" &&
  grep -q -F -e '"cc-rs discovery"' "$pins" &&
  grep -q -F -e '"cc-rs assembly tools"' "$pins" &&
  grep -q -F -e '"proc-macro DLLs"' "$pins" &&
  grep -q -F -e 'declared-input fixtures without host Visual Studio state' "$pins"; then
  ok
else
  bad "pins.bzl lost its seven transport cases with the declared-input contract under issue #497"
fi

# Pins record the rejected proof substitute plus host-state denial.
if grep -q -F -e 'compiler-target availability as proof' "$pins" &&
  grep -q -F -e 'host Visual Studio state' "$pins" &&
  grep -q -F -e '"installed Build Tools fallback"' "$pins"; then
  ok
else
  bad "pins.bzl lost its compiler-target-availability rejection plus host-state denial under issue #497"
fi

# Pins keep the backend provisional with owned gaps elsewhere.
if grep -q -F -e 'Backend stays provisional' "$pins" &&
  grep -q -F -e '"#498"' "$pins" &&
  grep -q -F -e '"#499"' "$pins" &&
  grep -q -F -e '"#500"' "$pins" &&
  grep -q -F -e '"#501"' "$pins"; then
  ok
else
  bad "pins.bzl lost its provisional-backend record with owned gaps under issue #497"
fi

# Native plan owns the qualified transport record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #497' "$native" &&
  grep -q -F -e 'windows_transport_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/windows_transport/pins.bzl' "$native" &&
  grep -q -F -e 'Does Windows native transport preserve all inputs and ABI selection?' "$native" &&
  grep -q -F -e 'compiler-target availability alone is not proof' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified transport record with fixtures under issue #497"
fi

# Support matrix toolchains_msvc row owns the transport qualified record.
if grep -q -F -e 'transport plus ABI qualified seed-only under issue #497' "$matrix" &&
  grep -q -F -e 'windows_transport_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/windows_transport/pins.bzl' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its transport qualified record under issue #497"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "windows_transport_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:windows_transport_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the windows_transport_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under #497.
if grep -q -F -e 'windows_transport_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #497' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:windows_transport_qualification' "$verify" &&
  grep -q -F -e '`windows_transport_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #497 transport qualified record"
fi

# Fixture expected covers all seven declared-input cases.
if grep -q -F -e 'batch wrappers' "$expected" &&
  grep -q -F -e 'response files' "$expected" &&
  grep -q -F -e 'spaces' "$expected" &&
  grep -q -F -e 'SDK system libraries' "$expected" &&
  grep -q -F -e 'cc-rs discovery' "$expected" &&
  grep -q -F -e 'cc-rs assembly tools' "$expected" &&
  grep -q -F -e 'proc-macro DLLs' "$expected"; then
  ok
else
  bad "transport.expected lost declared-input cases (want batch plus response plus spaces plus SDK plus cc-rs discovery/assembly plus proc-macro DLLs)"
fi

# Response fixture preserves both include spellings plus both suffixes plus
# spaces plus SDK libraries with relative declared inputs only.
if grep -q -F -e '/external:I' "$rsp" &&
  grep -q -F -e '/imsvc' "$rsp" &&
  grep -q -F -e '.lib' "$rsp" &&
  grep -q -F -e '.obj' "$rsp" &&
  grep -q -F -e 'with spaces' "$rsp" &&
  grep -q -F -e 'kernel32.lib' "$rsp" &&
  grep -q -F -e 'without host Visual Studio state' "$rsp" &&
  ! grep -q -F -e 'C:\\Program Files' "$rsp" &&
  ! grep -q -F -e 'C:/Program Files' "$rsp"; then
  ok
else
  bad "response.rsp lost its preserved inputs (want /external:I plus /imsvc plus .lib/.obj plus spaces plus SDK libs, no absolute host paths)"
fi

# Batch wrapper forwards the response file verbatim with no host state.
if grep -q -F -e '@response.rsp' "$wrapper" &&
  grep -q -F -e 'lld-link' "$wrapper" &&
  grep -q -F -e 'without host Visual Studio state' "$wrapper" &&
  ! grep -q -F -e 'C:\\Program Files' "$wrapper" &&
  ! grep -q -F -e 'VsDevCmd' "$wrapper"; then
  ok
else
  bad "batch_wrapper.txt lost its verbatim response forwarding without host state (want lld-link @response.rsp, no VsDevCmd or absolute host paths)"
fi

# Live proof: declared-input fixtures need no host state, so the seed hello
# plus the fixture corpus build green on the seed host.
if bazel build //cc/tests/fixtures/hello:hello //cc/tests/fixtures/windows_transport/... --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "seed hello plus transport fixture build failed (want green without host Visual Studio state, issue #497)"
fi

dx_test_summary "windows transport qualification harness"
