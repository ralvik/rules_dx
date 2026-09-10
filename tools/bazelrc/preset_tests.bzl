"""Preset freshness tests (M05 WP4, O62).

Pins the version-matched Bazel pin and the reviewed flag inventory
hermetically: a version bump without a reviewed regen, a stale generated
file, or a missing `.bazelrc` import fails here. The checked-in generated
files are reproduced by `bazel run //tools/bazelrc:preset.update`; that
command with `--verify-only` prints the flag diff under review.
"""

load("//libs/starlark:defs.bzl", "expect_equal", "starlark_test")
load(":bazelrc-preset.bzl", "EXTRA_PRESETS", "PRESET_BAZEL_VERSION", "PRESET_FLAGS")

def preset_update_tests(name):
    """Declare the vendored-preset freshness test.

    Args:
      name: test target name.
    """
    starlark_test(
        name = name,
        mode = "execution",
        checks = [
            expect_equal("preset pin", PRESET_BAZEL_VERSION, "9.2.0"),
            expect_equal("upstream flag count", len(PRESET_FLAGS), 3),
            expect_equal("extra presets groups", sorted(EXTRA_PRESETS.keys()), ["coverage"]),
            expect_equal("coverage flag count", len(EXTRA_PRESETS["coverage"]), 3),
        ],
        file_checks = {
            "//:.bazelversion": "9.2.0",
            "//:.bazelrc": "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc",
            ":bazelrc-preset.bzl": "PRESET_BAZEL_VERSION = \"9.2.0\"\n\"common --enable_bzlmod\",\n\"build --verbose_failures\",\n\"test --test_output=errors\",\n\"coverage\": [\n\"coverage --test_env=GENERATE_LLVM_LCOV=1\",\n\"coverage --combined_report=lcov\",\n\"coverage --instrumentation_filter=^//\",",
            ":preset.bazelrc": "common --enable_bzlmod\nbuild --verbose_failures\ntest --test_output=errors\n# Owned extra_presets group: coverage.\ncoverage --test_env=GENERATE_LLVM_LCOV=1\ncoverage --combined_report=lcov\ncoverage --instrumentation_filter=^//",
        },
    )
