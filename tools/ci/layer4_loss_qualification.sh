#!/usr/bin/env bash
# Layer-4 loss restore-or-wont-fix qualification harness.
#
# Qualifies the three Layer-4 losses with fixture evidence (issue #645):
# - real-daemon exit 3 wont-fix: a live Bazel daemon `bazel test` failure
#   code 3 is not exercised in CI; the hermetic customer path is the
#   `cli/cli/src/exec` unit pin (`bazel_forwards_argv_verbatim_and_exit_code`
#   with `ArgvProbe code Some(3)` plus `Some(3)` in `bazel.rs`);
# - real Buildifier rewrite wont-fix: an in-place rewrite against a live
#   workspace is not exercised in CI; the hermetic customer path is the
#   `//quality/testdata:runner_matrix` golden (`matrix_starlark_format_fail`
#   `x=1` to `x = 1` via the standalone `@dx_tools//:buildifier` artifact);
# - full consumer wiring wont-fix, smoke-only: a full adopt-* consumer
#   matrix is not exercised in CI; the hermetic customer path is the
#   adopt-rust `dx_dev` smoke in normal CI (`local_path_override` plus
#   `dx_dev` wiring) with retained `DxSubjectInfo` pins
#   (`//quality/testdata:real_aspect_presence`) plus `//:preset_parity_test`;
# - fixtures: `cli/cli/tests/fixtures/layer4_loss/` (`pins.bzl` plus
#   `layer4_loss.expected`) pins dispositions plus hermetic customer path
#   plus rejected nested/non-customer harnesses plus honesty;
# - scope: test only, no Bazel semantics change. Seed only: platform plus
#   consumer plus release evidence stays owned gap; no Supported claim.
#
# Versioned here, run by CI via `bazel run //tools/ci:layer4_loss_qualification`,
# following //tools/ci:cli_execution_gaps_qualification.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap: Bazel runfiles forest first (`data = ["//tools/sh:lib"]`), then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/lib.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/lib.sh" 2>/dev/null || source "$(dirname "${BASH_SOURCE[0]}")/../sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="cli/cli/tests/fixtures/layer4_loss/pins.bzl"
expected="cli/cli/tests/fixtures/layer4_loss/layer4_loss.expected"
fixture_build="cli/cli/tests/fixtures/layer4_loss/BUILD.bazel"
exec_bazel="cli/cli/src/exec/bazel.rs"
exec_support="cli/cli/src/exec/test_support.rs"
matrix="quality/testdata/runner_matrix_cases.bzl"
aspect="quality/testdata/real_aspect_subject.bzl"
testdata_build="quality/testdata/BUILD.bazel"
preset_build="BUILD.bazel"
module="MODULE.bazel"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"
testing="docs/testing/cli.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$expected" && -f "$fixture_build" ]]; then
  ok
else
  bad "layer4_loss fixture missing (want $pins plus layer4_loss.expected plus $fixture_build)"
fi

# Pins record the three per-loss wont-fix dispositions plus smoke-only.
if grep -q -F -e 'LOSS_DAEMON_EXIT3 = "wont-fix"' "$pins" &&
  grep -q -F -e 'LOSS_BUILDIFIER_REWRITE = "wont-fix"' "$pins" &&
  grep -q -F -e 'LOSS_FULL_WIRING = "wont-fix"' "$pins" &&
  grep -q -F -e 'WIRING_SMOKE_ONLY = "smoke-only"' "$pins"; then
  ok
else
  bad "pins.bzl lost its three per-loss wont-fix plus smoke-only dispositions under #645"
fi

# Pins record the hermetic customer path plus rejected harnesses plus honesty.
if grep -q -F -e 'hermetic pins stay the customer path' "$pins" &&
  grep -q -F -e 'nested-Bazel CI harness rejected' "$pins" &&
  grep -q -F -e 'non-customer harness in CI rejected' "$pins" &&
  grep -q -F -e 'no second-Bazel download' "$pins" &&
  grep -q -F -e 'qualified seed-only under issue #645' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its customer-path plus rejected plus honesty wiring under #645"
fi

# Pins record the daemon hermetic pin (exec code 3 plus unit test).
if grep -q -F -e 'cli/cli/src/exec/bazel.rs code 3' "$pins" &&
  grep -q -F -e 'bazel_forwards_argv_verbatim_and_exit_code' "$pins" &&
  grep -q -F -e 'ArgvProbe code Some(3)' "$pins"; then
  ok
else
  bad "pins.bzl lost its daemon hermetic exec code 3 pin under #645"
fi

# Pins record the Buildifier hermetic pin (runner matrix plus dirty rewrite).
if grep -q -F -e '//quality/testdata:runner_matrix' "$pins" &&
  grep -q -F -e 'matrix_starlark_format_fail' "$pins" &&
  grep -q -F -e 'matrix/starlark_dirty.bzl' "$pins" &&
  grep -q -F -e 'x=1 to x = 1' "$pins" &&
  grep -q -F -e '@dx_tools//:buildifier' "$pins"; then
  ok
else
  bad "pins.bzl lost its Buildifier hermetic runner-matrix rewrite pin under #645"
fi

# Pins record the wiring hermetic pins (smoke plus aspect plus preset).
if grep -q -F -e 'bazel build //examples/adopt-rust/... --config=dx_dev' "$pins" &&
  grep -q -F -e '//quality/testdata:real_aspect_presence' "$pins" &&
  grep -q -F -e '//:preset_parity_test' "$pins"; then
  ok
