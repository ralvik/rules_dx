"""Shared toolchain pins plus registration order. Contract: docs/decisions/0014-tested-platform-release-stack.md."""

# Pinned shared toolchains (see MODULE.bazel; per-split pin_consistency in tools/ci/pin_consistency.sh).
RULES_CC_VERSION = "0.2.22"
GOOGLETEST_VERSION = "1.18.0"
RULES_GO_VERSION = "0.63.0"
GAZELLE_VERSION = "0.52.2"
RULES_SHELL_VERSION = "0.6.1"
PLATFORMS_VERSION = "1.1.0"
BAZEL_SKYLIB_VERSION = "1.9.0"
RULES_PROTO_VERSION = "7.1.0"
GO_SDK_VERSION = "1.26.6"
GO_LANGUAGE_FLOOR = "1.24.12"

# Pinned Go SDK archives (see MODULE.bazel go_sdk.download sdks; per-split
# pin_consistency in tools/ci/pin_consistency.sh). Filenames plus sha256 from
# https://go.dev/dl/?mode=json for 1.26.6 (verified 2026-09-22); the sha is the
# trust anchor, the single dl.google.com URL is availability only. Pinning
# sdks avoids the go.dev index fetch and lets lockfile facts cover airgapped
# builds with a warm download cache.
GO_SDK_SDKS = {
    "darwin_amd64": ["go1.26.6.darwin-amd64.tar.gz", "08b65a63f244115121ced6c3b55ad38d801a7442acad5c949a17aad84ae6d684"],
    "darwin_arm64": ["go1.26.6.darwin-arm64.tar.gz", "2dc95ce4675829f2df0e86b28bcef3283635902062a5f0580ca659bf570f3204"],
    "linux_amd64": ["go1.26.6.linux-amd64.tar.gz", "708effb774be8237570d0add163225abbdfaf4fca28b2611df167beba4feef89"],
    "linux_arm64": ["go1.26.6.linux-arm64.tar.gz", "d0507e9e9d7fe012aae570108cbd76c15de879e17130ab8cb90d4d7445cb1f2e"],
    "windows_amd64": ["go1.26.6.windows-amd64.zip", "5b6c5b556525810463b5c897b50dc7a82d6a3dc0bfaf55d990a7e9f31d6b2318"],
    "windows_arm64": ["go1.26.6.windows-arm64.zip", "06dbe785743d534ef8a469dad88adf7f1b2b438507ccfef9b98e7cf8c97b4b68"],
}

# Standalone quality-tool hosts (see //quality/artifacts:extension.bzl).
DX_TOOL_PLATFORMS = [
    "linux_x86_64",
    "linux_arm64",
    "macos_arm64",
    "macos_x86_64",
    "windows_x86_64",
]
