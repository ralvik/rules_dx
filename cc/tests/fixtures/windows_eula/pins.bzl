"""Windows EULA acknowledgement UX pins."""

TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"
TOOLCHAINS_MSVC_EXTENSION = "extensions.bzl"
TOOLCHAINS_MSVC_SOURCES = [
    "private/vs_channel_manifest.bzl",
    "extensions.bzl",
    "private/msvc_toolchains_repo.bzl",
]

WINDOWS_SUPPORT_VERSION = "v0.4.1"
WINDOWS_SUPPORT_COMMIT = "43dce21da77d8cb4a486709e34f1d887e69058c6"
WINDOWS_SUPPORT_MSVC = "14.50.35717"
WINDOWS_SUPPORT_REDIST = "14.50.35710"
WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"

EULA_ENV_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
EULA_REQUIRED_VALUE = "1"
EULA_MECHANISM = "repository-env"
EULA_README_MISMATCH = True

ACK_HOW = "export BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1 plus --repo_env=BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1"
ACK_NEVER_AUTOMATIC = "never automatic via .bazelrc or wrapper defaults"
ACK_NEVER_BYPASS = "never bypassing upstream controls"

FAIL_BEFORE_FETCH = "missing acknowledgement fails before restricted MSVC payload download"
ACTIONABLE_ERROR = "Set BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA=1 via repository environment after reviewing the applicable Microsoft terms. See docs/native-toolchains.md#windows-acquisition-and-compatibility"
ERROR_NAMES_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
ERROR_MECHANISM = "repository-env"

LAZY_CONTRACT = [
    "adding the module requires no acceptance",
    "adding the module fetches no restricted payloads",
    "unrelated workflows fetch no Windows payloads",
    "missing acceptance leaves unrelated workflows green",
    "deferred acceptance failure never proves laziness",
    "extension evaluation already fetches manifests",
]

REJECTED_ALTERNATIVES = [
    "automatic acceptance",
    "bypassing upstream controls",
    "pre-seeded payloads",
    "patched extension checks",
    "installed Build Tools fallback",
    "cross-host Windows route",
]

NOT_IN_SCOPE = [
    "rights review itself under issue #496",
    "redistribution permission",
    "hermetic-llvm SDK EULA variable under issue #496",
]

RELEASE_EVIDENCE = "windows_x86_64 sbom-provenance delivered under issue #807"

WINDOWS_EULA_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_eula:corpus_starlark"
WINDOWS_EULA_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
WINDOWS_EULA_GATE = "//cc/tests/fixtures/windows_eula:eula_gate"
WINDOWS_EULA_EXPECTED = "cc/tests/fixtures/windows_eula/windows_eula.expected"
