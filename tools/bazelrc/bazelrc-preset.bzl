"""Vendored Bazel execution preset (M05 WP4, O62) -- GENERATED, do not edit.

Version-matched to Bazel 9.2.0 (`.bazelversion`). Upstream-derived flags, owned `extra_presets` groups, and owned `BUILD_PROFILES`, each reviewed in `tools/bazelrc/preset.py`.
Regenerate:

    bazel run //tools/bazelrc:preset.update
"""

PRESET_BAZEL_VERSION = "9.2.0"

PRESET_FLAGS = [
    "common --enable_bzlmod",
    "build --verbose_failures",
    "test --test_output=errors",
]

EXTRA_PRESETS = {
    "coverage": [
        "coverage --test_env=GENERATE_LLVM_LCOV=1",
        "coverage --combined_report=lcov",
        "coverage --test_tag_filters=-no-coverage",
        "coverage --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov",
        "coverage --instrumentation_filter=^//",
    ],
}

BUILD_PROFILES = [
    "build:dx_debug --compilation_mode=dbg",
    "build:dx_dev --compilation_mode=fastbuild",
    "build:dx_release --compilation_mode=opt",
]
