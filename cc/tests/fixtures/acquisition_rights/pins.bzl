"""Apple plus Microsoft acquisition rights pins (issue #496).

Contract: `docs/native-toolchains.md#windows-acquisition-and-compatibility`,
`docs/native-toolchains.md#qualification-questions-and-delivery`,
`docs/decisions/0014-tested-platform-release-stack.md#decision`.
Fixture: `cc/tests/fixtures/acquisition_rights/` via
`bazel run //tools/ci:acquisition_rights_qualification`.

Decides the rights slice of the native baseline: the hermetic-llvm
Apple-SDK backend plus the toolchains_msvc backend stay provisional, but
their acquisition-plus-cache rights are reviewed with fixture evidence
recorded here and in the owning docs. Assume rights stays rejected per
the issue alternatives. Usage vs redistribution stay distinct; acceptance
is never permission to redistribute. Mirrors, redistribution, internal
caches, and remote workers need license-approved boundaries recorded
separately from technical download success. Official download
availability is not permission.

Acquisition mechanics stay qualified under issue #495, transport plus
ABI under issue #497, interop under issue #498, corpus plus floors plus
coverage under issues #499/#500/#501, never double-claimed here.
"""

# Apple starting point: hermetic-llvm pinned MacOSX SDK extraction.
APPLE_SDK_IDENTITY = "MacOSX26.5"
APPLE_SDK_SOURCE = "hermetic-llvm v0.8.19 pinned extraction"
APPLE_SDK_HERMETIC_LLVM_VERSION = "v0.8.19"
APPLE_SDK_HERMETIC_LLVM_COMMIT = "6314688712edf3a95f78642d80393868256b4ef2"

# Apple terms reference: review the terms accompanying that exact package
# against the Apple SDK agreement.
APPLE_SDK_AGREEMENT = "https://www.apple.com/legal/sla/docs/xcode.pdf"

# Apple-hosted execution does not by itself authorize separate extraction
# or unrestricted caching.
APPLE_HOSTED_EXECUTION_NOTE = "Apple-hosted execution does not by itself authorize separate extraction or unrestricted caching"
APPLE_EXTRACTION_RESTRICTION = "separate extraction needs terms review"
APPLE_CACHING_RESTRICTION = "unrestricted caching needs terms review"

# Microsoft prototype identity for the rights scope (acquisition mechanics
# qualified under issue #495; rights decided here).
TOOLCHAINS_MSVC_COMMIT = "8e2aa4624bbb5a53a94f135e90995f307875d1ad"
TOOLCHAINS_MSVC_MODULE = "0.0.0"

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

# windows_support SDK implementation does not enforce the SDK EULA
# variable documented by hermetic-llvm; applicable terms and
# acknowledgement must be reviewed rather than inferred from setting a
# variable.
SDK_EULA_NOTE = "windows_support does not enforce the SDK EULA variable documented by hermetic-llvm"
SDK_EULA_INFERENCE_REJECTED = "applicable terms and acknowledgement must be reviewed rather than inferred from setting a variable"

# Usage vs redistribution stay distinct: acceptance is not permission to
# redistribute. Direct SDK downloads, permission to use, and permission to
# redistribute are separate checks.
USAGE_VS_REDISTRIBUTION = "usage vs redistribution reviewed separately"
ACCEPTANCE_NOT_REDISTRIBUTION = "acceptance is not permission to redistribute"
DOWNLOAD_USE_REDISTRIBUTE_SEPARATE = "Direct SDK downloads, permission to use, and permission to redistribute are separate checks"

# Cache, mirror, and remote-worker boundaries: review actual package
# terms, deliberate acceptance, extraction, mirrors, redistribution,
# internal caches and remote workers. Official download availability is
# not permission. License-approved boundaries stay recorded separately
# from technical download success.
CACHE_RIGHTS = [
    "mirrors need terms review",
    "redistribution needs terms review",
    "internal caches need terms review",
    "remote workers need terms review",
]
OFFICIAL_DOWNLOAD_NOT_PERMISSION = "Official download availability is not permission"
LICENSE_BOUNDARY_NOTE = "Record license-approved cache/mirror/remote-worker boundaries separately from technical download success"

# Rejected substitutes: assume rights per the issue alternatives, plus
# automatic acceptance, inferred acknowledgement, unrestricted
# caching/mirroring, and redistribution without approval.
REJECTED_ALTERNATIVES = [
    "assume rights",
    "automatic acceptance",
    "inferred acknowledgement from setting a variable",
    "unrestricted caching",
    "unrestricted mirroring",
    "redistribution without approval",
]

# Live proof labels: seed hello builds without acceptance; the hermetic
# script plus corpus target prove the fixture is wired.
ACQUISITION_RIGHTS_FIXTURE_CORPUS = "//cc/tests/fixtures/acquisition_rights:corpus_starlark"
ACQUISITION_RIGHTS_LIVE_HELLO = "//cc/tests/fixtures/hello:hello"
