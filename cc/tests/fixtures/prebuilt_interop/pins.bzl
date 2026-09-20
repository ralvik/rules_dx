"""Prebuilt interop pins (issue #498).

Contract: `docs/native-toolchains.md#windows-acquisition-and-compatibility`,
`docs/native-toolchains.md#profiles-and-cross-builds`,
`docs/native-toolchains.md#qualification-questions-and-delivery`.
Fixture: `cc/tests/fixtures/prebuilt_interop/` via
`bazel run //tools/ci:prebuilt_interop_qualification`.

Decides the interop slice of the native baseline: explicit STL plus CRT
plus linker plus library combos are defined with fixture evidence recorded
here and in the owning docs. Compiler-target availability alone plus LLVM
ancestry alone stay rejected per the issue alternatives. Single-combo
proof stays rejected: at least the Windows retail plus Linux comparison
combos below must pass. Backends stay provisional; transport plus corpus
plus floors plus coverage stay owned under issues #497/#499/#500/#501,
never double-claimed here.
"""

# Windows starting point: retail dynamic CRT `/MD` with Microsoft STL
# plus UCRT plus VCRuntime. `/MT` plus debug CRT are not assumed
# interchangeable. Aligned Rust CRT mode plus iterator-debug settings plus
# system libraries plus redistributable deployment are the compat gate.
WINDOWS_CRT_START = "/MD"
WINDOWS_STL = "Microsoft STL"
WINDOWS_RUNTIME_LIBS = ["UCRT", "VCRuntime"]
WINDOWS_REDIST_NOTE = "Align Rust CRT mode, iterator-debug settings, system libraries and redistributable deployment"

# Explicit Windows STL/CRT/linker/library combos (single-combo rejected).
# Host-to-target build routes plus target execution qualify separately;
# compiler-target availability alone is not proof. Independently compiled
# MSVC static/import libraries plus DLLs incl STL values carry the
# exceptions plus RTTI plus allocation-ownership proof below.
WINDOWS_COMBOS = [
    "clang-cl + Microsoft STL + /MD + lld-link + static .lib",
    "clang-cl + Microsoft STL + /MD + lld-link + import .lib + DLL",
    "cl.exe compat + Microsoft STL + /MD + link + static .lib (diagnosis only, never a second default)",
]

# Windows compat limits: match compiler/linker/redist requirements, never
# arbitrary objects. Missing mandatory evidence blocks Windows support.
WINDOWS_COMPAT_LIMITS = "https://learn.microsoft.com/en-us/cpp/porting/binary-compat-2015-2017?view=msvc-170"
WINDOWS_CLANG_COMPAT = "https://clang.llvm.org/docs/MSVCCompatibility.html"
WINDOWS_REJECTED_OBJECTS = ["/GL", "/LTCG", "arbitrary CRT combinations", "vendor libraries"]

# Linux GNU C++ prebuilt compatibility: explicit dynamic libstdc++
# alternative on Linux glibc only, never default libc++ substitution.
# Upstream supports it only on Linux glibc. Patched Rust runtime selection
# plus actual GCC ABI/library fixtures must pass before claiming it.
LINUX_DEFAULT_CXX_LIB = "libc++"
LINUX_ALT_CXX_LIB = "explicit dynamic libstdc++ (Linux glibc only)"
LINUX_ALT_SCOPE = "Upstream supports it only on Linux glibc"
LINUX_GCC_ABI_NOTE = "Patched Rust runtime selection and actual GCC ABI/library fixtures must pass"

# Linking plus ABI boundaries: ordinary object linking without
# cross-language LTO initially. Shared-runtime deployment, exceptions,
# RTTI, allocation ownership plus ABI boundaries qualify explicitly here.
LINK_MODEL = "ordinary object linking without cross-language LTO"
ABI_PROOF = [
    "shared-runtime deployment",
    "exceptions across the boundary",
    "RTTI across the boundary",
    "allocation ownership (new pairs with delete on the owning side)",
    "ABI boundaries",
]

# Mixed Rust plus C/C++ wiring: the decided single-graph CXX identity
# (`cxx == cxxbridge-cmd == 1.0.200` from the single `crates` graph with
# `@crates//:cxxbridge-cmd`, never a `cxx.rs` second graph, decided under
# issue #474) is the Rust/C++ bridge shape; full `cxxbridge-cmd`
# execution plus corpus wiring stays owned under issue #499, never
# double-claimed here. Path transport (batch wrappers, response files,
# `/external:I` vs `/imsvc`, `.lib`/`.obj` bare paths, spaces, SDK
# libraries, cc-rs discovery/assembly, proc-macro DLLs) stays owned under
# issue #497 without host Visual Studio state.
CXX_IDENTITY_VERSION = "1.0.200"
CXXBRIDGE_CMD_LABEL = "@crates//:cxxbridge-cmd"
MIXED_NOTE = "mixed Rust/C/C++ host-to-target plus target execution qualify separately"

# Rejected substitutes per the issue alternatives: single-combo proof,
# compiler-target availability as proof, LLVM ancestry as proof.
REJECTED_ALTERNATIVES = [
    "single-combo proof",
    "compiler-target availability as proof",
    "LLVM ancestry as proof",
]

# Live proof labels: the interop lib plus test below prove the Linux
# STL/exception/RTTI/ownership shape on the seed host; the cxx_identity
# bridge plus the stable-stack hellos prove mixed Rust/C/C++ composition.
PREBUILT_INTEROP_FIXTURE_LIB = "//cc/tests/fixtures/prebuilt_interop:interop"
PREBUILT_INTEROP_FIXTURE_TEST = "//cc/tests/fixtures/prebuilt_interop:interop_test"
PREBUILT_INTEROP_MIXED_CXX = "//rust/tests/fixtures/cxx_identity:bridge"
PREBUILT_INTEROP_MIXED_RUST = "//rust/tests/fixtures/cxx_identity:cxx_identity"
PREBUILT_INTEROP_STABLE_CC = "//cc/tests/fixtures/hello:hello"
PREBUILT_INTEROP_STABLE_RUST = "//rust/tests/fixtures/hello:hello"
PREBUILT_INTEROP_FIXTURE_CORPUS = "//cc/tests/fixtures/prebuilt_interop:corpus_starlark"
