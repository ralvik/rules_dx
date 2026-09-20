"""Non-dogfed e2e plus negative pins (issue #508). Contract: `docs/testing/verification-matrix.md#layers`, `docs/testing/README.md#end-to-end-tests`. Fixture: `tools/ci/tests/fixtures/non_dogfed/` via `bazel run //tools/ci:non_dogfed_qualification`.
"""
# E2E drivers deleted (issue #407): no nested workspace, no second Bazel, no manual drivers. Term e2e throughout, never integration for drivers.
E2E_NO_WORKSPACE = "no integration/ workspace"
E2E_NO_DRIVERS = ["tools/ci/e2e.sh", "tools/ci/e2e_format.sh", "tools/ci/e2e_preset.sh", "tools/ci/e2e_cases.sh"]
E2E_NO_SUITE = "//tools/ci:e2e"
E2E_NO_DOWNLOAD = "no second-Bazel download"
E2E_TERM = "e2e"
E2E_REJECTED_TERM = "integration drivers"
# Hermetic e2e equivalents run under bazel test //... (issue #407 pins).
E2E_HERMETIC_EXEC = "cli/cli/src/exec/bazel.rs code 3"
E2E_HERMETIC_MATRIX = "//quality/testdata:runner_matrix"
E2E_HERMETIC_ASPECT = "//quality/testdata:real_aspect_presence"
E2E_HERMETIC_PRESET = "//:preset_parity_test"
E2E_HERMETIC_SMOKE = "bazel build //examples/adopt-rust/... --config=dx_dev"
E2E_LOSS = ["real-daemon exit 3", "real Buildifier rewrite", "smoke-only"]
# Negative fixtures are green hermetic proofs (issue #406): no manual, no nested Bazel.
NEGATIVE_DEMOS = ["//libs/starlark/tests/negative:failing_check_demo", "//libs/starlark/tests/negative:missing_observation_demo", "//libs/starlark/tests/negative:missing_fragment_demo", "//libs/starlark/tests/negative:wrong_phase_demo"]
NEGATIVE_MANUAL_SUBJECT = "//quality/testdata:fixture_real_markdown_no_config_subject"
NEGATIVE_PROOF_KINDS = ["failure_test", "failing_check_test.sh", "missing_observation_test.sh", "missing_fragment_test.sh", "fixture_real_markdown_no_config_test"]
NEGATIVE_CI = "bazel test //..."
NEGATIVE_REJECTED = "manual_negatives shell loop"
# No-coverage cohort runs under test, skips only under coverage.
NO_COVERAGE_FILTER = "coverage --test_tag_filters=-no-coverage"
NO_COVERAGE_MIN_COUNT = 10
NO_COVERAGE_FAIL_FIXTURE = "matrix_python_lint_fail"
NO_COVERAGE_SKIP_PROOF = "tools/ci/target_tags.sh"
NO_COVERAGE_GATES = ["bazel test //...", "//tools/ci:target_tags", "//tools/ci:coverage_cell", "//tools/ci:coverage_qualification"]
NO_COVERAGE_CELLS = 7
# Shell sources carry no quality class by design; ownership plus execution explicit.
SHELL_KNOWN_CLASS = '"shell",'
SHELL_NO_OWNER = "no shell_srcs"
SHELL_CORPUS_SCOPE = "BUILD/MODULE/bzl/toml/md only"
SHELL_CODE_SCOPE = "code languages only"
SHELL_OWNERSHIP = "every .sh in deps(//...)"
SHELL_EXECUTION = "bazel test //..."
SHELL_PORTABILITY = "//tools/ci:shell_contract"
# Rejected alternative: silent under dogfood gates.
REJECTED_ALTERNATIVES = ["silent under dogfood gates", "manual e2e drivers", "integration term for drivers", "manual negatives loop", "seed-only forever"]
NO_SUPPORTED_CLAIM = "no Supported claim"
OWNED_GAPS = "platform plus consumer plus release evidence stays owned gap"
