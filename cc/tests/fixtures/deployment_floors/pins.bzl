"""Deployment plus execution floors pins.

Contract: `docs/native-toolchains.md#profiles-and-cross-builds`,
`docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/decisions/0014-tested-platform-release-stack.md#decision`.
Fixture: `cc/tests/fixtures/deployment_floors/` via
`bazel run //tools/ci:deployment_floors_qualification`.
"""

# Linux glibc floor: upstream glibc 2.28 symbol floor with libc++ and
# ordinary dynamic glibc linkage. Application glibc implementations
# come from deployment systems, not the link stubs.
GLIBC_FLOOR = "2.28"
GLIBC_SYMBOL_FLOOR = "Upstream glibc 2.28 symbol floor"
GLIBC_CXX_LIB = "libc++"
GLIBC_LINK_MODEL = "ordinary dynamic glibc linkage"
GLIBC_NOTE = "Application glibc implementations come from deployment systems, not the link stubs"

# macOS floors: pinned acquired Apple SDK with SDK libc++ headers and
# system dynamic libc++, plus the upstream deployment default 14.0 as
# the starting point. SDK version is not deployment floor.
MACOS_DEPLOYMENT_DEFAULT = "14.0"
MACOS_SDK_LIBCXX = "SDK libc++ headers"
MACOS_RUNTIME_LIB = "system dynamic libc++"
MACOS_SDK_NOTE = "SDK version is not deployment floor"
APPLE_SDK_IDENTITY = "MacOSX26.5"
APPLE_SDK_SOURCE = "hermetic-llvm v0.8.19 pinned extraction"
APPLE_SDK_HERMETIC_LLVM_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"

# Windows floors: retail dynamic CRT /MD with Microsoft STL plus UCRT
# plus VCRuntime as the starting point. /MT plus debug CRT are not
# assumed interchangeable. MSVC plus redist plus SDK package identities
# are tracked separately and are not automatically the resolved inputs.
WINDOWS_CRT_START = "/MD"
WINDOWS_STL = "Microsoft STL"
WINDOWS_RUNTIME_LIBS = ["UCRT", "VCRuntime"]
WINDOWS_SUPPORT_MSVC = "14.50.35717"
WINDOWS_SUPPORT_REDIST = "14.50.35710"
WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"
WINDOWS_REJECTED_CRT = ["/MT", "debug CRT"]
WINDOWS_CRT_NOTE = "/MT plus debug CRT are not assumed interchangeable"

# Oldest-target vs current-host separation: the oldest-target fixture
OLDEST_TARGET_NOTE = "oldest-target and current-host fixtures run separately"
OLDEST_TARGET_CASES = [
    "glibc 2.28 oldest-target symbols",
    "macOS 14.0 oldest-target deployment",
    "Windows /MD retail oldest-target CRT",
]
CURRENT_HOST_CASES = [
    "seed-host plus CI-runner execution floors",
    "matching native target execution, cross-building alone is insufficient",
]

# Loader dependencies: compiler, clangd and bindgen loader dependencies
# are inspected separately from the link floors. The minimal compiler
# archive alone is not proof that a loadable libclang exists.
LOADER_DEPS = [
    "compiler loader dependencies",
    "clangd loader dependencies",
    "bindgen loader dependencies",
]
LOADER_NOTE = "inspect compiler, clangd and bindgen loader dependencies"
LOADER_MINIMAL_ARCHIVE_NOTE = "the minimal compiler archive alone is not proof that a loadable libclang exists"

# Apple framework subset: check the extracted SDK framework subset for
# completeness. Oldest-OS execution plus framework completeness plus
# licensing remain gates; best-effort gaps never block required-host
# release.
APPLE_FRAMEWORK_NOTE = "Check Apple's extracted SDK framework subset"
APPLE_GATES = [
    "oldest-OS execution",
    "framework completeness",
    "licensing",
]
APPLE_BEST_EFFORT_NOTE = "best-effort gaps never block required-host release"

# Rejected substitutes per the issue alternatives: unpinned floors.
REJECTED_ALTERNATIVES = [
    "unpinned floors",
    "merged oldest-target plus current-host proof",
    "SDK version as deployment floor",
]

# Live proof labels: the seed hello builds on the seed host; the corpus
# target plus the linux corpus composition prove the fixture is wired.
DEPLOYMENT_FLOORS_FIXTURE_CORPUS = "//cc/tests/fixtures/deployment_floors:corpus_starlark"
DEPLOYMENT_FLOORS_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
DEPLOYMENT_FLOORS_LINUX_CORPUS = "//cc/tests/fixtures/linux_corpus:corpus"
