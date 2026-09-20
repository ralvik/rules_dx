"""Windows immutable-lazy acquisition pins (issue #495).

Contract: `docs/native-toolchains.md#windows-acquisition-and-compatibility`.
Fixture: `cc/tests/fixtures/windows_acquisition/` via
`bazel run //tools/ci:windows_acquisition_qualification`.

Decides the acquisition-mechanism slice of the Windows baseline: the
toolchains_msvc backend stays provisional, but its fetch contract is
immutable plus lazy. Mutable fetch stays rejected per the issue
alternatives. Rights plus transport plus interop stay owned under
issues #496/#497/#498, never double-claimed here.
"""

# Prototype identity: head observed, prototype, no published release.
# Declared module `0.0.0` is not a release.
TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"

# Inspected acquisition sources: live channel resolution plus the
# extension plus toolchain registration plus the clang-cl template.
TOOLCHAINS_MSVC_SOURCES = [
    "private/vs_channel_manifest.bzl",
    "extensions.bzl",
    "private/msvc_toolchains_repo.bzl",
    "overlays/toolchain/clang-cl/BUILD.toolchain.tpl",
]

# Acquisition building block, not a compiler toolchain.
WINDOWS_SUPPORT_VERSION = "v0.4.1"
WINDOWS_SUPPORT_COMMIT = "43dce21da77d8cb4a486709e34f1d887e69058c6"

# Inspected windows_support defaults: MSVC plus redist plus SDK package
# tracked separately. They are not automatically the prototype inputs.
WINDOWS_SUPPORT_MSVC = "14.50.35717"
WINDOWS_SUPPORT_REDIST = "14.50.35710"
WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"

# Explicit acceptance stays deliberate through the upstream
# repository-env mechanism, never automatic. The README advertises a
# different variable; the observed API below is the pinned identity.
EULA_ENV_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
EULA_REQUIRED_VALUE = "1"
EULA_MECHANISM = "repository-env"
EULA_README_MISMATCH = True

# Immutability: narrow upstream fixed-manifest plus package-index inputs.
# Live VS channel manifests plus minor-version selectors stay rejected;
# individual payload checksums alone plus a pinned rules commit alone do
# not freeze clean re-resolution.
IMMUTABLE_INPUTS = ["fixed-manifest", "package-index"]
MUTABLE_REJECTED = [
    "live VS channel manifests",
    "minor-version selectors",
    "individual payload checksums only",
    "pinned rules commit only",
]

# Laziness: merely adding the module requires no acceptance and fetches
# no restricted payloads; unrelated workflows fetch nothing; missing
# acceptance breaks only Windows acquisition, never unrelated builds.
# Deferred acceptance failure does not prove laziness: extension
# evaluation already fetches manifests.
LAZY_CONTRACT = [
    "adding the module requires no acceptance",
    "adding the module fetches no restricted payloads",
    "unrelated workflows fetch no Windows payloads",
    "missing acceptance leaves unrelated workflows green",
]

# Rejected substitutes: mutable fetch per the issue, a separate
# project-owned downloader, cross-host inference, installed fallback.
REJECTED_ALTERNATIVES = [
    "mutable fetch",
    "project-owned downloader",
    "cross-host Windows route",
    "installed Build Tools fallback",
]

# Live proof labels: seed hello builds without acceptance; the hermetic
# script plus corpus target prove the fixture is wired.
WINDOWS_ACQUISITION_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_acquisition:corpus_starlark"
WINDOWS_ACQUISITION_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
