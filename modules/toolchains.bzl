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

# Standalone quality-tool hosts (see //quality/artifacts:extension.bzl).
DX_TOOL_PLATFORMS = [
    "linux_x86_64",
    "linux_arm64",
    "macos_arm64",
    "macos_x86_64",
    "windows_x86_64",
]
