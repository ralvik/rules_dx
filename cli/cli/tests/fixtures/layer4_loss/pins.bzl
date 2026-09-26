"""Layer-4 loss restore-or-wont-fix fixture.

Contract: `docs/product/support-matrix.md`,
`docs/testing/cli.md#end-to-end-tests`.
Fixture: `cli/cli/tests/fixtures/layer4_loss/` via
`bazel run //tools/ci:layer4_loss_qualification`.
"""

# Per-loss dispositions (hermetic pins stay the customer path; no
# nested-Bazel CI harness; non-customer harness in CI rejected).
LOSS_DAEMON_EXIT3 = "wont-fix"
LOSS_BUILDIFIER_REWRITE = "wont-fix"
LOSS_FULL_WIRING = "wont-fix"

# Real-daemon exit 3 is wont-fix: a live Bazel daemon `bazel test`
# failure code 3 is not exercised in CI. The hermetic customer path is
# the `cli/cli/src/exec` unit pin (`bazel_forwards_argv_verbatim_and_exit_code`
# with `ArgvProbe code Some(3)` plus `Some(3)` in `cli/cli/src/exec/bazel.rs`).
DAEMON_HERMETIC_EXEC = "cli/cli/src/exec/bazel.rs code 3"
DAEMON_HERMETIC_TEST = "bazel_forwards_argv_verbatim_and_exit_code"
DAEMON_HERMETIC_PROBE = "ArgvProbe code Some(3)"

# Real Buildifier rewrite is wont-fix: an in-place rewrite against a live
# workspace is not exercised in CI. The hermetic customer path is the
# `//quality/testdata:runner_matrix` golden (`matrix_starlark_format_fail`
# `x=1` to `x = 1` via the standalone `@dx_tools//:buildifier` artifact).
BUILDIFIER_HERMETIC_MATRIX = "//quality/testdata:runner_matrix"
BUILDIFIER_HERMETIC_CASE = "matrix_starlark_format_fail"
BUILDIFIER_HERMETIC_DIRTY = "matrix/starlark_dirty.bzl"
BUILDIFIER_HERMETIC_REWRITE = "x=1 to x = 1"
BUILDIFIER_HERMETIC_TOOL = "@dx_tools//:buildifier"

# Full consumer wiring is wont-fix, smoke-only: a full adopt-* consumer
WIRING_HERMETIC_SMOKE = "bazel build //examples/adopt-rust/... --config=dx_dev"
WIRING_HERMETIC_ASPECT = "//quality/testdata:real_aspect_presence"
WIRING_HERMETIC_PRESET = "//:preset_parity_test"
WIRING_SMOKE_ONLY = "smoke-only"

# Customer path plus rejected routes (never pinned as supported here).
HERMETIC_CUSTOMER_PATH = "hermetic pins stay the customer path"
REJECTED_NESTED_HARNESS = "nested-Bazel CI harness rejected"
REJECTED_NON_CUSTOMER_HARNESS = "non-customer harness in CI rejected"
REJECTED_SECOND_BAZEL = "no second-Bazel download"

# Honesty lines (never pinned as supported here).
NO_SUPPORTED = "no Supported claim"
SEED_ONLY = "qualified seed-only under issue #645"
