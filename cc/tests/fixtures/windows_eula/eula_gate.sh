#!/usr/bin/env bash
# Windows EULA acknowledgement gate fixture.
# Fails before restricted acquisition when acknowledgement is missing.
set -euo pipefail
var="BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA"
val="${BAZEL_TOOLCHAINS_MSVC_ACCEPT_MICROSOFT_VISUAL_STUDIO_BUILDTOOLS_EULA:-}"
if [[ "$val" != "1" ]]; then
  echo "error: missing Microsoft EULA acknowledgement before Windows MSVC acquisition" >&2
  echo "Set $var=1 via repository environment (export $var=1 plus bazel build --repo_env=$var=1) after reviewing the applicable Microsoft terms. See docs/native-toolchains.md#windows-acquisition-and-compatibility. Automatic acceptance and bypassing upstream controls are rejected. Unrelated workflows require no acknowledgement." >&2
  exit 1
fi
echo "EULA acknowledgement present ($var=1); restricted Windows MSVC acquisition may proceed."
