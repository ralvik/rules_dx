APPLE_SDK_IDENTITY = "MacOSX26.5"
APPLE_SDK_SOURCE = "hermetic-llvm v0.8.19 pinned extraction"
APPLE_SDK_HERMETIC_LLVM_VERSION = "v0.8.19"
APPLE_SDK_HERMETIC_LLVM_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"

APPLE_SDK_AGREEMENT = "https://www.apple.com/legal/sla/docs/xcode.pdf"

APPLE_HOSTED_EXECUTION_NOTE = "Apple-hosted execution does not by itself authorize separate extraction or unrestricted caching"
APPLE_EXTRACTION_RESTRICTION = "separate extraction needs terms review"
APPLE_CACHING_RESTRICTION = "unrestricted caching needs terms review"

TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"

WINDOWS_SUPPORT_VERSION = "v0.4.1"
WINDOWS_SUPPORT_COMMIT = "43dce21da77d8cb4a486709e34f1d887e69058c6"

WINDOWS_SUPPORT_MSVC = "14.50.35717"
WINDOWS_SUPPORT_REDIST = "14.50.35710"
WINDOWS_SUPPORT_SDK_PACKAGE = "10.0.26100.7705"

EULA_ENV_VAR = "BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
EULA_REQUIRED_VALUE = "1"
EULA_MECHANISM = "repository-env"
EULA_README_MISMATCH = True

SDK_EULA_NOTE = "windows_support does not enforce the SDK EULA variable documented by hermetic-llvm"
SDK_EULA_INFERENCE_REJECTED = "applicable terms and acknowledgement must be reviewed rather than inferred from setting a variable"

USAGE_VS_REDISTRIBUTION = "usage vs redistribution reviewed separately"
ACCEPTANCE_NOT_REDISTRIBUTION = "acceptance is not permission to redistribute"
DOWNLOAD_USE_REDISTRIBUTE_SEPARATE = "Direct SDK downloads, permission to use, and permission to redistribute are separate checks"

CACHE_RIGHTS = [
    "mirrors need terms review",
    "redistribution needs terms review",
    "internal caches need terms review",
    "remote workers need terms review",
]
OFFICIAL_DOWNLOAD_NOT_PERMISSION = "Official download availability is not permission"
LICENSE_BOUNDARY_NOTE = "Record license-approved cache/mirror/remote-worker boundaries separately from technical download success"

REJECTED_ALTERNATIVES = [
    "assume rights",
    "automatic acceptance",
    "inferred acknowledgement from setting a variable",
    "unrestricted caching",
    "unrestricted mirroring",
    "redistribution without approval",
]

ACQUISITION_RIGHTS_FIXTURE_CORPUS = "//cc/tests/fixtures/acquisition_rights:corpus_starlark"
ACQUISITION_RIGHTS_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
