#!/usr/bin/env bash
# Prebuilt interop qualification harness.
#
# Defines plus proves the prebuilt interop slice of the native baseline
# with fixture evidence, without claiming a qualified hermetic-llvm
# backend, a qualified toolchains_msvc backend, Windows support, or
# Supported:
# - Windows combos: explicit STL/CRT/linker/library combos pinned in
#   `cc/tests/fixtures/prebuilt_interop/pins.bzl` (clang-cl plus Microsoft
#   STL plus retail dynamic CRT `/MD` as the starting point with
#   UCRT/VCRuntime plus redist alignment; static .lib plus import .lib plus
#   DLL shapes; cl.exe compat as diagnosis only, never a second default).
#   `/MT` plus debug CRT never interchangeable; `/GL`/`/LTCG` plus arbitrary
#   CRT combinations plus vendor libraries rejected under the Microsoft
#   binary-compat plus Clang MSVC-compat limits.
# - Linux comparison: default libc++ vs explicit dynamic libstdc++
#   alternative on Linux glibc only (never default substitution); patched
#   Rust runtime selection plus GCC ABI/library fixtures must pass.
# - ABI proof: ordinary object linking without cross-language LTO
#   initially; shared-runtime deployment, exceptions, RTTI, allocation
#   ownership plus ABI boundaries proven by the interop lib plus test.
# - mixed Rust/C/C++: decided single-graph CXX identity (`cxx ==
# cxxbridge-cmd == 1.0.200` from the single `crates` graph,)
#   with host-to-target plus target execution qualifying separately;
#   compiler-target availability alone plus LLVM ancestry alone rejected;
#   single-combo proof rejected. Path transport stays owned under issue
# , corpus wiring, never double-claimed here.
# - open with honest records: full backends, transport,
# corpus plus floors plus coverage, release
#   evidence. Backends stay provisional.
#
# Versioned here, run by CI via `bazel run //tools/ci:prebuilt_interop_qualification`,
# following //tools/ci:acquisition_rights_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cc/tests/fixtures/prebuilt_interop/pins.bzl"
pins_build="cc/tests/fixtures/prebuilt_interop/BUILD.bazel"
interop_h="cc/tests/fixtures/prebuilt_interop/interop.h"
interop_cc="cc/tests/fixtures/prebuilt_interop/interop.cc"
interop_test="cc/tests/fixtures/prebuilt_interop/interop_test.cc"
native="docs/native-toolchains.md"
matrix="docs/product/support-matrix.md"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$interop_h" && -f "$interop_cc" && -f "$interop_test" ]]; then
  ok
else
  bad "prebuilt interop fixture missing (want $pins plus $pins_build plus interop.h/cc/test.cc)"
fi

# Pins record the Windows retail CRT start plus STL plus runtime plus redist alignment.
if grep -q -F -e 'WINDOWS_CRT_START = "/MD"' "$pins" &&
  grep -q -F -e 'WINDOWS_STL = "Microsoft STL"' "$pins" &&
  grep -q -F -e '"UCRT"' "$pins" &&
  grep -q -F -e '"VCRuntime"' "$pins" &&
  grep -q -F -e 'Align Rust CRT mode, iterator-debug settings' "$pins"; then
  ok
else
  bad "pins.bzl lost its Windows /MD plus Microsoft STL plus UCRT/VCRuntime plus redist alignment under issue #498"
fi

# Pins record explicit Windows STL/CRT/linker/library combos (single-combo rejected).
if grep -q -F -e 'clang-cl + Microsoft STL + /MD + lld-link + static .lib' "$pins" &&
  grep -q -F -e 'clang-cl + Microsoft STL + /MD + lld-link + import .lib + DLL' "$pins" &&
  grep -q -F -e 'cl.exe compat + Microsoft STL + /MD' "$pins" &&
  grep -q -F -e 'diagnosis only, never a second default' "$pins" &&
  grep -q -F -e 'host-to-target plus target execution qualify separately' "$pins"; then
  ok
else
  bad "pins.bzl lost its explicit Windows STL/CRT/linker/library combos plus host-to-target separation under issue #498"
fi

# Pins record the compat limits plus the arbitrary-object rejection.
if grep -q -F -e 'binary-compat-2015-2017' "$pins" &&
  grep -q -F -e 'MSVCCompatibility.html' "$pins" &&
  grep -q -F -e '"/GL"' "$pins" &&
  grep -q -F -e '"/LTCG"' "$pins" &&
  grep -q -F -e 'arbitrary CRT combinations' "$pins" &&
  grep -q -F -e 'vendor libraries' "$pins"; then
  ok
else
  bad "pins.bzl lost its Microsoft plus Clang compat limits plus /GL//LTCG/CRT/vendor rejection under issue #498"
fi

# Pins record the Linux default vs explicit libstdc++ alternative plus the GCC ABI gate.
if grep -q -F -e 'LINUX_DEFAULT_CXX_LIB = "libc++"' "$pins" &&
  grep -q -F -e 'explicit dynamic libstdc++ (Linux glibc only)' "$pins" &&
  grep -q -F -e 'Upstream supports it only on Linux glibc' "$pins" &&
  grep -q -F -e 'Patched Rust runtime selection and actual GCC ABI/library fixtures must pass' "$pins"; then
  ok
