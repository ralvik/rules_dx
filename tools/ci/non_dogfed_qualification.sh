#!/usr/bin/env bash
# Non-dogfed e2e plus negatives qualification harness.
#
# Qualifies the non-dogfed execution slice with fixture evidence pinned in
# `tools/ci/tests/fixtures/non_dogfed/pins.bzl` (plus `non_dogfed.expected`),
# without claiming qualified backends, floors, coverage, or Supported:
# - e2e drivers deleted with hermetic equivalents under
#   `bazel test //...` (term e2e throughout, never integration for drivers),
# - negatives as green hermetic proofs (no manual loop),
# - no-coverage cohort via coverage-excluded runs with ownership gates,
# - shell sources with no quality class by design via ownership plus execution.
# Silent coverage under the standard dogfood gates stays rejected.
#
# Versioned here, run by CI via `bazel run //tools/ci:non_dogfed_qualification`,
# following //tools/ci:non_dogfed_paths.
set -euo pipefail

# Shared workspace + runfiles helpers.
# Bootstrap via tools/sh/bootstrap.sh (issue #654): runfiles forest first, then source tree.
source "${RUNFILES_DIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${TEST_SRCDIR:-/dev/null}/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$0.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "${BASH_SOURCE[0]}.runfiles/_main/tools/sh/bootstrap.sh" 2>/dev/null || source "$(git rev-parse --show-toplevel 2>/dev/null)/tools/sh/bootstrap.sh"
dx_bootstrap "tools/sh/lib.sh"

dx_cd_workspace

dx_test_init

pins="tools/ci/tests/fixtures/non_dogfed/pins.bzl"
pins_build="tools/ci/tests/fixtures/non_dogfed/BUILD.bazel"
expected="tools/ci/tests/fixtures/non_dogfed/non_dogfed.expected"
plan="tools/ci/non_dogfed_paths.sh"
build="tools/ci/BUILD.bazel"
ci=".github/workflows/ci.yml"
verify="docs/testing/verification-matrix.md"
testing_readme="docs/testing/README.md"

# Fixture files stay present.
if [[ -f "$pins" && -f "$pins_build" && -f "$expected" ]]; then
  ok
else
  bad "non-dogfed fixture missing (want $pins plus $pins_build plus non_dogfed.expected)"
fi

# Pins record the deleted e2e drivers plus the e2e term.
if grep -q -F -e 'E2E_NO_WORKSPACE = "no integration/ workspace"' "$pins" &&
  grep -q -F -e 'tools/ci/e2e.sh' "$pins" &&
  grep -q -F -e 'E2E_NO_SUITE = "//tools/ci:e2e"' "$pins" &&
  grep -q -F -e 'E2E_NO_DOWNLOAD = "no second-Bazel download"' "$pins" &&
  grep -q -F -e 'E2E_TERM = "e2e"' "$pins" &&
  grep -q -F -e 'never integration for drivers' "$pins"; then
  ok
else
  bad "pins.bzl lost its deleted e2e drivers plus e2e term under issue #508"
fi

# Pins record the hermetic e2e equivalents plus the recorded loss.
if grep -q -F -e 'E2E_HERMETIC_EXEC' "$pins" &&
  grep -q -F -e '//quality/testdata:runner_matrix' "$pins" &&
  grep -q -F -e '//quality/testdata:real_aspect_presence' "$pins" &&
  grep -q -F -e '//:preset_parity_test' "$pins" &&
  grep -q -F -e 'bazel build //examples/adopt-rust/... --config=dx_dev' "$pins" &&
  grep -q -F -e 'real-daemon exit 3' "$pins" &&
  grep -q -F -e 'smoke-only' "$pins"; then
  ok
else
  bad "pins.bzl lost its hermetic e2e equivalents plus loss record under issue #508"
fi

# Pins record the green negative proofs plus the rejected manual loop.
if grep -q -F -e '//libs/starlark/tests/negative:failing_check_demo' "$pins" &&
  grep -q -F -e '//libs/starlark/tests/negative:missing_observation_demo' "$pins" &&
  grep -q -F -e 'fixture_real_markdown_no_config_subject' "$pins" &&
  grep -q -F -e 'failure_test' "$pins" &&
  grep -q -F -e 'bazel test //...' "$pins" &&
  grep -q -F -e 'manual_negatives shell loop' "$pins"; then
  ok
else
  bad "pins.bzl lost its green negative proofs plus manual-loop rejection under issue #508"
fi

# Pins record the no-coverage cohort with gates plus the seven cells.
if grep -q -F -e 'coverage --test_tag_filters=-no-coverage' "$pins" &&
  grep -q -F -e 'matrix_python_lint_fail' "$pins" &&
  grep -q -F -e 'tools/ci/target_tags.sh' "$pins" &&
  grep -q -F -e '//tools/ci:coverage_cell' "$pins" &&
  grep -q -F -e 'NO_COVERAGE_CELLS = 7' "$pins"; then
  ok
else
  bad "pins.bzl lost its no-coverage cohort plus gates plus seven cells under issue #508"
fi

# Pins record shell with no quality class plus the rejected silent gap.
if grep -q -F -e 'SHELL_KNOWN_CLASS' "$pins" &&
  grep -q -F -e 'no shell_srcs' "$pins" &&
  grep -q -F -e 'every .sh in deps(//...)' "$pins" &&
  grep -q -F -e '//tools/ci:shell_contract' "$pins" &&
  grep -q -F -e 'silent under dogfood gates' "$pins" &&
  grep -q -F -e 'no Supported claim' "$pins"; then
  ok
