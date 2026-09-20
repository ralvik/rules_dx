"""Windows transport plus ABI pins.
Contract: `docs/native-toolchains.md#windows-acquisition-and-compatibility`.
Fixture: `cc/tests/fixtures/windows_transport/` via `bazel run //tools/ci:windows_transport_qualification`.
"""

# Prototype identity: head observed, prototype, no published release.
TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_REGISTRATION = "private/msvc_toolchains_repo.bzl"
TOOLCHAINS_MSVC_TEMPLATE = "overlays/toolchain/clang-cl/BUILD.toolchain.tpl"

# ABI selection: registration constrains Windows/CPU but not the LLVM MSVC
# ABI constraint used by rules_rs. Qualify an explicit constraint fix so it
# cannot accidentally satisfy GNU/GNULVM targets.
RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"
RULES_RS_TRIPLES = "rs/platforms/triples.bzl"
ABI_CONSTRAINT = "LLVM MSVC ABI constraint"
ABI_REJECTED = ["GNU", "GNULVM"]
ABI_FIX = "explicit constraint fix so it cannot accidentally satisfy GNU/GNULVM targets"

# Path rebasing: pinned Rust build-script rebasing code handles /imsvc but
# not /external:I, and omits .lib/.obj from bare-path handling. The fix
# preserves both include spellings plus both link suffixes.
PATCHED_RULES_RUST_COMMIT = "e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde"
REBASE_SOURCE = "cargo/private/cargo_build_script.bzl"
REBASE_HANDLES = ["/imsvc"]
REBASE_FIXES = ["/external:I", ".lib", ".obj"]
REBASE_NOTE = "bare-path handling covers .lib plus .obj without host Visual Studio state"

# Transport: every input rides declared-input fixtures without host Visual
# Studio state. Batch linker wrappers plus response-file contents plus
# spaces plus SDK system libraries plus cc-rs discovery plus cc-rs assembly
# tools plus proc-macro DLLs stay covered, never inferred.
TRANSPORT_CASES = [
    "batch wrappers",
    "response files",
    "spaces",
    "SDK system libraries",
    "cc-rs discovery",
    "cc-rs assembly tools",
    "proc-macro DLLs",
]
TRANSPORT_CONTRACT = "declared-input fixtures without host Visual Studio state"

# Rejected substitutes: compiler-target availability alone is not proof that
# host-to-target routes plus target execution work; installed fallback plus
# host state plus ad-hoc compose stay rejected.
REJECTED_ALTERNATIVES = [
    "compiler-target availability as proof",
    "host Visual Studio state",
    "installed Build Tools fallback",
]

# Backend stays provisional; rights plus interop plus corpus plus floors
# plus coverage stay owned.
BACKEND_NOTE = "Backend stays provisional"
OWNED_GAPS = ["#496", "#498", "#499", "#500", "#501"]

# Live proof labels: seed hello builds without host state; the hermetic
# script plus corpus target prove the fixture is wired.
WINDOWS_TRANSPORT_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_transport:corpus_starlark"
WINDOWS_TRANSPORT_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
