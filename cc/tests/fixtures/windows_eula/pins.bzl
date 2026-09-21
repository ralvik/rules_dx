"""Windows EULA acknowledgement UX pins.

Contract: `docs/native-toolchains.md#windows-acquisition-and-compatibility`.
Fixture: `cc/tests/fixtures/windows_eula/` via
`bazel run //tools/ci:windows_eula_qualification`.
"""

# Selected upstream acquisition route: toolchains_msvc prototype head,
# never a published release. Declared module `0.0.0` is not a release.
TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"
TOOLCHAINS_MSVC_EXTENSION = "extensions.bzl"
TOOLCHAINS_MSVC_SOURCES = [
    "private/vs_channel_manifest.bzl",
    "extensions.bzl",
    "private/msvc_toolchains_repo.bzl",
]

# Acquisition building block, not a compiler toolchain.
WINDOWS_SUPPORT_VERSION = "v0.4.1"
WINDOWS_SUPPORT_COMMIT = "43dce21da77d8cb4a486709e34f1d887e69058c6"
WINDOWS_SUPPORT_MSVC = "14.50.35717"
WINDOWS_SUPPORT_REDIST = "14.50.35710"
WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"

# Acknowledgement variable/mechanism for the selected route: the upstream
# extension reads repository environment, never a rules_dx setup command.
EULA_ENV_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
EULA_REQUIRED_VALUE = "1"
EULA_MECHANISM = "repository-env"
EULA_README_MISMATCH = True

# Deliberate acknowledgement UX: consumer exports the variable plus passes
# it as repository environment (export VAR=1 plus --repo_env=VAR=1) after
# reviewing the applicable Microsoft terms. Never automatic via .bazelrc or
# wrapper defaults, never bypassing upstream controls, never pre-seeded
# payloads or patched extension checks.
ACK_HOW = "export BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1 plus --repo_env=BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1"
ACK_NEVER_AUTOMATIC = "never automatic via .bazelrc or wrapper defaults"
ACK_NEVER_BYPASS = "never bypassing upstream controls"

# Fail-closed: missing acknowledgement fails with an actionable error before
# restricted MSVC payload download. Manifest reads stay version-resolution
# cost; deferred failure never proves laziness. The error names the variable,
# the required value, the repository-env mechanism, and the docs link.
FAIL_BEFORE_FETCH = "missing acknowledgement fails before restricted MSVC payload download"
ACTIONABLE_ERROR = "Set BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1 via repository environment after reviewing the applicable Microsoft terms. See docs/native-toolchains.md#windows-acquisition-and-compatibility"
ERROR_NAMES_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
ERROR_MECHANISM = "repository-env"

# Laziness: merely adding the module requires no acceptance and fetches no
# restricted payloads; unrelated workflows fetch nothing and stay green
# without acknowledgement. Extension evaluation already fetches manifests,
# so deferred acceptance failure never proves laziness.
LAZY_CONTRACT = [
    "adding the module requires no acceptance",
    "adding the module fetches no restricted payloads",
    "unrelated workflows fetch no Windows payloads",
    "missing acceptance leaves unrelated workflows green",
    "deferred acceptance failure never proves laziness",
    "extension evaluation already fetches manifests",
]

# Rejected substitutes: automatic acceptance, bypassing upstream controls,
# pre-seeded payloads, patched checks, installed fallback, cross-host route.
REJECTED_ALTERNATIVES = [
    "automatic acceptance",
    "bypassing upstream controls",
    "pre-seeded payloads",
    "patched extension checks",
    "installed Build Tools fallback",
    "cross-host Windows route",
]

# Not in scope: the usage-vs-redistribution rights review itself (issue
# #496), redistribution permission, or the hermetic-llvm SDK EULA variable
# (also issue #496, reviewed not inferred).
NOT_IN_SCOPE = [
    "rights review itself under issue #496",
    "redistribution permission",
    "hermetic-llvm SDK EULA variable under issue #496",
]

# Release linkage: per-host Windows release evidence stays owned under
# issue #807; this UX is the EULA cell of that row, never Supported alone.
RELEASE_EVIDENCE = "windows_x86_64 sbom-provenance delivered under issue #807"

# Live proof labels: seed hello builds without acceptance; the gate plus
# expected plus corpus targets prove the fixture is wired.
WINDOWS_EULA_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_eula:corpus_starlark"
WINDOWS_EULA_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
WINDOWS_EULA_GATE = "//cc/tests/fixtures/windows_eula:eula_gate"
WINDOWS_EULA_EXPECTED = "cc/tests/fixtures/windows_eula/windows_eula.expected"
