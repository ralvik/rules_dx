"""Preset freshness tests.

Pins the version-matched Bazel pin and the reviewed flag inventory
hermetically via file substring checks: a version bump without a reviewed
regen, a stale generated fragment, or a missing `.bazelrc` import fails
here. The checked-in generated fragment is reproduced by
`bazel run //tools/bazelrc:preset.update`; that command with
`--verify-only` prints the flag diff under review.
"""

load("//libs/starlark:defs.bzl", "starlark_test")

def preset_update_tests(name):
    """Declare the vendored-preset freshness test.

    Args:
      name: test target name.
    """
    starlark_test(
        name = name,
        mode = "execution",
        file_checks = {
            "//:.bazelversion": "9.2.0",
            "//:.bazelrc": "import %workspace%/tools/bazelrc/preset.bazelrc\ntry-import %workspace%/user.bazelrc",
            ":preset.bazelrc": "common --enable_bzlmod\nbuild --verbose_failures\ntest --test_output=errors\n# Owned extra_presets group: coverage.\ncoverage --test_env=GENERATE_LLVM_LCOV=1\ncoverage --combined_report=lcov\ncoverage --test_tag_filters=-no-coverage\ncoverage --test_env=COVERAGE_GCOV_PATH=/usr/bin/gcov\ncoverage --instrumentation_filter=^//\n# Owned build profiles (issue #177).\nbuild:dx_debug --compilation_mode=dbg\nbuild:dx_dev --compilation_mode=fastbuild\nbuild:dx_release --compilation_mode=opt",
        },
    )