else
  bad "pins.bzl lost its Linux libc++ vs explicit libstdc++ plus GCC ABI gate under issue #498"
fi

# Pins record ordinary linking plus the shared-runtime/exception/RTTI/ownership/ABI proof.
if grep -q -F -e 'ordinary object linking without cross-language LTO' "$pins" &&
  grep -q -F -e 'shared-runtime deployment' "$pins" &&
  grep -q -F -e 'exceptions across the boundary' "$pins" &&
  grep -q -F -e 'RTTI across the boundary' "$pins" &&
  grep -q -F -e 'allocation ownership (new pairs with delete on the owning side)' "$pins" &&
  grep -q -F -e 'ABI boundaries' "$pins"; then
  ok
else
  bad "pins.bzl lost its ordinary-linking plus shared-runtime/exception/RTTI/ownership/ABI proof under issue #498"
fi

# Pins record the mixed Rust/C/C++ identity without double-claiming transport plus corpus.
if grep -q -F -e 'CXX_IDENTITY_VERSION = "1.0.200"' "$pins" &&
  grep -q -F -e '@crates//:cxxbridge-cmd' "$pins" &&
  grep -q -F -e 'mixed Rust/C/C++ host-to-target plus target execution qualify separately' "$pins" &&
  grep -q -F -e 'stays owned under' "$pins" &&
  grep -q -F -e 'issue #497' "$pins" &&
  grep -q -F -e 'issue #499' "$pins"; then
  ok
else
  bad "pins.bzl lost its mixed Rust/C/C++ CXX identity plus #497/#499 ownership under issue #498"
fi

# Pins record the rejected substitutes (single-combo plus availability plus ancestry).
if grep -q -F -e '"single-combo proof"' "$pins" &&
  grep -q -F -e '"compiler-target availability as proof"' "$pins" &&
  grep -q -F -e '"LLVM ancestry as proof"' "$pins"; then
  ok
else
  bad "pins.bzl lost its single-combo plus availability plus ancestry rejection under issue #498"
fi

# Fixture BUILD composes the interop lib plus test through the wrapper contracts.
if grep -q -F -e 'name = "interop"' "$pins_build" &&
  grep -q -F -e 'name = "interop_test"' "$pins_build" &&
  grep -q -F -e 'interop.cc' "$pins_build" &&
  grep -q -F -e 'interop.h' "$pins_build" &&
  grep -q -F -e 'interop_test.cc' "$pins_build"; then
  ok
else
  bad "prebuilt_interop BUILD.bazel lost its interop lib plus test composition (want interop plus interop_test)"
fi

# Native plan owns the qualified interop record plus fixture proof.
if grep -q -F -e 'qualified seed-only under issue #498' "$native" &&
  grep -q -F -e 'prebuilt_interop_qualification' "$native" &&
  grep -q -F -e 'cc/tests/fixtures/prebuilt_interop/pins.bzl' "$native" &&
  grep -q -F -e 'Which prebuilt native libraries interoperate?' "$native" &&
  grep -q -F -e 'single-combo proof rejected' "$native"; then
  ok
else
  bad "docs/native-toolchains.md lost its qualified interop record with fixtures under issue #498"
fi

# Support matrix owns the qualified interop record with no Supported claim.
if grep -q -F -e 'qualified seed-only under issue #498' "$matrix" &&
  grep -q -F -e 'prebuilt_interop_qualification' "$matrix" &&
  grep -q -F -e 'cc/tests/fixtures/prebuilt_interop/pins.bzl' "$matrix" &&
  grep -q -F -e 'single-combo proof rejected' "$matrix"; then
  ok
else
  bad "docs/product/support-matrix.md lost its qualified interop record under issue #498"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "prebuilt_interop_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:prebuilt_interop_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the prebuilt_interop_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'prebuilt_interop_qualification' "$verify" &&
  grep -q -F -e 'qualified seed-only under #498' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:prebuilt_interop_qualification' "$verify" &&
  grep -q -F -e '`prebuilt_interop_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix lost its #498 interop qualified record"
fi

# Live proof: the interop lib builds on the seed host.
if bazel build //cc/tests/fixtures/prebuilt_interop:interop --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "prebuilt interop lib failed to build (want interop green on the seed host, issue #498)"
fi

# Live proof: the interop ABI test passes (STL plus exceptions plus RTTI plus ownership).
if bazel test //cc/tests/fixtures/prebuilt_interop:interop_test --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "prebuilt interop test failed (want STL plus exceptions plus RTTI plus ownership green, issue #498)"
fi

# Live proof: mixed Rust/C/C++ composition stays green (single-graph bridge, never a second graph).
if bazel build //rust/tests/fixtures/cxx_identity:bridge //rust/tests/fixtures/cxx_identity:cxx_identity --noshow_progress >/dev/null 2>&1; then
  ok
else
  bad "mixed Rust/C/C++ bridge failed to build (want cxx_identity bridge plus lib green, issue #498)"
fi

dx_test_summary "prebuilt interop qualification harness"