else
  bad "pins.bzl lost its wiring hermetic smoke plus aspect plus preset pins under #645"
fi

# Expected fixture pins the three losses plus customer path plus rejected plus honesty.
if grep -q -F -e 'real-daemon exit 3 wont-fix' "$expected" &&
  grep -q -F -e 'real Buildifier rewrite wont-fix' "$expected" &&
  grep -q -F -e 'full consumer wiring wont-fix smoke-only' "$expected" &&
  grep -q -F -e 'hermetic pins stay the customer path' "$expected" &&
  grep -q -F -e 'nested-Bazel CI harness rejected' "$expected" &&
  grep -q -F -e 'non-customer harness in CI rejected' "$expected" &&
  grep -q -F -e 'qualified seed-only under issue #645' "$expected" &&
  grep -q -F -e 'no Supported claim' "$expected"; then
  ok
else
  bad "layer4_loss.expected lost its three losses plus customer-path plus rejected plus honesty lines under #645"
fi

# Fixture BUILD exports pins plus expected with corpus coverage.
if grep -q -F -e 'pins.bzl' "$fixture_build" &&
  grep -q -F -e 'layer4_loss.expected' "$fixture_build" &&
  grep -q -F -e 'corpus_starlark' "$fixture_build"; then
  ok
else
  bad "layer4_loss BUILD.bazel lost its pins plus expected exports with corpus under #645"
fi

# Exec keeps the hermetic daemon exit-code pin (code 3 plus unit test).
if grep -q -F -e 'Some(3)' "$exec_bazel" &&
  grep -q -F -e 'bazel_forwards_argv_verbatim_and_exit_code' "$exec_bazel" &&
  [[ -f "$exec_support" ]]; then
  ok
else
  bad "exec/bazel.rs lost its hermetic daemon exit-code pin (Some(3) plus unit test, issue #645)"
fi

# Runner matrix keeps the hermetic Buildifier rewrite golden.
if grep -q -F -e 'matrix_starlark_format_fail' "$matrix" &&
  grep -q -F -e 'matrix/starlark_dirty.bzl' "$matrix" &&
  grep -q -F -e 'x = 1' "$matrix" &&
  grep -q -F -e '@dx_tools//:buildifier' "$matrix"; then
  ok
else
  bad "runner_matrix_cases.bzl lost its hermetic Buildifier x=1 to x = 1 golden under #645"
fi

# Wiring hermetic pins stay live (aspect plus preset plus adopt-rust smoke).
if grep -q -F -e 'DxSubjectInfo' "$aspect" &&
  grep -q -F -e 'real_aspect_presence' "$testdata_build" &&
  grep -q -F -e 'preset_parity_test' "$preset_build" &&
  grep -q -F -e 'bazel build --noshow_progress //examples/adopt-rust/... --config=dx_dev' "$ci"; then
  ok
else
  bad "wiring hermetic pins lost (DxSubjectInfo plus real_aspect_presence plus preset_parity_test plus adopt-rust smoke, issue #645)"
fi

# No nested-Bazel CI harness and no second Bazel (rejected alternative stays rejected).
if [[ ! -d "integration" ]] &&
  [[ ! -f "tools/ci/e2e.sh" ]] &&
  ! grep -q -F -e '//tools/ci:e2e' "$ci" &&
  ! grep -q -F -e 'bazel_binaries.download' "$module" &&
  ! grep -q -F -e 'rules_bazel_integration_test' "$module"; then
  ok
else
  bad "nested-Bazel harness reappeared (integration/ plus e2e drivers plus second Bazel stay rejected under #645)"
fi

# Verification matrix owns the per-loss wont-fix record with fixtures plus qualification.
if grep -q -F -e 'qualified seed-only under #645' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:layer4_loss_qualification' "$verify" &&
  grep -q -F -e 'cli/cli/tests/fixtures/layer4_loss/' "$verify" &&
  grep -q -F -e 'real-daemon exit 3' "$verify" &&
  grep -q -F -e 'real Buildifier rewrite' "$verify" &&
  grep -q -F -e 'smoke-only' "$verify"; then
  ok
else
  bad "verification-matrix.md lost its #645 per-loss wont-fix record with fixtures plus qualification"
fi

# Testing CLI owns the per-loss record with fixtures plus qualification.
if grep -q -F -e 'issue #645' "$testing" &&
  grep -q -F -e 'cli/cli/tests/fixtures/layer4_loss/' "$testing" &&
  grep -q -F -e 'bazel run //tools/ci:layer4_loss_qualification' "$testing" &&
  grep -q -F -e 'nested-Bazel' "$testing"; then
  ok
else
  bad "testing/cli.md lost its #645 per-loss record with fixtures plus qualification"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "layer4_loss_qualification"' "$build" &&
  grep -q -F -e 'layer4_loss_qualification.sh' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:layer4_loss_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the layer4_loss_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix Green plus dogfood lists the harness count.
if grep -q -F -e ':layer4_loss_qualification' "$verify" &&
  grep -q -F -e '`layer4_loss_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix Green lost layer4_loss_qualification 16/16"
fi

dx_test_summary "layer-4 loss qualification harness"
