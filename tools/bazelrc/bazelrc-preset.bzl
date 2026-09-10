"""Vendored Bazel execution preset (M05 WP4, O62) -- GENERATED, do not edit.

Version-matched to Bazel 9.2.1 (`.bazelversion`). Upstream-derived flags and owned
`extra_presets` groups, each reviewed in `tools/bazelrc/preset.py`. Regenerate:

    bazel run //tools/bazelrc:preset.update
"""

PRESET_BAZEL_VERSION = "9.2.1"

PRESET_FLAGS = [
    "common --enable_bzlmod",
    "build --verbose_failures",
    "test --test_output=errors",
]

EXTRA_PRESETS = {
    "coverage": [
        "coverage --test_env=GENERATE_LLVM_LCOV=1",
        "coverage --combined_report=lcov",
        "coverage --instrumentation_filter=^//",
    ],
}
