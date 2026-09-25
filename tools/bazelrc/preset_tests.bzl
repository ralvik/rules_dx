"""Preset freshness tests. Contract: `docs/contributing/local-workflows.md#preset-update-loop`."""

load("//libs/starlark:defs.bzl", "starlark_test")

def preset_update_tests(name):
    """Declare the vendored-preset freshness test."""
    starlark_test(
        name = name,
        mode = "execution",
        file_checks = {
            "//:.bazelversion": "9.2.0",
            "//:.bazelrc": "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc",
            ":preset.bazelrc": "GENERATED, do not edit.\n# Regenerate: `bazel run //tools/bazelrc:preset_update`.\ncommon --enable_bzlmod\nbuild --verbose_failures\ntest --test_output=errors\n# Owned extra_presets group: coverage.\ncommon --enable_platform_specific_config\ncoverage --test_env=GENERATE_LLVM_LCOV=1\ncoverage --combined_report=lcov\ncoverage --test_tag_filters=-no-coverage\ncoverage --enable_runfiles\ncoverage:linux --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov\ncoverage:macos --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov\ncoverage --instrumentation_filter=^//\n# Owned build profiles (issue #177; See: docs/decisions/0021-build-profiles.md).\nbuild:dx_debug --compilation_mode=dbg\nbuild:dx_dev --compilation_mode=fastbuild\nbuild:dx_release --compilation_mode=opt\nbuild:dx_dev_remote --compilation_mode=fastbuild\nbuild:dx_toolchain --compilation_mode=fastbuild",
            ":src/lib.rs": "PRESET_BAZEL_VERSION",
        },
    )
