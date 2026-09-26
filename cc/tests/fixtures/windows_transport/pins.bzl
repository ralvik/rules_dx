TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_REGISTRATION = "private/msvc_toolchains_repo.bzl"
TOOLCHAINS_MSVC_TEMPLATE = "overlays/toolchain/clang-cl/BUILD.toolchain.tpl"

RULES_RS_VERSION = "v0.0.109"
RULES_RS_COMMIT = "b55b132af0c9951807c926768e40222330348632"
RULES_RS_TRIPLES = "rs/platforms/triples.bzl"
ABI_CONSTRAINT = "LLVM MSVC ABI constraint"
ABI_REJECTED = ["GNU", "GNULVM"]
ABI_FIX = "explicit constraint fix so it cannot accidentally satisfy GNU/GNULVM targets"

PATCHED_RULES_RUST_COMMIT = "e9dd49f22cfa43c75ba30cd9d9bb7d8bdc459dde"
REBASE_SOURCE = "cargo/private/cargo_build_script.bzl"
REBASE_HANDLES = ["/imsvc"]
REBASE_FIXES = ["/external:I", ".lib", ".obj"]
REBASE_NOTE = "bare-path handling covers .lib plus .obj without host Visual Studio state"

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

REJECTED_ALTERNATIVES = [
    "compiler-target availability as proof",
    "host Visual Studio state",
    "installed Build Tools fallback",
]

BACKEND_NOTE = "Backend stays provisional"
OWNED_GAPS = ["#496", "#498", "#499", "#500", "#501"]

WINDOWS_TRANSPORT_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_transport:corpus_starlark"
WINDOWS_TRANSPORT_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