else
  bad "pins.bzl lost its shell no-class plus ownership plus silent rejection under issue #508"
fi

# Expected plan covers the four cohorts plus term plus rejection.
if grep -q -F -e 'e2e drivers (deleted' "$expected" &&
  grep -q -F -e 'negatives (green hermetic proofs' "$expected" &&
  grep -q -F -e 'no-coverage cohort (coverage-excluded runs)' "$expected" &&
  grep -q -F -e 'shell sources (no quality class by design)' "$expected" &&
  grep -q -F -e 'Term e2e throughout' "$expected" &&
  grep -q -F -e 'rejected: silent under dogfood gates' "$expected"; then
  ok
else
  bad "non_dogfed.expected lost cohort coverage (want e2e plus negatives plus no-coverage plus shell plus term plus rejection, issue #508)"
fi

# Plan harness stays versioned with explicit suites plus coverage-excluded runs plus ownership audits.
if grep -q -F -e 'Non-dogfed execution plan (issue #508' "$plan" &&
  grep -q -F -e 'CLI contract (issue #407, replaces nested E2E)' "$plan" &&
  grep -q -F -e 'negative fixtures' "$plan" &&
  grep -q -F -e 'no-coverage cohort' "$plan" &&
  grep -q -F -e 'shell sources with no quality class' "$plan"; then
  ok
else
  bad "tools/ci/non_dogfed_paths.sh lost its four-cohort plan record under issue #508"
fi

# Verification matrix owns the qualified seed-only record under.
if grep -q -F -e 'qualified seed-only under #508' "$verify" &&
  grep -q -F -e 'non_dogfed_qualification' "$verify" &&
  grep -q -F -e 'tools/ci/tests/fixtures/non_dogfed/pins.bzl' "$verify" &&
  grep -q -F -e 'bazel run //tools/ci:non_dogfed_qualification' "$verify"; then
  ok
else
  bad "verification-matrix lost its #508 qualified seed-only record with fixtures"
fi

# Testing README keeps the explicit-path record with the qualification pin.
if grep -q -F -e 'bazel run //tools/ci:non_dogfed_paths' "$testing_readme" &&
  grep -q -F -e 'bazel run //tools/ci:non_dogfed_qualification' "$testing_readme" &&
  grep -q -F -e 'hermetic pins under `bazel test //...`' "$testing_readme"; then
  ok
else
  bad "docs/testing/README.md lost its non-dogfed explicit-path plus qualification record (issue #508)"
fi

# BUILD owns the harness target plus CI wires it in dogfood-freshness.
if grep -q -F -e 'name = "non_dogfed_qualification"' "$build" &&
  grep -q -F -e 'bazel run --noshow_progress //tools/ci:non_dogfed_qualification' "$ci"; then
  ok
else
  bad "tools/ci/BUILD.bazel or ci.yml lost the non_dogfed_qualification wiring (want target plus dogfood-freshness)"
fi

# Verification matrix Green lists the harness count.
if grep -q -F -e '`non_dogfed_qualification` 16/16' "$verify"; then
  ok
else
  bad "verification-matrix Green lost non_dogfed_qualification 16/16"
fi

# Term e2e throughout: no integration-drivers prose in the new pins plus plan plus matrix record.
if grep -q -F -e 'Term e2e throughout' "$expected" &&
  grep -q -F -e 'replaces nested E2E' "$plan" &&
  ! grep -q -F -e 'integration drivers' "$plan" &&
  grep -q -F -e 'E2E' "$verify"; then
  ok
else
  bad "e2e term drifted (want e2e throughout with no integration-drivers prose, issue #508)"
fi

# Live proof: the versioned plan harness stays green on the seed host.
if bazel run --noshow_progress //tools/ci:non_dogfed_paths >/dev/null 2>&1; then
  ok
else
  bad "non_dogfed_paths live run failed (want green on the seed host, issue #508)"
fi

# Live proof: fixture corpus plus negative demos resolve green queries.
if bazel build //tools/ci/tests/fixtures/non_dogfed/... --noshow_progress >/dev/null 2>&1 &&
  bazel query '//libs/starlark/tests/negative/...' 2>/dev/null | grep -q -F -e 'failing_check_demo' &&
  bazel query 'attr(tags, manual, //quality/testdata:fixture_real_markdown_no_config_subject)' 2>/dev/null | grep -q -F -e 'fixture_real_markdown_no_config_subject'; then
  ok
else
  bad "fixture plus negative live shapes failed (want corpus build plus demos plus manual subject, issue #508)"
fi

# Live proof: no-coverage cohort plus shell ownership stay explicit (no silent gap).
if bazel query 'attr(tags, no-coverage, kind(test, //...))' 2>/dev/null | grep -q -F -e 'matrix_python_lint_fail' &&
  grep -q -F -e 'coverage --test_tag_filters=-no-coverage' tools/bazelrc/preset.bazelrc &&
  bazel query "kind('source file', deps(//tools/ci:non_dogfed_qualification))" 2>/dev/null | grep -q -F -e 'non_dogfed_qualification.sh'; then
  ok
else
  bad "no-coverage plus shell live shapes failed (want matrix fail plus preset filter plus owned shell harness, issue #508)"
fi

dx_test_summary "non-dogfed qualification harness"
