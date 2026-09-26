TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"

TOOLCHAINS_MSVC_SOURCES = [
    "private/vs_channel_manifest.bzl",
    "extensions.bzl",
    "private/msvc_toolchains_repo.bzl",
    "overlays/toolchain/clang-cl/BUILD.toolchain.tpl",
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

IMMUTABLE_INPUTS = ["fixed-manifest", "package-index"]
MUTABLE_REJECTED = [
    "live VS channel manifests",
    "minor-version selectors",
    "individual payload checksums only",
    "pinned rules commit only",
]

LAZY_CONTRACT = [
    "adding the module requires no acceptance",
    "adding the module fetches no restricted payloads",
    "unrelated workflows fetch no Windows payloads",
    "missing acceptance leaves unrelated workflows green",
]

REJECTED_ALTERNATIVES = [
    "mutable fetch",
    "project-owned downloader",
    "cross-host Windows route",
    "installed Build Tools fallback",
]

WINDOWS_ACQUISITION_FIXTURE_CORPUS = "//cc/tests/fixtures/windows_acquisition:corpus_starlark"
WINDOWS_ACQUISITION_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
